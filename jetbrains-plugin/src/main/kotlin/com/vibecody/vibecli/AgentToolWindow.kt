package com.vibecody.vibecli

import com.intellij.openapi.project.Project
import com.intellij.openapi.wm.ToolWindow
import com.intellij.openapi.wm.ToolWindowFactory
import com.intellij.ui.components.JBScrollPane
import com.intellij.ui.components.JBTabbedPane
import com.intellij.ui.content.ContentFactory
import java.awt.*
import java.awt.event.ActionEvent
import java.awt.event.KeyAdapter
import java.awt.event.KeyEvent
import javax.swing.*
import javax.swing.text.DefaultCaret

/**
 * Tool-window factory — registered in plugin.xml as `factoryClass`.
 *
 * Creates two tabs:
 * 1. **Chat** — stateless single-turn conversation
 * 2. **Agent** — multi-step agent task with live SSE streaming
 */
class AgentToolWindowFactory : ToolWindowFactory {
    override fun createToolWindowContent(project: Project, toolWindow: ToolWindow) {
        val tabs = JBTabbedPane()
        tabs.addTab("💬 Chat", ChatPanel(project))
        tabs.addTab("🤖 Agent", AgentPanel(project))
        tabs.addTab("📋 Jobs", JobsPanel(project))

        val content = ContentFactory.getInstance().createContent(tabs, "", false)
        toolWindow.contentManager.addContent(content)
    }
}

// ── Shared helpers ─────────────────────────────────────────────────────────────

private val PANEL_BG = Color(30, 30, 30)
private val INPUT_BG = Color(45, 45, 45)
private val ACCENT   = Color(100, 180, 255)
private val TEXT_FG  = Color(204, 204, 204)
private val DIM_FG   = Color(128, 128, 128)
private val SUCCESS  = Color(100, 220, 100)
private val ERROR    = Color(255, 100, 100)

private fun basePanel(): JPanel = JPanel(BorderLayout()).apply {
    background = PANEL_BG
    border = BorderFactory.createEmptyBorder(8, 8, 8, 8)
}

private fun styledTextArea(): JTextArea = JTextArea().apply {
    isEditable = false
    lineWrap = true
    wrapStyleWord = true
    background = INPUT_BG
    foreground = TEXT_FG
    font = Font(Font.MONOSPACED, Font.PLAIN, 12)
    border = BorderFactory.createEmptyBorder(6, 6, 6, 6)
    (caret as DefaultCaret).updatePolicy = DefaultCaret.ALWAYS_UPDATE
}

private fun styledInput(placeholder: String = ""): JTextArea = JTextArea(3, 40).apply {
    lineWrap = true
    wrapStyleWord = true
    background = INPUT_BG
    foreground = TEXT_FG
    caretColor = ACCENT
    font = Font(Font.SANS_SERIF, Font.PLAIN, 13)
    border = BorderFactory.createEmptyBorder(6, 6, 6, 6)
    if (placeholder.isNotEmpty()) toolTipText = placeholder
}

private fun styledButton(label: String): JButton = JButton(label).apply {
    background = Color(60, 60, 80)
    foreground = ACCENT
    isFocusPainted = false
    isOpaque = true
    border = BorderFactory.createEmptyBorder(6, 14, 6, 14)
}

private fun statusLabel(text: String = "") = JLabel(text).apply {
    foreground = DIM_FG
    font = Font(Font.SANS_SERIF, Font.ITALIC, 11)
}

// ── Chat Panel ─────────────────────────────────────────────────────────────────

private class ChatPanel(private val project: Project) : JPanel(BorderLayout()) {

    private val output = styledTextArea()
    private val input  = styledInput("Ask anything… (Shift+Enter to send)")
    private val sendBtn = styledButton("Send ↵")
    private val statusLbl = statusLabel("Not connected")
    private val service = VibeCLIService.getInstance()

