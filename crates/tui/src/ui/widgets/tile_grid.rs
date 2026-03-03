use protocol::{AttentionLevel, WorkspaceSummary};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub const TILE_W: u16 = 34;
pub const TILE_H: u16 = 9;

pub fn render(
    frame: &mut Frame,
    area: Rect,
    items: &[WorkspaceSummary],
    selected: usize,
    flash_on: bool,
) {
    if items.is_empty() {
        frame.render_widget(
            Paragraph::new("No workspaces yet. Press `n` to add current directory.")
                .block(Block::default().title("Home").borders(Borders::ALL)),
            area,
        );
        return;
    }

    let cols = (area.width / TILE_W).max(1) as usize;
    for (i, ws) in items.iter().enumerate() {
        let row = i / cols;
        let col = i % cols;
        let x = area.x + (col as u16 * TILE_W);
        let y = area.y + (row as u16 * TILE_H);
        let tile = Rect {
            x,
            y,
            width: TILE_W.min(area.width.saturating_sub(col as u16 * TILE_W)),
            height: TILE_H.min(area.height.saturating_sub(row as u16 * TILE_H)),
        };

        if tile.width < 6 || tile.height < 4 {
            continue;
        }

        let is_selected = i == selected;
        let mut style = Style::default();
        if is_selected {
            style = style
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD);
        } else if matches!(
            ws.attention,
            AttentionLevel::NeedsInput | AttentionLevel::Error
        ) && flash_on
        {
            style = style.fg(Color::Yellow).add_modifier(Modifier::BOLD);
        }

        let body = format!(
            "{}\n{}\nbranch: {}\ndirty: {}",
            ws.name,
            ws.path,
            ws.branch.as_deref().unwrap_or("-"),
            ws.dirty_files
        );
        frame.render_widget(
            Paragraph::new(body)
                .style(style)
                .block(Block::default().borders(Borders::ALL)),
            tile,
        );
    }
}

pub fn index_at(area: Rect, x: u16, y: u16, item_count: usize) -> Option<usize> {
    if item_count == 0 {
        return None;
    }
    if x < area.x || y < area.y || x >= area.right() || y >= area.bottom() {
        return None;
    }
    let rel_x = x - area.x;
    let rel_y = y - area.y;
    let cols = (area.width / TILE_W).max(1) as usize;
    let col = (rel_x / TILE_W) as usize;
    let row = (rel_y / TILE_H) as usize;
    let idx = row * cols + col;
    if idx < item_count {
        Some(idx)
    } else {
        None
    }
}
