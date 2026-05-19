package com.vibecody.vibecli

import com.intellij.openapi.components.PersistentStateComponent
import com.intellij.openapi.components.Service
import com.intellij.openapi.components.State
import com.intellij.openapi.components.Storage

/**
 * Persistent plugin settings stored in `vibecli.xml`.
 *
 * Access via [VibeCLISettings.getInstance].
 */
@Service(Service.Level.APP)
@State(name = "VibeCLISettings", storages = [Storage("vibecli.xml")])
class VibeCLISettings : PersistentStateComponent<VibeCLISettings.State> {

    data class State(
        var daemonUrl: String = "http://localhost:7878",
        var provider: String = "ollama",
        var model: String = "qwen2.5-coder:7b",
        var approvalMode: String = "suggest",   // suggest | auto-edit | full-auto
        var streamingEnabled: Boolean = true,
        /**
         * Subprocess hooks fired by [HookExecutor] on agent lifecycle
         * events. Parity with `.claude/settings.json` hook protocol
         * (`vibecli-cli/src/hook_abort.rs`). XML-serialized as a list
         * of `HookConfig` rows; the Settings UI is the only authoring
         * surface today.
         */
        var hooks: MutableList<HookConfig> = mutableListOf(),
    )

    private var myState = State()

    override fun getState(): State = myState

    override fun loadState(state: State) {
        myState = state
    }

    companion object {
        fun getInstance(): VibeCLISettings =
            com.intellij.openapi.application.ApplicationManager
                .getApplication()
                .getService(VibeCLISettings::class.java)
    }
}