    init {
        background = PANEL_BG
        border = BorderFactory.createEmptyBorder(8, 8, 8, 8)

        // ── Top status bar ──────────────────────────────────────────────────
        val topBar = JPanel(BorderLayout()).apply {
            background = PANEL_BG
            border = BorderFactory.createEmptyBorder(0, 0, 6, 0)
        }
        val refreshBtn = styledButton("⟳").apply { preferredSize = Dimension(32, 24) }
        refreshBtn.addActionListener { checkHealth() }
        topBar.add(statusLbl, BorderLayout.CENTER)
        topBar.add(refreshBtn, BorderLayout.EAST)
        add(topBar, BorderLayout.NORTH)

        // ── Output ──────────────────────────────────────────────────────────
        val scroll = JBScrollPane(output).apply {
            border = BorderFactory.createLineBorder(Color(60, 60, 60))
        }
        add(scroll, BorderLayout.CENTER)

        // ── Input bar ───────────────────────────────────────────────────────
        val inputScroll = JBScrollPane(input).apply {
            preferredSize = Dimension(0, 90)
            border = BorderFactory.createLineBorder(Color(60, 60, 60))
        }
        input.addKeyListener(object : KeyAdapter() {
            override fun keyPressed(e: KeyEvent) {
                if (e.keyCode == KeyEvent.VK_ENTER && e.isShiftDown) {
                    e.consume()
                    send()
                }
            }
        })
        sendBtn.addActionListener { send() }

        val bottomBar = JPanel(BorderLayout(6, 0)).apply { background = PANEL_BG }
        bottomBar.add(inputScroll, BorderLayout.CENTER)
        bottomBar.add(sendBtn, BorderLayout.EAST)

        val south = JPanel(BorderLayout(0, 4)).apply { background = PANEL_BG }
        south.add(bottomBar, BorderLayout.CENTER)
        add(south, BorderLayout.SOUTH)

        checkHealth()
    }

    private fun checkHealth() {
        statusLbl.text = "Checking…"
        statusLbl.foreground = DIM_FG
        Thread {
            val ok = service.isHealthy()
            SwingUtilities.invokeLater {
                if (ok) {
                    statusLbl.text = "● Connected to ${VibeCLISettings.getInstance().state.daemonUrl}"
                    statusLbl.foreground = SUCCESS
                } else {
                    statusLbl.text = "○ Daemon not reachable — run: vibecli serve"
                    statusLbl.foreground = ERROR
                }
            }
        }.start()
    }

    private fun send() {
        val msg = input.text.trim()
        if (msg.isEmpty()) return
        input.text = ""
        sendBtn.isEnabled = false

        appendOutput("\nYou: $msg\n", TEXT_FG)
        appendOutput("Assistant: ", ACCENT)

        service.chat(msg).whenComplete { reply, err ->
            SwingUtilities.invokeLater {
                sendBtn.isEnabled = true
                if (err != null) {
                    appendOutput("Error: ${err.message}\n", ERROR)
                } else {
                    appendOutput("$reply\n\n", TEXT_FG)
                }
            }
        }
    }

    private fun appendOutput(text: String, color: Color) {
        // Simple appending; a more advanced version would use StyledDocument
        output.append(text)
        output.caretPosition = output.document.length
    }
}

// ── Agent Panel ────────────────────────────────────────────────────────────────

private class AgentPanel(private val project: Project) : JPanel(BorderLayout()) {

    private val output   = styledTextArea()
    private val input    = styledInput("Describe the task… (Shift+Enter to start)")
    private val runBtn   = styledButton("▶ Run")
    private val stopBtn  = styledButton("■ Stop").apply { isEnabled = false }
    private val statusLbl = statusLabel("Idle")
    private val service  = VibeCLIService.getInstance()
    private var cancelFlag = java.util.concurrent.atomic.AtomicBoolean(false)

    init {
        background = PANEL_BG
        border = BorderFactory.createEmptyBorder(8, 8, 8, 8)

        // Status
        val topBar = JPanel(FlowLayout(FlowLayout.LEFT, 4, 0)).apply { background = PANEL_BG }
        topBar.add(statusLbl)
        add(topBar, BorderLayout.NORTH)

        // Output
        val scroll = JBScrollPane(output).apply {
            border = BorderFactory.createLineBorder(Color(60, 60, 60))
        }
        add(scroll, BorderLayout.CENTER)

        // Input + buttons
        input.addKeyListener(object : KeyAdapter() {
            override fun keyPressed(e: KeyEvent) {
                if (e.keyCode == KeyEvent.VK_ENTER && e.isShiftDown) {
                    e.consume()
                    startAgent()
                }
            }
        })
        runBtn.addActionListener { startAgent() }
        stopBtn.addActionListener { cancelFlag.set(false) }

        val inputScroll = JBScrollPane(input).apply {
            preferredSize = Dimension(0, 90)
            border = BorderFactory.createLineBorder(Color(60, 60, 60))
        }
        val btnPanel = JPanel(GridLayout(2, 1, 0, 4)).apply {
            background = PANEL_BG
            add(runBtn)
            add(stopBtn)
        }
        val bottomBar = JPanel(BorderLayout(6, 0)).apply { background = PANEL_BG }
        bottomBar.add(inputScroll, BorderLayout.CENTER)
        bottomBar.add(btnPanel, BorderLayout.EAST)
        add(bottomBar, BorderLayout.SOUTH)
    }

