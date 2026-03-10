use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
    layout::Rect,
};

use crate::app::{Focus, TuiApp};
use protocol::Route;

/// Returns a bold white span for a keybinding label.
fn key(k: &str) -> Span<'static> {
    Span::styled(
        k.to_string(),
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    )
}

/// Returns a dark-gray span for a keybinding description.
fn desc(d: &str) -> Span<'static> {
    Span::styled(d.to_string(), Style::default().fg(Color::DarkGray))
}

/// Returns a two-space gap span used to separate hint groups.
fn gap() -> Span<'static> {
    Span::raw("  ")
}

/// Builds the context-sensitive key hint line displayed in the application footer.
///
/// Returns a `Line` whose spans vary based on the current route and focus state in `app`.
pub fn build_footer_hints(app: &TuiApp) -> Line<'static> {
    let spans = match app.route {
        Route::Home => {
            if app.ssh_history_picker.is_some() {
                vec![
                    key("j/k"), desc(" navigate"),
                    gap(),
                    key("Enter"), desc(" select"),
                    gap(),
                    key("n"), desc(" new"),
                    gap(),
                    key("Esc"), desc(" cancel"),
                ]
            } else if app.is_adding_ssh_workspace() {
                vec![
                    key("Tab"), desc(" next field"),
                    gap(),
                    key("Enter"), desc(" add"),
                    gap(),
                    key("Esc"), desc(" cancel"),
                ]
            } else if app.is_adding_workspace() {
                vec![
                    key("Esc"), desc(" cancel"),
                ]
            } else if app.is_settings_open() {
                vec![
                    key("j/k"), desc(" navigate"),
                    gap(),
                    key("Space"), desc(" toggle"),
                    gap(),
                    key("Esc"), desc(" close"),
                ]
            } else if app.is_confirming_delete() {
                vec![
                    key("Y"), desc(" confirm delete"),
                    gap(),
                    key("N"), desc(" cancel"),
                ]
            } else if app.is_renaming_workspace() {
                vec![
                    key("Enter"), desc(" confirm"),
                    gap(),
                    key("Esc"), desc(" cancel"),
                ]
            } else {
                vec![
                    key("Enter"), desc(" open"),
                    gap(),
                    key("n"), desc(" new"),
                    gap(),
                    key("R"), desc(" ssh"),
                    gap(),
                    key("e"), desc(" rename"),
                    gap(),
                    key("D"), desc(" delete"),
                    gap(),
                    key("!"), desc(" attention"),
                    gap(),
                    key("S"), desc(" settings"),
                    gap(),
                    key("q"), desc(" quit"),
                ]
            }
        }
        Route::Workspace { .. } => match app.focus {
            Focus::WsTerminalTabs => vec![
                key("h/l"), desc(" switch tab"),
                gap(),
                key("n"), desc(" new tab"),
                gap(),
                key("x"), desc(" close"),
                gap(),
                key("r"), desc(" rename"),
                gap(),
                key("F"), desc(" fullscreen"),
                gap(),
                key("Tab"), desc(" next pane"),
                gap(),
                key("Shift+Tab"), desc(" previous pane"),
                gap(),
                key("Esc"), desc(" home"),
            ],
            Focus::WsTerminal => vec![
                desc("(keys sent to terminal)"),
                gap(),
                key("Ctrl+G"), desc(" passthrough"),
                gap(),
                key("F"), desc(" fullscreen"),
                gap(),
                key("Tab"), desc(" next pane"),
                gap(),
                key("Shift+Tab"), desc(" previous pane"),
                gap(),
                key("Esc"), desc(" unfocus"),
            ],
            Focus::WsBranches => vec![
                key("j/k"), desc(" navigate"),
                gap(),
                key("[/]"), desc(" local/remote"),
                gap(),
                key("Space"), desc(" checkout"),
                gap(),
                key("c"), desc(" create"),
                gap(),
                key("p"), desc(" pull"),
                gap(),
                key("f"), desc(" fetch"),
                gap(),
                key("P"), desc(" push"),
                gap(),
                key("F"), desc(" fullscreen"),
                gap(),
                key("Tab"), desc(" next pane"),
                gap(),
                key("Shift+Tab"), desc(" previous pane"),
                gap(),
                key("Esc"), desc(" home"),
            ],
            Focus::WsLog => match app.log_item_at(app.ws_selected_commit) {
                crate::app::LogItem::UncommittedHeader => vec![
                    key("j/k"), desc(" navigate"),
                    gap(),
                    key("Enter"), desc(" expand/collapse"),
                    gap(),
                    key("+/-"), desc(" stage all"),
                    gap(),
                    key("c"), desc(" commit"),
                    gap(),
                    key("s"), desc(" stash"),
                    gap(),
                    key("t"), desc(" tags"),
                    gap(),
                    key("F"), desc(" fullscreen"),
                    gap(),
                    key("Tab"), desc(" next pane"),
                    gap(),
                    key("Shift+Tab"), desc(" previous pane"),
                    gap(),
                    key("Esc"), desc(" home"),
                ],
                crate::app::LogItem::ChangedFile(_) => vec![
                    key("j/k"), desc(" navigate"),
                    gap(),
                    key("Space"), desc(" stage/unstage"),
                    gap(),
                    key("+/-"), desc(" all"),
                    gap(),
                    key("c"), desc(" commit"),
                    gap(),
                    key("d"), desc(" discard"),
                    gap(),
                    key("s"), desc(" stash"),
                    gap(),
                    key("Enter"), desc(" diff"),
                    gap(),
                    key("t"), desc(" tags"),
                    gap(),
                    key("F"), desc(" fullscreen"),
                    gap(),
                    key("Tab"), desc(" next pane"),
                    gap(),
                    key("Shift+Tab"), desc(" previous pane"),
                    gap(),
                    key("Esc"), desc(" home"),
                ],
                crate::app::LogItem::Commit(_) => vec![
                    key("j/k"), desc(" navigate"),
                    gap(),
                    key("Enter"), desc(" expand/collapse"),
                    gap(),
                    key("t"), desc(" tags"),
                    gap(),
                    key("F"), desc(" fullscreen"),
                    gap(),
                    key("Tab"), desc(" next pane"),
                    gap(),
                    key("Shift+Tab"), desc(" previous pane"),
                    gap(),
                    key("Esc"), desc(" home"),
                ],
                crate::app::LogItem::CommitFile(_, _) => vec![
                    key("j/k"), desc(" navigate"),
                    gap(),
                    key("Enter"), desc(" diff"),
                    gap(),
                    key("t"), desc(" tags"),
                    gap(),
                    key("F"), desc(" fullscreen"),
                    gap(),
                    key("Tab"), desc(" next pane"),
                    gap(),
                    key("Shift+Tab"), desc(" previous pane"),
                    gap(),
                    key("Esc"), desc(" home"),
                ],
            },
            Focus::WsDiff => vec![
                key("j/k"), desc(" scroll"),
                gap(),
                key("F"), desc(" fullscreen"),
                gap(),
                key("Tab"), desc(" next pane"),
                gap(),
                key("Shift+Tab"), desc(" previous pane"),
                gap(),
                key("Esc"), desc(" home"),
            ],
            _ => vec![
                key("F"), desc(" fullscreen"),
                gap(),
                key("Tab"), desc(" next pane"),
                gap(),
                key("Shift+Tab"), desc(" previous pane"),
                gap(),
                key("Esc"), desc(" home"),
            ],
        },
    };

    Line::from(spans)
}

/// Renders the context-sensitive key hint footer into `area`.
pub fn render(frame: &mut Frame, area: Rect, app: &TuiApp) {
    frame.render_widget(
        Paragraph::new(build_footer_hints(app))
            .block(Block::default().borders(Borders::TOP))
            .style(Style::default().fg(Color::Gray)),
        area,
    );
}
