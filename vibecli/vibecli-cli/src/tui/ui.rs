use ratatui::{
    layout::{Constraint, Direction, Layout, Rect, Alignment},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::tui::app::{App, CurrentScreen};
use crate::tui::components::agent_view::AgentStatus;
use vibe_ai::agent::AgentStep;

pub fn draw(f: &mut Frame, app: &App) {
    // Diagnostics pane is 4 lines tall; hide it when there are no items to save space.
    let diag_height = diag_panel_height(app.diagnostics_panel.items.len());

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(diag_height),
            Constraint::Length(3),
        ].as_ref())
        .split(f.area());

    match app.current_screen {
        CurrentScreen::Chat => draw_main_area(f, app, chunks[0]),
        CurrentScreen::DiffView => draw_diff_view(f, app, chunks[0]),
        CurrentScreen::FileTree => draw_file_tree(f, app, chunks[0]),
        CurrentScreen::Agent => draw_agent_view(f, app, chunks[0]),
        CurrentScreen::VimEditor => {
            // Vim editor renders itself into the full available area (no input strip)
            app.vim_editor.render(f, chunks[0]);
            return;
        }
    }

    if diag_height > 0 {
        draw_diagnostics_panel(f, app, chunks[1]);
    }
    draw_input_area(f, app, chunks[2]);
}

fn draw_file_tree(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;
    let title = format!(" File Tree: {} ", app.file_tree.current_dir.display());
    let block = Block::default().borders(Borders::ALL).title(title);
    let inner_area = block.inner(area);
    f.render_widget(block, area);

    let mut items = Vec::new();
    for (i, path) in app.file_tree.items.iter().enumerate() {
        let file_name = path.file_name().unwrap_or_default().to_string_lossy();
        let icon = if path.is_dir() { "📁" } else { "📄" };
        let style = if i == app.file_tree.selected_index {
            Style::default().fg(t.selection_fg).bg(t.selection_bg).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(t.text)
        };
        items.push(Line::from(Span::styled(format!("{} {}", icon, file_name), style)));
    }

    let paragraph = Paragraph::new(items).scroll((0, 0));
    f.render_widget(paragraph, inner_area);
}

fn draw_diff_view(f: &mut Frame, app: &App, area: Rect) {
    let mode_label = app.diff_view.view_mode.label();
    let title = format!(" Diff View [{}] (Press ESC or /chat to return) ", mode_label);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title);
    let inner_area = block.inner(area);
    f.render_widget(block, area);

    let lines = app.diff_view.render_lines();
    let paragraph = Paragraph::new(lines).scroll((app.diff_view.scroll, 0));
    f.render_widget(paragraph, inner_area);
}

pub fn draw_agent_view(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;
    let av = &app.agent_view;

    let status_label = match &av.status {
        AgentStatus::Running => " Agent: Running... ",
        AgentStatus::WaitingApproval => " Agent: Awaiting Approval — y/n/a ",
        AgentStatus::Complete(_) => " Agent: Complete ",
        AgentStatus::Error(_) => " Agent: Error ",
    };
    let status_style = match &av.status {
        AgentStatus::WaitingApproval => Style::default().fg(t.warning).add_modifier(Modifier::BOLD),
        AgentStatus::Complete(_)     => Style::default().fg(t.success).add_modifier(Modifier::BOLD),
        AgentStatus::Error(_)        => Style::default().fg(t.error).add_modifier(Modifier::BOLD),
        AgentStatus::Running         => Style::default().fg(t.primary).add_modifier(Modifier::BOLD),
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(status_label, status_style));
    let inner_area = block.inner(area);
    f.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();

    for step in &av.steps {
        render_step(&mut lines, step, t);
    }

    if !av.streaming_text.is_empty() {
        lines.push(Line::from(Span::styled(
            " AI: ",
            Style::default().fg(t.primary).add_modifier(Modifier::BOLD),
        )));
        for l in av.streaming_text.lines() {
            lines.push(Line::from(Span::raw(l.to_string())));
        }
        lines.push(Line::from(Span::styled("▋", Style::default().fg(t.text))));
        lines.push(Line::from(""));
    }

    if let Some(call) = &av.pending_call {
        lines.push(Line::from(Span::styled(
            "─── Approval Required ───────────────────────────",
            Style::default().fg(t.warning),
        )));
        lines.push(Line::from(vec![
            Span::styled(" Tool: ", Style::default().fg(t.warning).add_modifier(Modifier::BOLD)),
            Span::styled(call.name(), Style::default().fg(t.text).add_modifier(Modifier::BOLD)),
        ]));
        lines.push(Line::from(vec![
            Span::styled(" Call: ", Style::default().fg(t.warning)),
            Span::raw(call.summary()),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  [y] Approve   [n] Reject   [a] Approve all",
            Style::default().fg(t.success),
        )));
        lines.push(Line::from(""));
    }

    match &av.status {
        AgentStatus::Complete(summary) => {
            lines.push(Line::from(Span::styled(
                format!("✅ {}", summary),
                Style::default().fg(t.success).add_modifier(Modifier::BOLD),
            )));
        }
        AgentStatus::Error(e) => {
            lines.push(Line::from(Span::styled(
                format!("❌ {}", e),
                Style::default().fg(t.error).add_modifier(Modifier::BOLD),
            )));
        }
        _ => {}
    }

    let total = lines.len() as u16;
    let height = inner_area.height;
    let max_scroll = total.saturating_sub(height);
    let scroll = max_scroll.saturating_sub(av.scroll);

    let paragraph = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0));
    f.render_widget(paragraph, inner_area);
}