    private fun startAgent() {
        val task = input.text.trim()
        if (task.isEmpty()) return
        input.text = ""
        runBtn.isEnabled = false
        stopBtn.isEnabled = true
        setStatus("Starting…", ACCENT)
        output.text = ""
        output.append("Task: $task\n${"─".repeat(60)}\n\n")

        service.startAgent(task).whenComplete { sessionId, err ->
            SwingUtilities.invokeLater {
                if (err != null) {
                    setStatus("Error: ${err.message}", ERROR)
                    runBtn.isEnabled = true
                    stopBtn.isEnabled = false
                    return@invokeLater
                }
                setStatus("Running… (session $sessionId)", ACCENT)
                cancelFlag = service.streamSession(
                    sessionId,
                    onEvent = { event ->
                        SwingUtilities.invokeLater { renderEvent(event) }
                    },
                    onDone = {
                        SwingUtilities.invokeLater {
                            runBtn.isEnabled = true
                            stopBtn.isEnabled = false
                            if (statusLbl.text.startsWith("Running")) {
                                setStatus("Complete ✓", SUCCESS)
                            }
                        }
                    }
                )
            }
        }
    }

    private fun renderEvent(event: AgentEvent) {
        when (event) {
            is AgentEvent.Thinking    -> output.append("[Thinking] ${event.text}\n")
            is AgentEvent.Text        -> output.append(event.text)
            is AgentEvent.ToolCall    -> output.append("\n🔧 ${event.name}(${event.input.take(120)})\n")
            is AgentEvent.ToolResult  -> output.append("   → ${event.output.take(240)}\n")
            is AgentEvent.Complete    -> {
                output.append("\n\n${"─".repeat(60)}\n✅ ${event.summary}\n")
                setStatus("Complete ✓", SUCCESS)
            }
            is AgentEvent.Error       -> {
                output.append("\n❌ ${event.message}\n")
                setStatus("Failed", ERROR)
            }
        }
        output.caretPosition = output.document.length
    }

    private fun setStatus(text: String, color: Color) {
        statusLbl.text = text
        statusLbl.foreground = color
    }
}

// ── Jobs Panel ─────────────────────────────────────────────────────────────────

private class JobsPanel(private val project: Project) : JPanel(BorderLayout()) {

    private val listModel = DefaultListModel<String>()
    private val jobList   = JList(listModel).apply {
        background = INPUT_BG
        foreground = TEXT_FG
        font = Font(Font.MONOSPACED, Font.PLAIN, 12)
        selectionBackground = Color(60, 80, 120)
    }
    private val statusLbl = statusLabel("Loading…")
    private val service   = VibeCLIService.getInstance()
    private val allJobs   = mutableListOf<JobRecord>()

    init {
        background = PANEL_BG
        border = BorderFactory.createEmptyBorder(8, 8, 8, 8)

        val top = JPanel(BorderLayout()).apply { background = PANEL_BG }
        top.add(statusLbl, BorderLayout.CENTER)
        val refreshBtn = styledButton("⟳ Refresh")
        refreshBtn.addActionListener { load() }
        top.add(refreshBtn, BorderLayout.EAST)
        add(top, BorderLayout.NORTH)

        add(JBScrollPane(jobList).apply {
            border = BorderFactory.createLineBorder(Color(60, 60, 60))
        }, BorderLayout.CENTER)

        // Detail area below list
        val detail = styledTextArea().apply {
            preferredSize = Dimension(0, 100)
        }
        add(JBScrollPane(detail).apply {
            border = BorderFactory.createLineBorder(Color(60, 60, 60))
            preferredSize = Dimension(0, 120)
        }, BorderLayout.SOUTH)

        jobList.addListSelectionListener {
            val idx = jobList.selectedIndex
            if (idx >= 0 && idx < allJobs.size) {
                val job = allJobs[idx]
                detail.text = buildString {
                    appendLine("Session: ${job.sessionId}")
                    appendLine("Status:  ${job.status}")
                    appendLine("Provider: ${job.provider}")
                    appendLine("Task: ${job.task}")
                    if (job.summary != null) {
                        appendLine("\nSummary:\n${job.summary}")
                    }
                }
            }
        }

        load()
    }

    private fun load() {
        statusLbl.text = "Loading…"
        statusLbl.foreground = DIM_FG
        service.listJobs().whenComplete { jobs, err ->
            SwingUtilities.invokeLater {
                if (err != null) {
                    statusLbl.text = "Could not load — daemon running?"
                    statusLbl.foreground = ERROR
                    return@invokeLater
                }
                allJobs.clear()
                allJobs.addAll(jobs)
                listModel.clear()
                jobs.forEach { job ->
                    val icon = when (job.status) {
                        "complete" -> "✅"
                        "failed"   -> "❌"
                        "cancelled"-> "⛔"
                        else       -> "🟡"
                    }
                    val preview = job.task.take(60)
                    listModel.addElement("$icon [${ job.status.uppercase()}]  $preview")
                }
                statusLbl.text = "${jobs.size} job(s)"
                statusLbl.foreground = DIM_FG
            }
        }
    }
}
