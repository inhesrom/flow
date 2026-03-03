use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use crate::app::TuiApp;
use protocol::Route;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceHit {
    Header,
    TerminalTabs(protocol::TerminalKind),
    TerminalPane,
    FilesList(usize),
    DiffPane,
}

#[derive(Debug, Clone, Copy)]
struct WorkspaceLayout {
    header: Rect,
    terminal_tabs: Rect,
    terminal_pane: Rect,
    git_files: Rect,
    git_diff: Rect,
}

fn layout(area: Rect) -> WorkspaceLayout {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(5)])
        .split(area);
    let body = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(chunks[1]);
    let terminal_area = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(3)])
        .split(body[0]);
    let git_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(body[1]);

    WorkspaceLayout {
        header: chunks[0],
        terminal_tabs: terminal_area[0],
        terminal_pane: terminal_area[1],
        git_files: git_area[0],
        git_diff: git_area[1],
    }
}

pub fn render(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let l = layout(area);

    let focused_border = |focused: bool| {
        if focused {
            Style::default()
                .fg(Color::LightCyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        }
    };

    let ws_id = match app.route {
        Route::Workspace { id } => Some(id),
        _ => None,
    };
    let title = ws_id
        .and_then(|id| app.workspaces.iter().find(|w| w.id == id))
        .map(|w| format!("Workspace: {} ({})", w.name, w.path))
        .unwrap_or_else(|| "Workspace".to_string());

    frame.render_widget(
        Paragraph::new(if let Some(name) = &app.rename_workspace_input {
            format!("{title}\nRename: {name}")
        } else {
            title
        })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(focused_border(app.focus == crate::app::Focus::WsHeader)),
        ),
        l.header,
    );

    let files = ws_id
        .and_then(|id| app.workspace_git.get(&id))
        .map(|g| g.changed.clone())
        .unwrap_or_default();
    let mut list_state = ListState::default();
    if !files.is_empty() {
        list_state.select(Some(app.ws_selected_file.min(files.len() - 1)));
    }
    let file_items = files
        .iter()
        .map(|f| ListItem::new(format!("{:>2} {}", f.status, f.path)))
        .collect::<Vec<_>>();
    let file_list = List::new(file_items)
        .block(
            Block::default()
                .title("Changed Files")
                .borders(Borders::ALL)
                .border_style(focused_border(app.focus == crate::app::Focus::WsFiles)),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_stateful_widget(file_list, l.git_files, &mut list_state);

    let diff_text = ws_id
        .and_then(|id| app.workspace_diff.get(&id))
        .map(|(_, d)| d.clone())
        .unwrap_or_else(|| "Select a file and press Enter to load diff.".to_string());
    let diff_lines = diff_text
        .lines()
        .map(|line| {
            if line.starts_with('+') {
                Line::from(Span::styled(
                    line.to_string(),
                    Style::default().fg(Color::Green),
                ))
            } else if line.starts_with('-') {
                Line::from(Span::styled(
                    line.to_string(),
                    Style::default().fg(Color::Red),
                ))
            } else {
                Line::from(Span::raw(line.to_string()))
            }
        })
        .collect::<Vec<_>>();
    frame.render_widget(
        Paragraph::new(diff_lines)
            .block(
                Block::default()
                    .title("Diff")
                    .borders(Borders::ALL)
                    .border_style(focused_border(app.focus == crate::app::Focus::WsDiff)),
            )
            .scroll((app.ws_diff_scroll, 0))
            .wrap(Wrap { trim: false }),
        l.git_diff,
    );

    let ws_summary = ws_id.and_then(|id| app.workspaces.iter().find(|w| w.id == id));
    let (agent_running, shell_running) = ws_summary
        .map(|w| (w.agent_running, w.shell_running))
        .unwrap_or((false, false));
    let tabs = format!(
        "{} {} | {} {} | switch: `1`/`2`   start: `a`/`s` stop: `A`/`S`",
        if app.ws_active_terminal == protocol::TerminalKind::Agent {
            "[1] Agent*"
        } else {
            "[1] Agent"
        },
        if agent_running { "running" } else { "stopped" },
        if app.ws_active_terminal == protocol::TerminalKind::Shell {
            "[2] Shell*"
        } else {
            "[2] Shell"
        },
        if shell_running { "running" } else { "stopped" },
    );
    frame.render_widget(
        Paragraph::new(tabs).block(
            Block::default()
                .title("Terminal Tabs")
                .borders(Borders::ALL)
                .border_style(focused_border(
                    app.focus == crate::app::Focus::WsTerminalTabs,
                )),
        ),
        l.terminal_tabs,
    );

    let terminal_lines = ws_id
        .map(|id| app.terminal_lines(id, app.ws_active_terminal))
        .unwrap_or_else(|| vec![Line::from("No terminal output yet.")]);
    frame.render_widget(
        Paragraph::new(terminal_lines)
            .block(
                Block::default()
                    .title("Terminal")
                    .borders(Borders::ALL)
                    .border_style(focused_border(app.focus == crate::app::Focus::WsTerminal)),
            ),
        l.terminal_pane,
    );
}

pub fn hit_test(area: Rect, app: &TuiApp, x: u16, y: u16) -> Option<WorkspaceHit> {
    let l = layout(area);

    let point_inside = |r: Rect| x >= r.x && y >= r.y && x < r.right() && y < r.bottom();
    if point_inside(l.header) {
        return Some(WorkspaceHit::Header);
    }
    if point_inside(l.terminal_tabs) {
        let mid = l.terminal_tabs.x + l.terminal_tabs.width / 2;
        let kind = if x < mid {
            protocol::TerminalKind::Agent
        } else {
            protocol::TerminalKind::Shell
        };
        return Some(WorkspaceHit::TerminalTabs(kind));
    }
    if point_inside(l.terminal_pane) {
        return Some(WorkspaceHit::TerminalPane);
    }
    if point_inside(l.git_diff) {
        return Some(WorkspaceHit::DiffPane);
    }
    if point_inside(l.git_files) {
        let ws_id = match app.route {
            Route::Workspace { id } => id,
            _ => return None,
        };
        let file_count = app
            .workspace_git
            .get(&ws_id)
            .map(|g| g.changed.len())
            .unwrap_or(0);
        if file_count == 0 {
            return Some(WorkspaceHit::FilesList(0));
        }

        let content_top = l.git_files.y.saturating_add(1);
        if y < content_top {
            return Some(WorkspaceHit::FilesList(0));
        }
        let idx = (y - content_top) as usize;
        return Some(WorkspaceHit::FilesList(idx.min(file_count - 1)));
    }
    None
}

pub fn terminal_content_rect(area: Rect) -> Rect {
    let pane = layout(area).terminal_pane;
    Rect::new(
        pane.x.saturating_add(1),
        pane.y.saturating_add(1),
        pane.width.saturating_sub(2),
        pane.height.saturating_sub(2),
    )
}
