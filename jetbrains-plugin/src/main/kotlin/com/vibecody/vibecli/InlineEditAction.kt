package com.vibecody.vibecli

import com.intellij.openapi.actionSystem.AnAction
import com.intellij.openapi.actionSystem.AnActionEvent
import com.intellij.openapi.actionSystem.CommonDataKeys
import com.intellij.openapi.application.ApplicationManager
import com.intellij.openapi.command.WriteCommandAction
import com.intellij.openapi.editor.Editor
import com.intellij.openapi.progress.ProgressIndicator
import com.intellij.openapi.progress.ProgressManager
import com.intellij.openapi.progress.Task
import com.intellij.openapi.project.Project
import com.intellij.openapi.ui.Messages
import com.intellij.openapi.wm.ToolWindowManager

/**
 * Ctrl+Shift+K — apply an AI edit to the selected text.
 *
 * 1. Prompts user for an edit instruction (dialog).
 * 2. Sends `[ORIGINAL CODE]\n---\n[INSTRUCTION]` to the daemon `/chat` endpoint.
 * 3. Replaces the selection with the returned code in a write-command action
 *    so it is undoable via Ctrl+Z.
 *
 * If no text is selected the entire file content is used as context, and
 * the reply is appended at the cursor position instead of replacing.
 */
class InlineEditAction : AnAction() {

    override fun actionPerformed(e: AnActionEvent) {
        val project = e.project ?: return
        val editor  = e.getData(CommonDataKeys.EDITOR) ?: return
        val service = VibeCLIService.getInstance()

        // ── Prompt for instruction ─────────────────────────────────────────
        val instruction = Messages.showInputDialog(
            project,
            "Describe the edit:",
            "VibeCLI: Inline Edit",
            null,
        )?.trim()
        if (instruction.isNullOrEmpty()) return

        // ── Build prompt ───────────────────────────────────────────────────
        val selectionModel = editor.selectionModel
        val hasSelection   = selectionModel.hasSelection()
        val selectedText   = if (hasSelection) selectionModel.selectedText ?: "" else ""

        val prompt = buildString {
            if (hasSelection && selectedText.isNotEmpty()) {
                appendLine("You are a code editor. Apply the requested edit to the code below.")
                appendLine("Respond with ONLY the replacement code — no explanations, no markdown fences.")
                appendLine()
                appendLine("### Code")
                appendLine(selectedText)
                appendLine()
                appendLine("### Instruction")
                append(instruction)
            } else {
                // No selection — treat as a chat request using file context
                val fileText = editor.document.text.take(8000)
                appendLine("File contents (truncated to 8000 chars):")
                appendLine(fileText)
                appendLine()
                appendLine("User instruction: $instruction")
                appendLine()
                append("Respond with a short explanation and, if applicable, the replacement code block.")
            }
        }

        // ── Run with progress indicator ────────────────────────────────────
        ProgressManager.getInstance().run(object : Task.Backgroundable(
            project, "VibeCLI: Applying AI edit…", true
        ) {
            private var result: String? = null
            private var error: String? = null

            override fun run(indicator: ProgressIndicator) {
                indicator.isIndeterminate = true
                try {
                    result = service.chat(prompt).get()
                } catch (ex: Exception) {
                    error = ex.message ?: "Unknown error"
                }
            }

            override fun onFinished() {
                val err = error
                val text = result

                if (err != null) {
                    ApplicationManager.getApplication().invokeLater {
                        Messages.showErrorDialog(project, "VibeCLI error: $err", "Inline Edit Failed")
                    }
                    return
                }
                if (text.isNullOrBlank()) return

                ApplicationManager.getApplication().invokeLater {
                    applyEdit(project, editor, text.trim(), hasSelection, selectionModel)
                }
            }
        })
    }

    private fun applyEdit(
        project: Project,
        editor: Editor,
        replacement: String,
        hasSelection: Boolean,
        selectionModel: com.intellij.openapi.editor.SelectionModel,
    ) {
        WriteCommandAction.runWriteCommandAction(project, "VibeCLI Inline Edit", null, {
            val doc = editor.document
            if (hasSelection) {
                val start = selectionModel.selectionStart
                val end   = selectionModel.selectionEnd
                doc.replaceString(start, end, replacement)
                // Reposition caret after inserted text
                editor.caretModel.moveToOffset(start + replacement.length)
                selectionModel.removeSelection()
            } else {
                // Append at cursor
                val offset = editor.caretModel.offset
                doc.insertString(offset, "\n$replacement")
                editor.caretModel.moveToOffset(offset + replacement.length + 1)
            }
        })
    }

    override fun update(e: AnActionEvent) {
        // Action is available whenever an editor is open
        e.presentation.isEnabled = e.getData(CommonDataKeys.EDITOR) != null
    }
}

// ── Open-window action ─────────────────────────────────────────────────────────

/**
 * Ctrl+Shift+A — reveal the VibeCLI tool window.
 */
class OpenWindowAction : AnAction() {
    override fun actionPerformed(e: AnActionEvent) {
        val project = e.project ?: return
        ToolWindowManager.getInstance(project).getToolWindow("VibeCLI")?.show()
    }
}
