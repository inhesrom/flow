use std::collections::HashMap;
use std::path::Path;

use protocol::{GitState, Route, TerminalKind, WorkspaceId, WorkspaceSummary};
use ratatui::{
    style::{Color as TuiColor, Modifier, Style},
    text::{Line, Span},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    HomeGrid,
    WsHeader,
    WsFiles,
    WsDiff,
    WsTerminal,
    WsTerminalTabs,
}

pub struct TuiApp {
    pub route: Route,
    pub focus: Focus,
    pub workspaces: Vec<WorkspaceSummary>,
    pub workspace_git: HashMap<WorkspaceId, GitState>,
    pub workspace_diff: HashMap<WorkspaceId, (String, String)>,
    pub terminal_state: HashMap<WorkspaceId, WorkspaceTerminalState>,
    pub last_resize_sent: HashMap<(WorkspaceId, TerminalKind), (u16, u16)>,
    pub home_selected: usize,
    pub ws_selected_file: usize,
    pub ws_diff_scroll: u16,
    pub ws_active_terminal: TerminalKind,
    pub flash_on: bool,
    pub add_workspace_path_input: Option<String>,
    pub pending_delete_workspace: Option<WorkspaceId>,
    pub rename_workspace_input: Option<String>,
}

impl Default for TuiApp {
    fn default() -> Self {
        Self {
            route: Route::Home,
            focus: Focus::HomeGrid,
            workspaces: Vec::new(),
            workspace_git: HashMap::new(),
            workspace_diff: HashMap::new(),
            terminal_state: HashMap::new(),
            last_resize_sent: HashMap::new(),
            home_selected: 0,
            ws_selected_file: 0,
            ws_diff_scroll: 0,
            ws_active_terminal: TerminalKind::Shell,
            flash_on: false,
            add_workspace_path_input: None,
            pending_delete_workspace: None,
            rename_workspace_input: None,
        }
    }
}

impl TuiApp {
    pub fn set_workspaces(&mut self, workspaces: Vec<WorkspaceSummary>) {
        self.workspaces = workspaces;
        if self.workspaces.is_empty() {
            self.home_selected = 0;
        } else if self.home_selected >= self.workspaces.len() {
            self.home_selected = self.workspaces.len() - 1;
        }
    }

    pub fn selected_workspace_id(&self) -> Option<WorkspaceId> {
        self.workspaces.get(self.home_selected).map(|w| w.id)
    }

    pub fn active_workspace_id(&self) -> Option<WorkspaceId> {
        match self.route {
            Route::Workspace { id } => Some(id),
            Route::Home => None,
        }
    }

    pub fn move_home_selection(&mut self, delta: isize) {
        if self.workspaces.is_empty() {
            self.home_selected = 0;
            return;
        }

        let len = self.workspaces.len() as isize;
        let next = (self.home_selected as isize + delta).clamp(0, len - 1);
        self.home_selected = next as usize;
    }

    pub fn set_home_selection(&mut self, index: usize) {
        if self.workspaces.is_empty() {
            self.home_selected = 0;
        } else {
            self.home_selected = index.min(self.workspaces.len() - 1);
        }
    }

    pub fn begin_add_workspace(&mut self, initial_path: String) {
        self.add_workspace_path_input = Some(initial_path);
    }

    pub fn cancel_add_workspace(&mut self) {
        self.add_workspace_path_input = None;
    }

    pub fn add_workspace_input_mut(&mut self) -> Option<&mut String> {
        self.add_workspace_path_input.as_mut()
    }

    pub fn is_adding_workspace(&self) -> bool {
        self.add_workspace_path_input.is_some()
    }

    pub fn begin_delete_workspace(&mut self) {
        self.pending_delete_workspace = self.selected_workspace_id();
    }

    pub fn cancel_delete_workspace(&mut self) {
        self.pending_delete_workspace = None;
    }

    pub fn is_confirming_delete(&self) -> bool {
        self.pending_delete_workspace.is_some()
    }

    pub fn take_delete_workspace(&mut self) -> Option<WorkspaceId> {
        self.pending_delete_workspace.take()
    }

    pub fn begin_rename_workspace(&mut self) {
        let Some(id) = self.active_workspace_id() else {
            return;
        };
        self.rename_workspace_input = self
            .workspaces
            .iter()
            .find(|w| w.id == id)
            .map(|w| w.name.clone());
    }

    pub fn cancel_rename_workspace(&mut self) {
        self.rename_workspace_input = None;
    }

    pub fn is_renaming_workspace(&self) -> bool {
        self.rename_workspace_input.is_some()
    }

    pub fn rename_input_mut(&mut self) -> Option<&mut String> {
        self.rename_workspace_input.as_mut()
    }

    pub fn take_rename_request(&mut self) -> Option<(WorkspaceId, String)> {
        let id = self.active_workspace_id()?;
        let name = self.rename_workspace_input.take()?.trim().to_string();
        if name.is_empty() {
            return None;
        }
        Some((id, name))
    }

