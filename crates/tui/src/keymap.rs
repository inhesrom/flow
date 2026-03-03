use crossterm::event::{KeyCode, KeyEvent};

pub fn is_quit(key: KeyEvent) -> bool {
    matches!(key.code, KeyCode::Char('q'))
}
