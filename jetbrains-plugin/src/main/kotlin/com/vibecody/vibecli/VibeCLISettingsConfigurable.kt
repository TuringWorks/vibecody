package com.vibecody.vibecli

import com.intellij.openapi.options.Configurable
import com.intellij.openapi.ui.ComboBox
import com.intellij.ui.ToolbarDecorator
import com.intellij.ui.components.JBLabel
import com.intellij.ui.components.JBTextField
import com.intellij.ui.table.JBTable
import com.intellij.util.ui.FormBuilder
import javax.swing.JComponent
import javax.swing.JPanel
import javax.swing.table.AbstractTableModel
import javax.swing.table.DefaultTableCellRenderer
import javax.swing.DefaultCellEditor

/**
 * Settings page shown under IDE Settings → Tools → VibeCLI.
 */
class VibeCLISettingsConfigurable : Configurable {

    private var urlField    = JBTextField()
    private var providerBox = ComboBox(arrayOf("ollama", "claude", "openai", "gemini", "grok"))
    private var modelField  = JBTextField()
    private var approvalBox = ComboBox(arrayOf("suggest", "auto-edit", "full-auto"))
    private var panel: JPanel? = null

    // Hook configuration table. Backed by a local copy of the hook
    // list so Cancel doesn't persist edits.
    private val hooksModel = HookTableModel()
    private val hooksTable = JBTable(hooksModel)

    override fun getDisplayName() = "VibeCLI"

    override fun createComponent(): JComponent {
        // Make the event column a combo so users can only pick allowed kinds.
        val eventEditor = DefaultCellEditor(ComboBox(HookExecutor.ALLOWED_EVENTS.toTypedArray()))
        hooksTable.columnModel.getColumn(1).cellEditor = eventEditor
        hooksTable.columnModel.getColumn(1).cellRenderer = DefaultTableCellRenderer()

        val hooksPanel = ToolbarDecorator.createDecorator(hooksTable)
            .setAddAction { hooksModel.addRow(HookConfig(name = "new-hook")) }
            .setRemoveAction {
                val row = hooksTable.selectedRow
                if (row >= 0) hooksModel.removeRow(row)
            }
            .createPanel()

        val p = FormBuilder.createFormBuilder()
            .addLabeledComponent(JBLabel("Daemon URL:"), urlField, 1, false)
            .addLabeledComponent(JBLabel("Provider:"), providerBox, 1, false)
            .addLabeledComponent(JBLabel("Model:"), modelField, 1, false)
            .addLabeledComponent(JBLabel("Approval mode:"), approvalBox, 1, false)
            .addLabeledComponent(JBLabel("Hooks (event → shell command):"), hooksPanel, 1, true)
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
            || hooksModel.snapshot() != s.hooks
    }

    override fun apply() {
        val s = VibeCLISettings.getInstance().state
        s.daemonUrl    = urlField.text.trim().trimEnd('/')
        s.provider     = providerBox.selectedItem as String
        s.model        = modelField.text.trim()
        s.approvalMode = approvalBox.selectedItem as String
        s.hooks        = hooksModel.snapshot().toMutableList()
    }

    override fun reset() {
        val s = VibeCLISettings.getInstance().state
        urlField.text = s.daemonUrl
        providerBox.selectedItem = s.provider
        modelField.text = s.model
        approvalBox.selectedItem = s.approvalMode
        // Deep-copy each row so live editing doesn't mutate persisted state.
        hooksModel.replaceAll(s.hooks.map { it.copy() })
    }

    // ── Hooks table model ────────────────────────────────────────────────────

    private class HookTableModel : AbstractTableModel() {
        private val rows: MutableList<HookConfig> = mutableListOf()
        private val columns = arrayOf("Name", "Event", "Command", "Enabled")

        fun snapshot(): List<HookConfig> = rows.map { it.copy() }

        fun replaceAll(items: List<HookConfig>) {
            rows.clear()
            rows.addAll(items)
            fireTableDataChanged()
        }

        fun addRow(item: HookConfig) {
            rows.add(item)
            fireTableRowsInserted(rows.size - 1, rows.size - 1)
        }

        fun removeRow(index: Int) {
            rows.removeAt(index)
            fireTableRowsDeleted(index, index)
        }

        override fun getRowCount(): Int = rows.size
        override fun getColumnCount(): Int = columns.size
        override fun getColumnName(column: Int): String = columns[column]

        override fun getColumnClass(column: Int): Class<*> = when (column) {
            3 -> java.lang.Boolean::class.java
            else -> String::class.java
        }

        override fun isCellEditable(rowIndex: Int, columnIndex: Int): Boolean = true

        override fun getValueAt(rowIndex: Int, columnIndex: Int): Any {
            val r = rows[rowIndex]
            return when (columnIndex) {
                0 -> r.name
                1 -> r.event
                2 -> r.command
                3 -> r.enabled
                else -> ""
            }
        }

        override fun setValueAt(value: Any?, rowIndex: Int, columnIndex: Int) {
            val r = rows[rowIndex]
            when (columnIndex) {
                0 -> r.name = (value as? String).orEmpty()
                1 -> {
                    val s = (value as? String).orEmpty()
                    // Snap silently to the allow-list; the combo
                    // editor should already constrain this, but a
                    // future copy-paste could bypass.
                    r.event = if (HookExecutor.ALLOWED_EVENTS.contains(s)) s else r.event
                }
                2 -> r.command = (value as? String).orEmpty()
                3 -> r.enabled = (value as? Boolean) ?: r.enabled
            }
            fireTableCellUpdated(rowIndex, columnIndex)
        }
    }
}
