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
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(3),
        ].as_ref())
        .split(f.area());

    match app.current_screen {
        CurrentScreen::Chat => draw_main_area(f, app, chunks[0]),
        CurrentScreen::DiffView => draw_diff_view(f, app, chunks[0]),
        CurrentScreen::FileTree => draw_file_tree(f, app, chunks[0]),
        CurrentScreen::Agent => draw_agent_view(f, app, chunks[0]),
    }

    draw_input_area(f, app, chunks[1]);
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
    let t = &app.theme;
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Diff View (Press ESC or /chat to return) ");
    let inner_area = block.inner(area);
    f.render_widget(block, area);

    let mut lines = Vec::new();

    if !app.diff_view.raw_lines.is_empty() {
        for line in &app.diff_view.raw_lines {
            let style = if line.starts_with('+') {
                Style::default().fg(t.success)
            } else if line.starts_with('-') {
                Style::default().fg(t.error)
            } else if line.starts_with("@@") {
                Style::default().fg(t.info)
            } else if line.starts_with("diff") || line.starts_with("index") {
                Style::default().fg(t.secondary).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(t.dim)
            };
            lines.push(Line::from(Span::styled(line, style)));
        }
    } else {
        for hunk in &app.diff_view.hunks {
            lines.push(Line::from(Span::styled(
                format!(
                    "@@ -{},{} +{},{} @@",
                    hunk.old_start, hunk.old_count, hunk.new_start, hunk.new_count
                ),
                Style::default().fg(t.info),
            )));
            for line in &hunk.lines {
                let (prefix, style) = match line.tag {
                    vibe_core::diff::DiffTag::Equal  => (" ", Style::default().fg(t.dim)),
                    vibe_core::diff::DiffTag::Insert => ("+", Style::default().fg(t.success)),
                    vibe_core::diff::DiffTag::Delete => ("-", Style::default().fg(t.error)),
                };
                lines.push(Line::from(Span::styled(
                    format!("{}{}", prefix, line.content),
                    style,
                )));
            }
        }
    }

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

    let total_lines = lines.len() as u16;
    let view_height = area.height;
    let max_scroll = total_lines.saturating_sub(view_height);
    let scroll = max_scroll.saturating_sub(app.scroll_offset);

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