fn render_step(lines: &mut Vec<Line>, step: &AgentStep, t: &crate::tui::theme::Theme) {
    let icon = if step.tool_result.success { "✅" } else { "❌" };
    lines.push(Line::from(vec![
        Span::styled(
            format!(" {} Step {} — ", icon, step.step_num + 1),
            Style::default().fg(t.dim),
        ),
        Span::styled(
            step.tool_call.summary(),
            Style::default().fg(t.text).add_modifier(Modifier::BOLD),
        ),
    ]));
    for l in step.tool_result.output.lines().take(3) {
        lines.push(Line::from(Span::styled(
            format!("   {}", l),
            Style::default().fg(t.dim),
        )));
    }
    lines.push(Line::from(""));
}

fn draw_main_area(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;
    let mut lines = Vec::new();

    if app.messages.is_empty() {
        let logo_text = vec![
            Line::from(Span::styled("Welcome back User!", Style::default().add_modifier(Modifier::BOLD))),
            Line::from(""),
            Line::from(Span::styled("  o              o     o      o                       ", Style::default().fg(t.logo))),
            Line::from(Span::styled(" <|>            <|>  _<|>_   <|>                      ", Style::default().fg(t.logo))),
            Line::from(Span::styled(" < >            < >          / >                      ", Style::default().fg(t.logo))),
            Line::from(Span::styled("  \\o            o/     o    \\o__ __o       o__  __o  ", Style::default().fg(t.logo))),
            Line::from(Span::styled("   v\\          /v     <|>    |     v\\     /v      |> ", Style::default().fg(t.logo))),
            Line::from(Span::styled("    <\\        />      / \\   / \\     <\\   />      //  ", Style::default().fg(t.logo))),
            Line::from(Span::styled("      \\o    o/        \\o/   \\o/      /   \\o    o/    ", Style::default().fg(t.logo))),
            Line::from(Span::styled("       v\\  /v          |     |      o     v\\  /v __o ", Style::default().fg(t.logo))),
            Line::from(Span::styled("        <\\/>          / \\   / \\  __/>      <\\/> __/> ", Style::default().fg(t.logo))),
            Line::from(""),
            Line::from(Span::styled("Vibe Model • Vibe Max", Style::default().fg(t.dim))),
            Line::from(""),
            Line::from(Span::styled("Tips for getting started", Style::default().fg(t.secondary).add_modifier(Modifier::BOLD))),
            Line::from("Run /init to initialize project configuration"),
            Line::from("Run /help to see available commands"),
            Line::from("Run /agent <task> to start the coding agent"),
            Line::from(""),
        ];
        lines.extend(logo_text);
    }

    for msg in &app.messages {
        match msg {
            crate::tui::app::TuiMessage::User(content) => {
                lines.push(Line::from(vec![
                    Span::styled(" > ", Style::default().fg(t.text).add_modifier(Modifier::BOLD)),
                    Span::raw(content),
                ]));
                lines.push(Line::from(""));
            }
            crate::tui::app::TuiMessage::Assistant(content) => {
                lines.push(Line::from(Span::styled(
                    " AI: ",
                    Style::default().fg(t.primary).add_modifier(Modifier::BOLD),
                )));
                for line in content.lines() {
                    lines.push(Line::from(Span::raw(line)));
                }
                lines.push(Line::from(""));
            }
            crate::tui::app::TuiMessage::AssistantStreaming(content) => {
                lines.push(Line::from(Span::styled(
                    " AI: ",
                    Style::default().fg(t.primary).add_modifier(Modifier::BOLD),
                )));
                for line in content.lines() {
                    lines.push(Line::from(Span::raw(line)));
                }
                lines.push(Line::from(Span::styled("▋", Style::default().fg(t.text))));
                lines.push(Line::from(""));
            }
            crate::tui::app::TuiMessage::System(content) => {
                lines.push(Line::from(Span::styled(
                    " Sys: ",
                    Style::default().fg(t.secondary).add_modifier(Modifier::BOLD),
                )));
                for line in content.lines() {
                    lines.push(Line::from(Span::styled(
                        line,
                        Style::default().fg(t.secondary),
                    )));
                }
                lines.push(Line::from(""));
            }
            crate::tui::app::TuiMessage::CommandOutput { command, output } => {
                lines.push(Line::from(Span::styled(
                    format!(" $ {}", command),
                    Style::default().fg(t.success),
                )));
                for line in output.lines() {
                    lines.push(Line::from(Span::styled(
                        line,
                        Style::default().fg(t.dim),
                    )));
                }
                lines.push(Line::from(""));
            }
            crate::tui::app::TuiMessage::FileList { path, files } => {
                lines.push(Line::from(Span::styled(
                    format!(" 📂 {}", path),
                    Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
                )));
                for file in files {
                    let icon = if file.ends_with('/') { "📁" } else { "📄" };
                    lines.push(Line::from(format!("   {} {}", icon, file)));
                }
                lines.push(Line::from(""));
            }
            crate::tui::app::TuiMessage::Diff { file, diff } => {
                lines.push(Line::from(Span::styled(
                    format!(" 📊 Diff: {}", file),
                    Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
                )));
                for line in diff.lines() {
                    let style = if line.starts_with('+') {
                        Style::default().fg(t.success)
                    } else if line.starts_with('-') {
                        Style::default().fg(t.error)
                    } else if line.starts_with("@@") {
                        Style::default().fg(t.info)
                    } else {
                        Style::default().fg(t.dim)
                    };
                    lines.push(Line::from(Span::styled(line, style)));
                }
                lines.push(Line::from(""));
            }
            crate::tui::app::TuiMessage::Error(content) => {
                lines.push(Line::from(Span::styled(
                    format!(" ❌ Error: {}", content),
                    Style::default().fg(t.error).add_modifier(Modifier::BOLD),
                )));
                lines.push(Line::from(""));
            }
        }
    }

    let scroll = compute_scroll(lines.len() as u16, area.height, app.scroll_offset);

    let paragraph = Paragraph::new(lines)
        .block(Block::default())
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0));

    f.render_widget(paragraph, area);
}

