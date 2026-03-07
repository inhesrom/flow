use protocol::AttentionLevel;

const MAX_BUFFER: usize = 2048;
const TAIL_WINDOW: usize = 512;

const PATTERNS: &[&str] = &[
    "this command requires approval",
    "yes, and don't ask again",
    "yes, and dont ask again",
    "esc to cancel",
    "tab to amend",
    "[y/n]",
    "(y/n)",
    "waiting for input",
    "waiting for your input",
    "requires your input",
    "do you want to proceed?",
    "allow once",
    "allow always",
    "press enter to continue",
    "press return to continue",
    "approve command",
];

pub struct AttentionDetector {
    buffer: String,
}

impl AttentionDetector {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
        }
    }

    /// Normalize and append PTY output to the rolling buffer.
    /// Returns `true` if any non-empty content was appended.
    pub fn append(&mut self, bytes: &[u8]) -> bool {
        if bytes.is_empty() {
            return false;
        }
        let chunk = String::from_utf8_lossy(bytes);
        let cleaned = normalize_for_match(&chunk);
        if cleaned.is_empty() {
            return false;
        }
        if !self.buffer.is_empty() {
            self.buffer.push(' ');
        }
        self.buffer.push_str(&cleaned);
        if self.buffer.len() > MAX_BUFFER {
            trim_to_last_bytes_at_char_boundary(&mut self.buffer, MAX_BUFFER);
        }
        true
    }

    /// Check the last [`TAIL_WINDOW`] bytes of the buffer for prompt patterns.
    /// If a pattern matches, the buffer is cleared and `true` is returned.
    pub fn check_for_prompt(&mut self) -> bool {
        let tail = tail_str(&self.buffer, TAIL_WINDOW);
        if PATTERNS.iter().any(|p| tail.contains(p)) {
            self.buffer.clear();
            return true;
        }
        // Fallback: agent asked a question and output has settled
        if has_sentence_ending_question(tail) {
            self.buffer.clear();
            return true;
        }
        false
    }

    /// Clear internal state (called when attention is externally cleared).
    pub fn reset(&mut self) {
        self.buffer.clear();
    }
}

pub fn needs_flash(level: AttentionLevel) -> bool {
    matches!(level, AttentionLevel::NeedsInput | AttentionLevel::Error)
}

fn tail_str(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut start = s.len() - max_bytes;
    while start < s.len() && !s.is_char_boundary(start) {
        start += 1;
    }
    &s[start..]
}

/// Returns `true` if `c` is a Unicode character used for terminal UI decoration
/// (box drawing, block elements, dingbats, etc.) that should be treated as
/// whitespace during prompt matching normalization.
fn is_terminal_decoration(c: char) -> bool {
    matches!(c,
        '\u{2500}'..='\u{257F}' |  // Box Drawing (─│┃┌┐└┘├┤┬┴┼ etc.)
        '\u{2580}'..='\u{259F}' |  // Block Elements (▀▄█▌▐▛▜▝ etc.)
        '\u{2300}'..='\u{23FF}' |  // Miscellaneous Technical (⏺⌘ etc.)
        '\u{2700}'..='\u{27BF}'    // Dingbats (❯✓✗ etc.)
    )
}

fn normalize_for_match(input: &str) -> String {
    let no_ansi = strip_ansi(input);
    no_ansi
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_control() || is_terminal_decoration(c) { ' ' } else { c })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Returns `true` if `s` contains a `?` immediately preceded by an alphanumeric
/// character, indicating a genuine sentence-ending question rather than a
/// standalone `?` used as a shortcut hint (e.g. "? for shortcuts").
fn has_sentence_ending_question(s: &str) -> bool {
    let mut prev_alnum = false;
    for c in s.chars() {
        if c == '?' && prev_alnum {
            return true;
        }
        prev_alnum = c.is_alphanumeric();
    }
    false
}

fn strip_ansi(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' {
            match chars.peek().copied() {
                Some('[') => {
                    // CSI: ESC [ ... (terminator in @..~)
                    let _ = chars.next();
                    for c in chars.by_ref() {
                        if ('@'..='~').contains(&c) {
                            break;
                        }
                    }
                }
                Some(']') | Some('P') | Some('X') | Some('^') | Some('_') => {
                    // OSC / DCS / SOS / PM / APC: consume until BEL or ST (ESC \)
                    let _ = chars.next();
                    for c in chars.by_ref() {
                        if c == '\x07' {
                            break;
                        }
                        if c == '\u{1b}' {
                            if chars.peek() == Some(&'\\') {
                                let _ = chars.next();
                            }
                            break;
                        }
                    }
                }
                Some('(' | ')' | '*' | '+') => {
                    // SCS (Select Character Set): ESC <designator> <charset-char>
                    let _ = chars.next(); // consume designator
                    let _ = chars.next(); // consume charset character
                }
                _ => {
                    let _ = chars.next();
                }
            }
            continue;
        }
        out.push(ch);
    }
    out
}

