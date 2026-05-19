package com.vibecody.vibecli

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Unit tests for [HookExecutor]. Uses the `fireChain(list, event,
 * payload)` overload so we don't depend on the IntelliJ Platform
 * service container — these run as plain JUnit on a vanilla JVM.
 *
 * Shell-out tests use `sh -c` directly (the same shape HookExecutor
 * uses), so they require a POSIX-like environment. CI on macOS /
 * Linux is fine; Windows runs would need a guard or a PowerShell
 * variant — same constraint as the production code path.
 */
class HookExecutorTest {

    private val executor = HookExecutor()

    @Test
    fun no_hooks_configured_is_allow() {
        val d = executor.fireChain(emptyList(), "PreToolUse", "{}")
        assertEquals(HookExecutor.HookAction.ALLOW, d.action)
    }

    @Test
    fun disabled_hook_is_skipped() {
        val hooks = listOf(
            HookConfig(name = "off", event = "PreToolUse", command = "exit 2", enabled = false),
        )
        val d = executor.fireChain(hooks, "PreToolUse", "{}")
        assertEquals(HookExecutor.HookAction.ALLOW, d.action)
    }

    @Test
    fun event_filter_only_runs_matching_hooks() {
        val hooks = listOf(
            HookConfig(name = "wrong", event = "PostToolUse", command = "exit 2"),
        )
        val d = executor.fireChain(hooks, "PreToolUse", "{}")
        assertEquals(HookExecutor.HookAction.ALLOW, d.action)
    }

    @Test
    fun exit_zero_is_allow() {
        val hooks = listOf(HookConfig(name = "ok", event = "PreToolUse", command = "exit 0"))
        val d = executor.fireChain(hooks, "PreToolUse", "{}")
        assertEquals(HookExecutor.HookAction.ALLOW, d.action)
        assertEquals(0, d.exit_code)
    }

    @Test
    fun exit_two_is_block_with_stderr_reason() {
        val hooks = listOf(
            HookConfig(name = "no", event = "PreToolUse", command = "echo 'nope' 1>&2; exit 2"),
        )
        val d = executor.fireChain(hooks, "PreToolUse", "{}")
        assertEquals(HookExecutor.HookAction.BLOCK, d.action)
        assertEquals(2, d.exit_code)
        assertNotNull(d.reason)
        assertTrue("reason should carry stderr: ${d.reason}", d.reason!!.contains("nope"))
    }

    @Test
    fun exit_three_is_generic_error_non_blocking() {
        // CLI semantics: any non-0 non-2 exit code is a "generic error"
        // that warns but doesn't block. Mirrors hook_abort.rs.
        val hooks = listOf(
            HookConfig(name = "warn", event = "PreToolUse", command = "exit 3"),
        )
        val d = executor.fireChain(hooks, "PreToolUse", "{}")
        assertEquals(HookExecutor.HookAction.ALLOW, d.action)
        assertEquals(3, d.exit_code)
    }

    @Test
    fun structured_json_decision_overrides_exit_code() {
        // Exit 0 normally means ALLOW, but a stdout JSON decision of
        // BLOCK must win. Verifies the override-priority direction.
        val cmd = """printf '{"action":"block","reason":"by policy"}'; exit 0"""
        val hooks = listOf(HookConfig(name = "j", event = "PreToolUse", command = cmd))
        val d = executor.fireChain(hooks, "PreToolUse", "{}")
        assertEquals(HookExecutor.HookAction.BLOCK, d.action)
        assertEquals("by policy", d.reason)
    }

    @Test
    fun structured_json_allow_passes() {
        val cmd = """printf '{"action":"allow"}'; exit 0"""
        val hooks = listOf(HookConfig(name = "j", event = "PreToolUse", command = cmd))
        val d = executor.fireChain(hooks, "PreToolUse", "{}")
        assertEquals(HookExecutor.HookAction.ALLOW, d.action)
    }

    @Test
    fun chain_short_circuits_on_first_block() {
        // Two hooks. The first blocks; the second writes a sentinel
        // file. After fireChain, the file must NOT exist.
        val sentinel = java.io.File.createTempFile("hook-second-ran-", ".tmp").apply { delete() }
        val hooks = listOf(
            HookConfig(
                name = "first",
                event = "PreToolUse",
                command = "echo first 1>&2; exit 2",
            ),
            HookConfig(
                name = "second",
                event = "PreToolUse",
                command = "touch ${sentinel.absolutePath}",
            ),
        )
        val d = executor.fireChain(hooks, "PreToolUse", "{}")
        assertEquals(HookExecutor.HookAction.BLOCK, d.action)
        assertTrue(
            "second hook must not have run after the first blocked",
            !sentinel.exists(),
        )
    }

    @Test
    fun chain_continues_when_first_allows() {
        val sentinel = java.io.File.createTempFile("hook-chain-second-", ".tmp").apply { delete() }
        val hooks = listOf(
            HookConfig(name = "first", event = "PreToolUse", command = "exit 0"),
            HookConfig(
                name = "second",
                event = "PreToolUse",
                command = "touch ${sentinel.absolutePath}",
            ),
        )
        executor.fireChain(hooks, "PreToolUse", "{}")
        assertTrue(
            "second hook should have run when first allowed",
            sentinel.exists(),
        )
        sentinel.delete()
    }

    @Test
    fun empty_command_is_no_op_allow() {
        val hooks = listOf(HookConfig(name = "blank", event = "PreToolUse", command = "   "))
        val d = executor.fireChain(hooks, "PreToolUse", "{}")
        assertEquals(HookExecutor.HookAction.ALLOW, d.action)
    }

    @Test
    fun payload_is_piped_to_hook_stdin() {
        // Capture stdin into a file via `cat`. Verify the contents
        // came through unchanged.
        val captured = java.io.File.createTempFile("hook-stdin-", ".tmp")
        captured.delete()
        val cmd = "cat > ${captured.absolutePath}"
        val hooks = listOf(HookConfig(name = "echo", event = "PreToolUse", command = cmd))
        val payload = """{"event":"PreToolUse","tool":"read_file"}"""
        executor.fireChain(hooks, "PreToolUse", payload)
        assertEquals(payload, captured.readText())
        captured.delete()
    }

    @Test
    fun allowed_events_constant_matches_cli_set() {
        // Sanity check: catches the case where someone adds a new event
        // kind to plugin_manifest::ALLOWED_HOOK_EVENTS but forgets the
        // matching ALLOWED_EVENTS entry here. The two lists must stay
        // in sync; an updated CLI side with a missing JB entry would
        // silently let those hooks fall off the table.
        val expected = setOf(
            "PreToolUse",
            "PostToolUse",
            "UserPromptSubmit",
            "Stop",
            "SubagentStop",
            "Notification",
            "PreCompact",
        )
        assertEquals(expected, HookExecutor.ALLOWED_EVENTS.toSet())
    }

    @Test
    fun unstructured_stdout_does_not_trip_structured_parser() {
        // Stdout that happens to start with `{` but isn't a valid
        // structured decision should fall through to exit-code
        // semantics, not throw.
        val cmd = """printf '{not json'; exit 0"""
        val hooks = listOf(HookConfig(name = "garbage", event = "PreToolUse", command = cmd))
        val d = executor.fireChain(hooks, "PreToolUse", "{}")
        assertEquals(HookExecutor.HookAction.ALLOW, d.action)
        assertEquals(0, d.exit_code)
        assertNull("no structured reason when JSON malformed", d.reason)
    }
}