fn draw_input_area(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)].as_ref())
        .split(area);

    let input_prefix = Span::styled(" > ", Style::default().add_modifier(Modifier::BOLD));
    let input_text = Span::raw(&app.input);
    let cursor = Span::styled("█", Style::default().fg(t.text));
    let input_line = Line::from(vec![input_prefix, input_text, cursor]);
    f.render_widget(Paragraph::new(input_line), chunks[0]);

    let status_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(chunks[1]);

    let hints = match app.current_screen {
        CurrentScreen::Agent => {
            Paragraph::new(" y=approve  n=reject  a=approve-all  ESC=chat")
                .style(Style::default().fg(t.warning))
        }
        _ => Paragraph::new(" ? for shortcuts").style(Style::default().fg(t.dim)),
    };
    f.render_widget(hints, status_chunks[0]);

    let status_text = match app.current_screen {
        CurrentScreen::Agent => "Agent view (Tab to return)",
        _ => "Thinking on (tab to toggle)",
    };
    let status = Paragraph::new(status_text)
        .alignment(Alignment::Right)
        .style(Style::default().fg(t.dim));
    f.render_widget(status, status_chunks[1]);
}

// ── Diagnostics panel ─────────────────────────────────────────────────────────

/// Compute the vertical scroll offset for an auto-scrolling view.
///
/// `total_lines` is the number of content lines, `view_height` is the visible
/// rows, and `user_offset` is how far the user has scrolled back from the
/// bottom. Returns the `(row, col)` scroll value suitable for
/// `Paragraph::scroll`.
pub(crate) fn compute_scroll(total_lines: u16, view_height: u16, user_offset: u16) -> u16 {
    let max_scroll = total_lines.saturating_sub(view_height);
    max_scroll.saturating_sub(user_offset)
}

/// Classify a diff line for syntax coloring.  Returns a tag used to pick the
/// appropriate theme color.
#[cfg(test)]
pub(crate) fn classify_diff_line(line: &str) -> &'static str {
    if line.starts_with('+') {
        "added"
    } else if line.starts_with('-') {
        "removed"
    } else if line.starts_with("@@") {
        "hunk_header"
    } else {
        "context"
    }
}

