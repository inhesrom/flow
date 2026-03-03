pub mod attention;
pub mod git;
pub mod terminal;

pub use protocol::{AttentionLevel, ChangedFile, GitState, TerminalKind};
pub use terminal::WorkspaceTerminals;
