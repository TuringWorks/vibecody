package com.vibecody.vibecli

import com.intellij.openapi.components.Service
import com.intellij.openapi.diagnostic.thisLogger
import java.io.IOException
import java.util.concurrent.TimeUnit

/**
 * Hook execution service mirroring the CLI/Tauri hook protocol
 * (`vibecli-cli/src/hook_abort.rs`).
 *
 * Events are fired into a chain of configured subprocess hooks. Each
 * hook receives the event payload as JSON on stdin, runs its command,
 * and signals a decision via exit code:
 *
 * | Exit code | Decision                                              |
 * |:---------:|------------------------------------------------------|
 * |   `0`     | Allow — continue the action.                          |
 * |   `2`     | Block — halt the action; surface stderr to the agent. |
 * | other     | Generic error — non-blocking by default; warn user.   |
 *
 * Hooks may also emit a JSON object on stdout with shape
 * `{ "action": "allow"|"block"|"modify", "reason": "...", "message": "..." }`.
 * When present, that structured decision wins over the exit code.
 *
 * Parity scope:
 *   - PreToolUse, PostToolUse, UserPromptSubmit, Stop, SubagentStop,
 *     Notification, PreCompact events.
 *   - 30 s per-hook timeout (matches the CLI default).
 *   - Multiple hooks per event are run in declaration order; the
 *     first `block` short-circuits the rest.
 */
@Service(Service.Level.APP)
class HookExecutor {

    enum class HookAction { ALLOW, BLOCK, MODIFY }

    data class HookDecision(
        val action: HookAction,
        val reason: String? = null,
        val message: String? = null,
        val raw_stderr: String = "",
        val raw_stdout: String = "",
        val exit_code: Int = 0,
    )

    /**
     * Fire `event` through every configured hook of that kind, in
     * order. Returns the first non-ALLOW decision, or
     * `HookDecision(ALLOW)` when every hook permitted the action.
     */
    fun fire(event: String, payloadJson: String): HookDecision =
        fireChain(VibeCLISettings.getInstance().state.hooks, event, payloadJson)

    /**
     * Same as [fire] but takes an explicit hook list instead of
     * reading from [VibeCLISettings]. Exposed for unit testing — the
     * production path goes through [fire] and the settings service.
     */
    internal fun fireChain(
        all: List<HookConfig>,
        event: String,
        payloadJson: String,
    ): HookDecision {
        val configured = all.filter { it.enabled && it.event == event }
        if (configured.isEmpty()) return HookDecision(HookAction.ALLOW)

        for (hook in configured) {
            val decision = runOne(hook, payloadJson)
            if (decision.action != HookAction.ALLOW) {
                return decision
            }
        }
        return HookDecision(HookAction.ALLOW)
    }

    private fun runOne(hook: HookConfig, payloadJson: String): HookDecision {
        val command = hook.command.trim()
        if (command.isEmpty()) return HookDecision(HookAction.ALLOW)

        // Use the user's shell so command strings can carry pipes
        // and quoting the same way they would in `.claude/settings.json`.
        val argv = listOf("sh", "-c", command)

        return try {
            val proc = ProcessBuilder(argv)
                .redirectErrorStream(false)
                .start()
            proc.outputStream.use { it.write(payloadJson.toByteArray()) }
            val finished = proc.waitFor(HOOK_TIMEOUT_SECONDS, TimeUnit.SECONDS)
            if (!finished) {
                proc.destroyForcibly()
                return HookDecision(
                    action = HookAction.BLOCK,
                    reason = "hook timeout after ${HOOK_TIMEOUT_SECONDS}s",
                    raw_stderr = "",
                    raw_stdout = "",
                    exit_code = -1,
                )
            }
            val stdout = proc.inputStream.bufferedReader().readText()
            val stderr = proc.errorStream.bufferedReader().readText()
            val exit = proc.exitValue()

            // Try the structured-decision path first.
            parseStructuredDecision(stdout, exit, stderr, stdout)?.let { return it }

            // Fall back to exit-code semantics.
            val action = when (exit) {
                0 -> HookAction.ALLOW
                2 -> HookAction.BLOCK
                else -> HookAction.ALLOW // matches CLI: non-2 non-zero = generic error, non-blocking
            }
            HookDecision(
                action = action,
                reason = if (action == HookAction.BLOCK) stderr.trim().ifEmpty { "blocked by hook ${hook.name}" } else null,
                message = stderr.trim().takeIf { it.isNotEmpty() },
                raw_stderr = stderr,
                raw_stdout = stdout,
                exit_code = exit,
            )
        } catch (e: IOException) {
            thisLogger().warn("hook `${hook.name}` failed to spawn: ${e.message}")
            // Spawn failure is non-blocking — matches CLI semantics where
            // a missing hook command is a warning, not a hard error.
            HookDecision(
                action = HookAction.ALLOW,
                message = "hook ${hook.name} failed to spawn: ${e.message}",
                exit_code = -1,
            )
        } catch (e: InterruptedException) {
            Thread.currentThread().interrupt()
            HookDecision(
                action = HookAction.ALLOW,
                message = "hook ${hook.name} interrupted: ${e.message}",
                exit_code = -1,
            )
        }
    }

    /** Try to interpret stdout as a JSON decision object. Returns null
     *  when stdout is empty, isn't JSON, or doesn't have an `action`
     *  field — caller falls back to exit-code semantics. */
    private fun parseStructuredDecision(
        stdout: String,
        exit: Int,
        stderr: String,
        rawStdout: String,
    ): HookDecision? {
        val trimmed = stdout.trim()
        if (!trimmed.startsWith("{") || !trimmed.endsWith("}")) return null
        return try {
            val gson = com.google.gson.Gson()
            val obj = gson.fromJson(trimmed, com.google.gson.JsonObject::class.java) ?: return null
            val actionStr = obj.get("action")?.asString?.lowercase() ?: return null
            val action = when (actionStr) {
                "allow" -> HookAction.ALLOW
                "block" -> HookAction.BLOCK
                "modify" -> HookAction.MODIFY
                else -> return null
            }
            HookDecision(
                action = action,
                reason = obj.get("reason")?.asString,
                message = obj.get("message")?.asString,
                raw_stderr = stderr,
                raw_stdout = rawStdout,
                exit_code = exit,
            )
        } catch (e: Exception) {
            null
        }
    }

    companion object {
        const val HOOK_TIMEOUT_SECONDS: Long = 30

        /** Allow-list of event kinds the executor will fire. Mirrors
         *  `plugin_manifest::ALLOWED_HOOK_EVENTS` on the CLI side. */
        val ALLOWED_EVENTS: List<String> = listOf(
            "PreToolUse",
            "PostToolUse",
            "UserPromptSubmit",
            "Stop",
            "SubagentStop",
            "Notification",
            "PreCompact",
        )

        fun getInstance(): HookExecutor =
            com.intellij.openapi.application.ApplicationManager
                .getApplication()
                .getService(HookExecutor::class.java)
    }
}

/**
 * Persisted hook configuration. Stored as a list inside
 * `VibeCLISettings.State.hooks`. `event` must be one of
 * `HookExecutor.ALLOWED_EVENTS`; the settings UI enforces this via
 * a combo box.
 */
data class HookConfig(
    var name: String = "",
    var event: String = "PreToolUse",
    var command: String = "",
    var enabled: Boolean = true,
)