fn trim_to_last_bytes_at_char_boundary(s: &mut String, max_bytes: usize) {
    if s.len() <= max_bytes {
        return;
    }
    let mut start = s.len().saturating_sub(max_bytes);
    while start < s.len() && !s.is_char_boundary(start) {
        start += 1;
    }
    if start >= s.len() {
        s.clear();
        return;
    }
    s.drain(..start);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appends_and_normalizes() {
        let mut det = AttentionDetector::new();
        assert!(det.append(b"Hello  World\n"));
        assert_eq!(det.buffer, "hello world");
    }

    #[test]
    fn append_empty_returns_false() {
        let mut det = AttentionDetector::new();
        assert!(!det.append(b""));
    }

    #[test]
    fn append_strips_ansi() {
        let mut det = AttentionDetector::new();
        det.append(b"\x1b[32mgreen\x1b[0m text");
        assert_eq!(det.buffer, "green text");
    }

    #[test]
    fn check_for_prompt_matches_yn() {
        let mut det = AttentionDetector::new();
        det.append(b"Proceed? [y/n]");
        assert!(det.check_for_prompt());
        // Buffer should be cleared after match
        assert!(det.buffer.is_empty());
    }

    #[test]
    fn check_for_prompt_matches_approval() {
        let mut det = AttentionDetector::new();
        det.append(b"This command requires approval");
        assert!(det.check_for_prompt());
    }

    #[test]
    fn check_for_prompt_no_false_positive_confirm() {
        let mut det = AttentionDetector::new();
        det.append(b"I can confirm that the code is working");
        assert!(!det.check_for_prompt());
    }

    #[test]
    fn check_for_prompt_no_false_positive_continue() {
        let mut det = AttentionDetector::new();
        det.append(b"Let me continue with the implementation");
        assert!(!det.check_for_prompt());
    }

    #[test]
    fn check_for_prompt_matches_allow_once() {
        let mut det = AttentionDetector::new();
        det.append(b"Allow once  Allow always");
        assert!(det.check_for_prompt());
    }

    #[test]
    fn check_for_prompt_matches_do_you_want_to_proceed_with_question_mark() {
        let mut det = AttentionDetector::new();
        det.append(b"Do you want to proceed?");
        assert!(det.check_for_prompt());
    }

    #[test]
    fn check_for_prompt_no_match_do_you_want_to_proceed_without_question_mark() {
        let mut det = AttentionDetector::new();
        det.append(b"do you want to proceed with the task");
        assert!(!det.check_for_prompt());
    }

    #[test]
    fn check_for_prompt_approve_command() {
        let mut det = AttentionDetector::new();
        det.append(b"approve command");
        assert!(det.check_for_prompt());
    }

    #[test]
    fn only_checks_tail_window() {
        let mut det = AttentionDetector::new();
        // Push a pattern, then push enough filler to push it out of the tail window
        det.append(b"[y/n]");
        let filler = "x".repeat(TAIL_WINDOW + 100);
        det.append(filler.as_bytes());
        assert!(!det.check_for_prompt());
    }

    #[test]
    fn reset_clears_buffer() {
        let mut det = AttentionDetector::new();
        det.append(b"some text");
        det.reset();
        assert!(det.buffer.is_empty());
    }

    #[test]
    fn buffer_truncates_at_max() {
        let mut det = AttentionDetector::new();
        let big = "a".repeat(MAX_BUFFER + 500);
        det.append(big.as_bytes());
        assert!(det.buffer.len() <= MAX_BUFFER);
    }

    #[test]
    fn needs_flash_correct() {
        assert!(needs_flash(AttentionLevel::NeedsInput));
        assert!(needs_flash(AttentionLevel::Error));
        assert!(!needs_flash(AttentionLevel::None));
    }

    #[test]
    fn press_enter_to_continue_matches() {
        let mut det = AttentionDetector::new();
        det.append(b"Press Enter to continue");
        assert!(det.check_for_prompt());
    }

    #[test]
    fn generic_press_enter_no_match() {
        let mut det = AttentionDetector::new();
        det.append(b"press enter");
        assert!(!det.check_for_prompt());
    }

    #[test]
    fn trailing_question_mark_matches() {
        let mut det = AttentionDetector::new();
        det.append(b"Should I commit with this message?");
        assert!(det.check_for_prompt());
        assert!(det.buffer.is_empty());
    }

    #[test]
    fn no_question_mark_no_match() {
        let mut det = AttentionDetector::new();
        det.append(b"The answer is 42.");
        assert!(!det.check_for_prompt());
    }

    #[test]
    fn question_mark_mid_text_matches() {
        let mut det = AttentionDetector::new();
        det.append(b"What went wrong? Let me investigate");
        assert!(det.check_for_prompt());
    }

    #[test]
    fn question_mark_with_osc_suffix_matches() {
        let mut det = AttentionDetector::new();
        det.append(b"Should I commit?\x1b]133;D\x07");
        assert!(det.check_for_prompt());
    }

    #[test]
    fn strip_ansi_handles_osc_with_st() {
        let result = strip_ansi("hello\x1b]0;title\x1b\\world");
        assert_eq!(result, "helloworld");
    }

    #[test]
    fn strip_ansi_handles_dcs() {
        let result = strip_ansi("before\x1bPq#0;2;0;0;0\x1b\\after");
        assert_eq!(result, "beforeafter");
    }

    // --- Real-world ANSI output tests ---

    #[test]
    fn claude_code_permission_prompt_with_ansi() {
        let mut det = AttentionDetector::new();
        det.append(
            b"\x1b[1;33mThis command requires approval\x1b[0m\n\x1b[36mAllow once\x1b[0m",
        );
        assert!(det.check_for_prompt());
    }

    #[test]
    fn claude_code_yes_no_with_ansi() {
        let mut det = AttentionDetector::new();
        det.append(b"\x1b[1mProceed?\x1b[0m \x1b[2m(y/n)\x1b[0m");
        assert!(det.check_for_prompt());
    }

    #[test]
    fn claude_code_tab_to_amend() {
        let mut det = AttentionDetector::new();
        // Box-drawing chars + ANSI colour
        det.append(b"\xe2\x94\x8c\xe2\x94\x80\xe2\x94\x80 \x1b[34mTab to amend\x1b[0m");
        assert!(det.check_for_prompt());
    }

    #[test]
    fn no_false_positive_ansi_normal_output() {
        let mut det = AttentionDetector::new();
        det.append(b"\x1b[32m\xe2\x9c\x93 All tests passed\x1b[0m\n\x1b[32mBuild successful\x1b[0m");
        assert!(!det.check_for_prompt());
    }

    #[test]
    fn approve_command_with_osc_suffix() {
        let mut det = AttentionDetector::new();
        det.append(b"Approve command\x1b]133;D\x07\x1b]133;A\x07");
        assert!(det.check_for_prompt());
    }

    #[test]
    fn question_in_ansi_colored_output() {
        let mut det = AttentionDetector::new();
        det.append(b"\x1b[1;37mShould I apply this change?\x1b[0m");
        assert!(det.check_for_prompt());
    }

    #[test]
    fn multiple_appends_then_settle() {
        let mut det = AttentionDetector::new();
        det.append(b"\x1b[33mThis command ");
        det.append(b"requires ");
        det.append(b"approval\x1b[0m");
        assert!(det.check_for_prompt());
    }

    #[test]
    fn waiting_for_input_with_osc_title() {
        let mut det = AttentionDetector::new();
        det.append(b"\x1b]0;claude-code\x07\x1b[1mWaiting for your input\x1b[0m");
        assert!(det.check_for_prompt());
    }

    #[test]
    fn strip_ansi_handles_scs_g0() {
        // ESC ( B — Select Character Set G0 to ASCII
        let result = strip_ansi("hello\x1b(Bworld");
        assert_eq!(result, "helloworld");
    }

    #[test]
    fn strip_ansi_handles_scs_g1() {
        // ESC ) 0 — Select Character Set G1 to DEC Special Graphics
        let result = strip_ansi("hello\x1b)0world");
        assert_eq!(result, "helloworld");
    }

    #[test]
    fn trailing_question_mark_after_scs_matches() {
        let mut det = AttentionDetector::new();
        det.append(b"What can I help you with today?\x1b(B");
        assert!(det.check_for_prompt());
    }

    #[test]
    fn trailing_question_mark_with_whitespace_matches() {
        let mut det = AttentionDetector::new();
        det.append(b"Should I proceed?  \t\n");
        assert!(det.check_for_prompt());
    }

    #[test]
    fn claude_code_idle_prompt_detected() {
        let mut det = AttentionDetector::new();
        // Simulate: question text + box-drawing border + status bar
        let mut buf = Vec::new();
        buf.extend_from_slice(b"what can i help you with today? ");
        // 155 box-drawing chars (─ = 0xe2 0x94 0x80, 3 bytes each)
        for _ in 0..155 {
            buf.extend_from_slice("\u{2500}".as_bytes());
        }
        buf.extend_from_slice(b" ? for shortcuts | Update available! Run: brew upgrade claude-code");
        det.append(&buf);
        assert!(det.check_for_prompt());
    }

    #[test]
    fn decoration_only_returns_no_content() {
        let mut det = AttentionDetector::new();
        // Pure box-drawing output should normalize to empty → has_content = false
        let mut buf = Vec::new();
        for _ in 0..50 {
            buf.extend_from_slice("\u{2500}".as_bytes());
        }
        let has_content = det.append(&buf);
        assert!(!has_content);
    }

    #[test]
    fn standalone_question_mark_no_match() {
        let mut det = AttentionDetector::new();
        det.append(b"? for shortcuts");
        assert!(!det.check_for_prompt());
    }
}
