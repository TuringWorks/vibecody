package com.vibecody.vibecli

import com.intellij.openapi.options.Configurable
import com.intellij.openapi.ui.ComboBox
import com.intellij.ui.components.JBLabel
import com.intellij.ui.components.JBTextField
import com.intellij.util.ui.FormBuilder
import javax.swing.JComponent
import javax.swing.JPanel

/**
 * Settings page shown under IDE Settings → Tools → VibeCLI.
 */
class VibeCLISettingsConfigurable : Configurable {

    private var urlField    = JBTextField()
    private var providerBox = ComboBox(arrayOf("ollama", "claude", "openai", "gemini", "grok"))
    private var modelField  = JBTextField()
    private var approvalBox = ComboBox(arrayOf("suggest", "auto-edit", "full-auto"))
    private var panel: JPanel? = null

    override fun getDisplayName() = "VibeCLI"

    override fun createComponent(): JComponent {
        val p = FormBuilder.createFormBuilder()
            .addLabeledComponent(JBLabel("Daemon URL:"), urlField, 1, false)
            .addLabeledComponent(JBLabel("Provider:"), providerBox, 1, false)
            .addLabeledComponent(JBLabel("Model:"), modelField, 1, false)
            .addLabeledComponent(JBLabel("Approval mode:"), approvalBox, 1, false)
            .addComponentFillVertically(JPanel(), 0)
            .panel
        panel = p
        reset()
        return p
    }

    override fun isModified(): Boolean {
        val s = VibeCLISettings.getInstance().state
        return urlField.text != s.daemonUrl
            || providerBox.selectedItem != s.provider
            || modelField.text != s.model
            || approvalBox.selectedItem != s.approvalMode
    }

    override fun apply() {
        val s = VibeCLISettings.getInstance().state
        s.daemonUrl    = urlField.text.trim().trimEnd('/')
        s.provider     = providerBox.selectedItem as String
        s.model        = modelField.text.trim()
        s.approvalMode = approvalBox.selectedItem as String
    }

    override fun reset() {
        val s = VibeCLISettings.getInstance().state
        urlField.text = s.daemonUrl
        providerBox.selectedItem = s.provider
        modelField.text = s.model
        approvalBox.selectedItem = s.approvalMode
    }
}