/// Compute the diagnostics panel height: 4 lines when items exist, 0
/// otherwise.
pub(crate) fn diag_panel_height(item_count: usize) -> u16 {
    if item_count == 0 { 0 } else { 4 }
}

fn draw_diagnostics_panel(f: &mut Frame, app: &App, area: Rect) {
    use crate::tui::components::diagnostics::DiagSeverity;
    use ratatui::style::Color;

    let t = &app.theme;
    let dp = &app.diagnostics_panel;

    let count = dp.items.len();
    let title = format!(" Diagnostics ({}) — /check to refresh ", count);
    let block = Block::default()
        .borders(Borders::TOP)
        .title(title)
        .title_style(Style::default().fg(t.secondary).add_modifier(Modifier::BOLD));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let lines: Vec<Line> = dp
        .items
        .iter()
        .map(|d| {
            let (icon, color) = match d.severity {
                DiagSeverity::Error   => ("E", Color::Red),
                DiagSeverity::Warning => ("W", Color::Yellow),
                DiagSeverity::Info    => ("I", Color::Cyan),
            };
            let loc = if d.line > 0 {
                format!("{}:{}", d.file, d.line)
            } else {
                d.file.clone()
            };
            Line::from(vec![
                Span::styled(format!("[{}]", icon), Style::default().fg(color)),
                Span::raw(" "),
                if !loc.is_empty() {
                    Span::styled(format!("{}: ", loc), Style::default().fg(t.dim))
                } else {
                    Span::raw("")
                },
                Span::styled(
                    d.message.chars().take(120).collect::<String>(),
                    Style::default().fg(t.text),
                ),
            ])
        })
        .collect();

    let para = Paragraph::new(lines).scroll((dp.scroll, 0));
    f.render_widget(para, inner);
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── compute_scroll ────────────────────────────────────────────────────

    #[test]
    fn compute_scroll_content_fits_in_view() {
        // 10 lines in a 20-line view: max_scroll = 0, result = 0
        assert_eq!(compute_scroll(10, 20, 0), 0);
    }

    #[test]
    fn compute_scroll_content_exceeds_view_no_offset() {
        // 50 lines in a 20-line view: max_scroll = 30, offset = 0 → scroll = 30
        assert_eq!(compute_scroll(50, 20, 0), 30);
    }

    #[test]
    fn compute_scroll_with_user_offset() {
        // 50 lines, 20 view, user scrolled back 10 → 30 - 10 = 20
        assert_eq!(compute_scroll(50, 20, 10), 20);
    }

    #[test]
    fn compute_scroll_user_offset_exceeds_max() {
        // 50 lines, 20 view, max_scroll = 30, user offset 50 → saturating_sub → 0
        assert_eq!(compute_scroll(50, 20, 50), 0);
    }

    #[test]
    fn compute_scroll_zero_lines() {
        assert_eq!(compute_scroll(0, 20, 0), 0);
    }

    #[test]
    fn compute_scroll_zero_height() {
        // 10 lines in a 0-height view: max_scroll = 10
        assert_eq!(compute_scroll(10, 0, 0), 10);
    }

    #[test]
    fn compute_scroll_equal_content_and_view() {
        // Exact fit: no scrolling needed
        assert_eq!(compute_scroll(20, 20, 0), 0);
    }

    // ── classify_diff_line ────────────────────────────────────────────────

    #[test]
    fn classify_added_line() {
        assert_eq!(classify_diff_line("+added line"), "added");
    }

    #[test]
    fn classify_removed_line() {
        assert_eq!(classify_diff_line("-removed line"), "removed");
    }

    #[test]
    fn classify_hunk_header() {
        assert_eq!(classify_diff_line("@@ -1,3 +1,4 @@"), "hunk_header");
    }

    #[test]
    fn classify_context_line() {
        assert_eq!(classify_diff_line(" unchanged line"), "context");
    }

    #[test]
    fn classify_empty_line() {
        assert_eq!(classify_diff_line(""), "context");
    }

    #[test]
    fn classify_plus_only() {
        assert_eq!(classify_diff_line("+"), "added");
    }

    #[test]
    fn classify_minus_only() {
        assert_eq!(classify_diff_line("-"), "removed");
    }

    #[test]
    fn classify_at_but_not_double() {
        // Single @ is context, not a hunk header
        assert_eq!(classify_diff_line("@ something"), "context");
    }

    // ── diag_panel_height ─────────────────────────────────────────────────

    #[test]
    fn diag_panel_height_zero_items() {
        assert_eq!(diag_panel_height(0), 0);
    }

    #[test]
    fn diag_panel_height_some_items() {
        assert_eq!(diag_panel_height(1), 4);
        assert_eq!(diag_panel_height(100), 4);
    }
}