    pub fn take_add_workspace_request(&mut self) -> Option<(String, String)> {
        let path = self.add_workspace_path_input.take()?;
        let trimmed = path.trim().to_string();
        if trimmed.is_empty() {
            return None;
        }
        let name = Path::new(&trimmed)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "workspace".to_string());
        Some((name, trimmed))
    }

    pub fn set_workspace_git(&mut self, id: WorkspaceId, git: GitState) {
        self.workspace_git.insert(id, git);
        self.clamp_selected_file();
    }

    pub fn set_workspace_diff(&mut self, id: WorkspaceId, file: String, diff: String) {
        self.workspace_diff.insert(id, (file, diff));
    }

    pub fn append_terminal_bytes(&mut self, id: WorkspaceId, kind: TerminalKind, bytes: &[u8]) {
        let state = self
            .terminal_state
            .entry(id)
            .or_insert_with(WorkspaceTerminalState::new);
        match kind {
            TerminalKind::Agent => state.agent.process(bytes),
            TerminalKind::Shell => state.shell.process(bytes),
        };
    }

    pub fn reset_terminal(&mut self, id: WorkspaceId, kind: TerminalKind) {
        let state = self
            .terminal_state
            .entry(id)
            .or_insert_with(WorkspaceTerminalState::new);
        match kind {
            TerminalKind::Agent => state.agent = make_parser(),
            TerminalKind::Shell => state.shell = make_parser(),
        };
    }

    pub fn resize_terminal_parser(
        &mut self,
        id: WorkspaceId,
        kind: TerminalKind,
        cols: u16,
        rows: u16,
    ) {
        let state = self
            .terminal_state
            .entry(id)
            .or_insert_with(WorkspaceTerminalState::new);
        let cols = cols.max(1);
        let rows = rows.max(1);
        match kind {
            TerminalKind::Agent => state.agent.set_size(rows, cols),
            TerminalKind::Shell => state.shell.set_size(rows, cols),
        }
    }

    pub fn should_send_resize(
        &mut self,
        id: WorkspaceId,
        kind: TerminalKind,
        cols: u16,
        rows: u16,
    ) -> bool {
        let key = (id, kind);
        let next = (cols.max(1), rows.max(1));
        if self.last_resize_sent.get(&key).copied() == Some(next) {
            return false;
        }
        self.last_resize_sent.insert(key, next);
        true
    }

    pub fn terminal_lines(&self, id: WorkspaceId, kind: TerminalKind) -> Vec<Line<'static>> {
        let Some(state) = self.terminal_state.get(&id) else {
            return vec![Line::from("No terminal output yet.")];
        };
        let screen = match kind {
            TerminalKind::Agent => state.agent.screen(),
            TerminalKind::Shell => state.shell.screen(),
        };
        let (cursor_row, cursor_col) = screen.cursor_position();
        let show_cursor = !screen.hide_cursor();
        let (rows, cols) = screen.size();
        let mut lines = Vec::with_capacity(rows as usize);
        for r in 0..rows {
            let mut spans = Vec::with_capacity(cols as usize);
            for c in 0..cols {
                let Some(cell) = screen.cell(r, c) else {
                    continue;
                };
                if cell.is_wide_continuation() {
                    continue;
                }
                let mut style = Style::default();
                let fg = map_color(cell.fgcolor());
                let bg = map_color(cell.bgcolor());
                style = style.fg(fg).bg(bg);
                if cell.bold() {
                    style = style.add_modifier(Modifier::BOLD);
                }
                if cell.italic() {
                    style = style.add_modifier(Modifier::ITALIC);
                }
                if cell.underline() {
                    style = style.add_modifier(Modifier::UNDERLINED);
                }
                if cell.inverse() {
                    style = style.fg(bg).bg(fg);
                }
                if show_cursor && r == cursor_row && c == cursor_col {
                    style = style.add_modifier(Modifier::REVERSED);
                }
                let text = if cell.has_contents() {
                    cell.contents()
                } else {
                    " ".to_string()
                };
                spans.push(Span::styled(text, style));
            }
            lines.push(Line::from(spans));
        }
        lines
    }

    pub fn move_workspace_file_selection(&mut self, delta: isize) {
        let Some(id) = self.active_workspace_id() else {
            return;
        };
        let Some(git) = self.workspace_git.get(&id) else {
            self.ws_selected_file = 0;
            return;
        };
        if git.changed.is_empty() {
            self.ws_selected_file = 0;
            return;
        }
        let len = git.changed.len() as isize;
        let next = (self.ws_selected_file as isize + delta).clamp(0, len - 1);
        self.ws_selected_file = next as usize;
    }

    pub fn selected_changed_file(&self) -> Option<String> {
        let id = self.active_workspace_id()?;
        let git = self.workspace_git.get(&id)?;
        git.changed
            .get(self.ws_selected_file)
            .map(|c| c.path.clone())
    }

    fn clamp_selected_file(&mut self) {
        let Some(id) = self.active_workspace_id() else {
            return;
        };
        if let Some(git) = self.workspace_git.get(&id) {
            if git.changed.is_empty() {
                self.ws_selected_file = 0;
            } else if self.ws_selected_file >= git.changed.len() {
                self.ws_selected_file = git.changed.len() - 1;
            }
        }
    }
}

pub struct WorkspaceTerminalState {
    pub agent: vt100::Parser,
    pub shell: vt100::Parser,
}

impl WorkspaceTerminalState {
    fn new() -> Self {
        Self {
            agent: make_parser(),
            shell: make_parser(),
        }
    }
}

fn make_parser() -> vt100::Parser {
    vt100::Parser::new(24, 120, 8000)
}

fn map_color(color: vt100::Color) -> TuiColor {
    match color {
        vt100::Color::Default => TuiColor::Reset,
        vt100::Color::Idx(i) => TuiColor::Indexed(i),
        vt100::Color::Rgb(r, g, b) => TuiColor::Rgb(r, g, b),
    }
}
