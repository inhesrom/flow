use protocol::AttentionLevel;

pub fn needs_flash(level: AttentionLevel) -> bool {
    matches!(level, AttentionLevel::NeedsInput | AttentionLevel::Error)
}
