use ratatui::{
    layout::{Constraint, Direction, Layout, Rect, Alignment},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::tui::app::App;

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),     // Main Content (flexible)
            Constraint::Length(3),  // Input
        ].as_ref())
        .split(f.area());

    match app.current_screen {
        crate::tui::app::CurrentScreen::Chat => draw_main_area(f, app, chunks[0]),
        crate::tui::app::CurrentScreen::DiffView => draw_diff_view(f, app, chunks[0]),
        crate::tui::app::CurrentScreen::FileTree => draw_file_tree(f, app, chunks[0]),
    }
    
    draw_input_area(f, app, chunks[1]);
}

// draw_file_tree and draw_diff_view remain the same...

fn draw_file_tree(f: &mut Frame, app: &App, area: Rect) {
    let title = format!(" File Tree: {} ", app.file_tree.current_dir.display());
    let block = Block::default().borders(Borders::ALL).title(title);
    let inner_area = block.inner(area);
    f.render_widget(block, area);

    let mut items = Vec::new();
    for (i, path) in app.file_tree.items.iter().enumerate() {
        let file_name = path.file_name().unwrap_or_default().to_string_lossy();
        let icon = if path.is_dir() { "📁" } else { "📄" };
        let style = if i == app.file_tree.selected_index {
            Style::default().fg(Color::Black).bg(Color::Blue).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        items.push(Line::from(Span::styled(format!("{} {}", icon, file_name), style)));
    }

    let paragraph = Paragraph::new(items)
        .scroll((0, 0)); // TODO: Add scrolling for file tree if needed
    
    f.render_widget(paragraph, inner_area);
}

fn draw_diff_view(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title(" Diff View (Press ESC or /chat to return) ");
    let inner_area = block.inner(area);
    f.render_widget(block, area);

    let mut lines = Vec::new();

    if !app.diff_view.raw_lines.is_empty() {
        for line in &app.diff_view.raw_lines {
            let style = if line.starts_with('+') {
                Style::default().fg(Color::Green)
            } else if line.starts_with('-') {
                Style::default().fg(Color::Red)
            } else if line.starts_with("@@") {
                Style::default().fg(Color::Cyan)
            } else if line.starts_with("diff") || line.starts_with("index") {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };
            lines.push(Line::from(Span::styled(line, style)));
        }
    } else {
        for hunk in &app.diff_view.hunks {
            lines.push(Line::from(Span::styled(
                format!("@@ -{},{} +{},{} @@", hunk.old_start, hunk.old_count, hunk.new_start, hunk.new_count),
                Style::default().fg(Color::Cyan)
            )));
            for line in &hunk.lines {
                let (prefix, style) = match line.tag {
                    vibe_core::diff::DiffTag::Equal => (" ", Style::default().fg(Color::Gray)),
                    vibe_core::diff::DiffTag::Insert => ("+", Style::default().fg(Color::Green)),
                    vibe_core::diff::DiffTag::Delete => ("-", Style::default().fg(Color::Red)),
                };
                lines.push(Line::from(Span::styled(format!("{}{}", prefix, line.content), style)));
            }
        }
    }

    let paragraph = Paragraph::new(lines)
        .scroll((app.diff_view.scroll, 0));
    
    f.render_widget(paragraph, inner_area);
}

fn draw_main_area(f: &mut Frame, app: &App, area: Rect) {
    let mut lines = Vec::new();

    // Render Welcome Header if chat is empty
    if app.messages.is_empty() {
        let logo_text = vec![
            Line::from(Span::styled("Welcome back User!", Style::default().add_modifier(Modifier::BOLD))),
            Line::from(""),
            Line::from(Span::styled("  o              o     o      o                       ", Style::default().fg(Color::Rgb(255, 100, 100)))),
            Line::from(Span::styled(" <|>            <|>  _<|>_   <|>                      ", Style::default().fg(Color::Rgb(255, 100, 100)))),
            Line::from(Span::styled(" < >            < >          / >                      ", Style::default().fg(Color::Rgb(255, 100, 100)))),
            Line::from(Span::styled("  \\o            o/     o    \\o__ __o       o__  __o  ", Style::default().fg(Color::Rgb(255, 100, 100)))),
            Line::from(Span::styled("   v\\          /v     <|>    |     v\\     /v      |> ", Style::default().fg(Color::Rgb(255, 100, 100)))),
            Line::from(Span::styled("    <\\        />      / \\   / \\     <\\   />      //  ", Style::default().fg(Color::Rgb(255, 100, 100)))),
            Line::from(Span::styled("      \\o    o/        \\o/   \\o/      /   \\o    o/    ", Style::default().fg(Color::Rgb(255, 100, 100)))),
            Line::from(Span::styled("       v\\  /v          |     |      o     v\\  /v __o ", Style::default().fg(Color::Rgb(255, 100, 100)))),
            Line::from(Span::styled("        <\\/>          / \\   / \\  __/>      <\\/> __/> ", Style::default().fg(Color::Rgb(255, 100, 100)))),
            Line::from(""),
            Line::from(Span::styled("Vibe Model • Vibe Max", Style::default().fg(Color::DarkGray))),
            Line::from(""),
            Line::from(Span::styled("Tips for getting started", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
            Line::from("Run /init to initialize project configuration"),
            Line::from("Run /help to see available commands"),
            Line::from(""),
        ];
        lines.extend(logo_text);
    }

    for msg in &app.messages {
        match msg {
            crate::tui::app::TuiMessage::User(content) => {
                lines.push(Line::from(vec![
                    Span::styled(" > ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                    Span::raw(content),
                ]));
                lines.push(Line::from(""));
            }
            crate::tui::app::TuiMessage::Assistant(content) => {
                lines.push(Line::from(Span::styled(" AI: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))));
                for line in content.lines() {
                    lines.push(Line::from(Span::raw(line)));
                }
                lines.push(Line::from(""));
            }
            crate::tui::app::TuiMessage::System(content) => {
                lines.push(Line::from(Span::styled(" Sys: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))));
                for line in content.lines() {
                    lines.push(Line::from(Span::styled(line, Style::default().fg(Color::Yellow))));
                }
                lines.push(Line::from(""));
            }
            crate::tui::app::TuiMessage::CommandOutput { command, output } => {
                lines.push(Line::from(Span::styled(format!(" $ {}", command), Style::default().fg(Color::Green))));
                for line in output.lines() {
                    lines.push(Line::from(Span::styled(line, Style::default().fg(Color::DarkGray))));
                }
                lines.push(Line::from(""));
            }
            crate::tui::app::TuiMessage::FileList { path, files } => {
                lines.push(Line::from(Span::styled(format!(" 📂 {}", path), Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD))));
                for file in files {
                    let icon = if file.ends_with('/') { "📁" } else { "📄" };
                    lines.push(Line::from(format!("   {} {}", icon, file)));
                }
                lines.push(Line::from(""));
            }
            crate::tui::app::TuiMessage::Diff { file, diff } => {
                lines.push(Line::from(Span::styled(format!(" 📊 Diff: {}", file), Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD))));
                for line in diff.lines() {
                    let style = if line.starts_with('+') {
                        Style::default().fg(Color::Green)
                    } else if line.starts_with('-') {
                        Style::default().fg(Color::Red)
                    } else if line.starts_with("@@") {
                        Style::default().fg(Color::Cyan)
                    } else {
                        Style::default().fg(Color::Gray)
                    };
                    lines.push(Line::from(Span::styled(line, style)));
                }
                lines.push(Line::from(""));
            }
            crate::tui::app::TuiMessage::Error(content) => {
                lines.push(Line::from(Span::styled(format!(" ❌ Error: {}", content), Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))));
                lines.push(Line::from(""));
            }
        }
    }

    // Calculate scroll position (stick to bottom)
    let total_lines = lines.len() as u16;
    let view_height = area.height;
    let max_scroll = total_lines.saturating_sub(view_height);
    let scroll = max_scroll.saturating_sub(app.scroll_offset);

    let block = Block::default(); 
    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0));

    f.render_widget(paragraph, area);
}

fn draw_input_area(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)].as_ref())
        .split(area);

    // Input Line
    let input_prefix = Span::styled(" > ", Style::default().add_modifier(Modifier::BOLD));
    let input_text = Span::raw(&app.input);
    let cursor = Span::styled("█", Style::default().fg(Color::White)); // Fake cursor
    
    let input_line = Line::from(vec![input_prefix, input_text, cursor]);
    let input = Paragraph::new(input_line);
    f.render_widget(input, chunks[0]);

    // Status Line
    let status_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(chunks[1]);

    let hints = Paragraph::new(" ? for shortcuts").style(Style::default().fg(Color::DarkGray));
    f.render_widget(hints, status_chunks[0]);

    let status = Paragraph::new("Thinking on (tab to toggle)")
        .alignment(Alignment::Right)
        .style(Style::default().fg(Color::DarkGray)); // TODO: Dynamic status
    f.render_widget(status, status_chunks[1]);
}
