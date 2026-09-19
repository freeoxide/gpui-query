//! Redaction of sensitive patterns from error messages, without a `regex`
//! dependency. Covers connection strings, bearer tokens, file paths,
//! emails, and long hex runs.

/// Maximum length for sanitized error messages.
pub const SANITIZE_MAX_LEN: usize = 512;

/// Lowercase needles for recognized connection-string schemes.
const SCHEME_NEEDLES: [&str; 4] = ["postgres://", "mysql://", "mongodb://", "redis://"];

/// Lowercase needles for filesystem path prefixes, including macOS home dirs.
const PATH_NEEDLES: [&str; 4] = ["/home/", "/users/", "/etc/", "/var/"];

/// Redact known sensitive patterns from a message string and truncate to
/// [`SANITIZE_MAX_LEN`].
pub(crate) fn sanitize_message(msg: &str) -> String {
    use std::borrow::Cow;

    // Cow pipeline: a clean message stays borrowed through every rule and
    // never allocates until the final `into_owned`.
    let mut out: Cow<str> = Cow::Borrowed(msg);

    out = replace_regex(
        out,
        r"(?i)(postgres|mysql|mongodb|redis)://\S+",
        "[REDACTED_CONNECTION]",
    );
    out = replace_regex(
        out,
        r"(?i)(bearer\s+|token[=:]\s*)\S+",
        "$1[REDACTED_TOKEN]",
    );
    out = replace_regex(
        out,
        r"(?i)(/home/|/Users/|/etc/|/var/)\S+",
        "[REDACTED_PATH]",
    );
    out = replace_regex(
        out,
        r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b",
        "[REDACTED_EMAIL]",
    );
    out = replace_regex(out, r"\b[0-9a-fA-F]{16,}\b", "[REDACTED_HEX]");

    let mut s = out.into_owned();
    if s.len() > SANITIZE_MAX_LEN {
        let cut = s
            .char_indices()
            .map(|(b, _)| b)
            .rfind(|&b| b <= SANITIZE_MAX_LEN)
            .unwrap_or(0);
        s.truncate(cut);
        s.push_str("...[truncated]");
    }
    s
}

/// Apply one redaction rule. The `pattern` string only selects which rule
/// runs; each rule starts with a cheap `contains` guard so clean messages
/// skip the full scan, and returns the input unchanged (still borrowed)
/// when nothing can match.
fn replace_regex<'a>(
    input: std::borrow::Cow<'a, str>,
    pattern: &str,
    replacement: &str,
) -> std::borrow::Cow<'a, str> {
    let text: &str = &input;
    // ASCII lowercasing preserves byte offsets, so positions found in a
    // lowercased copy are valid slice indices into `text`.
    let owned = match pattern {
        p if p.contains("postgres") => {
            let lower = text.to_ascii_lowercase();
            if !SCHEME_NEEDLES.iter().any(|n| lower.contains(n)) {
                return input;
            }
            redact_until_whitespace(text, &lower, &SCHEME_NEEDLES, replacement)
        }
        p if p.contains("bearer") || p.contains("token") => {
            let lower = text.to_ascii_lowercase();
            if !lower.contains("bearer") && !lower.contains("token") {
                return input;
            }
            redact_tokens(text, replacement)
        }
        p if p.contains("/home/") => {
            let lower = text.to_ascii_lowercase();
            if !PATH_NEEDLES.iter().any(|n| lower.contains(n)) {
                return input;
            }
            redact_until_whitespace(text, &lower, &PATH_NEEDLES, replacement)
        }
        p if p.contains("@") && p.contains(".") => {
            if !text.contains('@') {
                return input;
            }
            redact_emails(text, replacement)
        }
        p if p.contains("0-9a-f") => {
            if !has_long_hex_run(text) {
                return input;
            }
            redact_hex(text, replacement)
        }
        _ => {
            debug_assert!(false, "replace_regex: unrecognized pattern {pattern:?}");
            return input;
        }
    };
    std::borrow::Cow::Owned(owned)
}

/// Whether `text` contains a run of 16+ hex digits.
fn has_long_hex_run(text: &str) -> bool {
    let mut run = 0usize;
    for c in text.chars() {
        if c.is_ascii_hexdigit() {
            run += 1;
            if run >= 16 {
                return true;
            }
        } else {
            run = 0;
        }
    }
    false
}

/// Redact every occurrence of any `needle` (located via its lowercase copy
/// `lower`) from the match start through the next whitespace character.
fn redact_until_whitespace(
    text: &str,
    lower: &str,
    needles: &[&str],
    replacement: &str,
) -> String {
    let mut result = String::with_capacity(text.len());
    let mut offset = 0;
    loop {
        let earliest = needles
            .iter()
            .filter_map(|n| lower[offset..].find(n).map(|rel| offset + rel))
            .min();
        match earliest {
            Some(start) => {
                let end = text[start..]
                    .find(char::is_whitespace)
                    .map_or(text.len(), |i| start + i);
                result.push_str(&text[offset..start]);
                result.push_str(replacement);
                offset = end;
                if offset >= text.len() {
                    break;
                }
            }
            None => {
                result.push_str(&text[offset..]);
                break;
            }
        }
    }
    result
}

/// Redact bearer/token patterns. ASCII whitespace after the keyword (or
/// after the `=`/`:` separator) is tolerated, per `bearer\s+|token[=:]\s*`.
fn redact_tokens(text: &str, replacement: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let lower: Vec<char> = chars.iter().map(|c| c.to_ascii_lowercase()).collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        if lower_matches_at(&lower, i, "bearer")
            && i + 6 < len
            && chars[i + 6].is_ascii_whitespace()
        {
            // Keep "bearer" plus its first whitespace char, then swallow any
            // extra whitespace so "bearer<TAB>x" redacts like "bearer x".
            for c in &chars[i..i + 7] {
                result.push(*c);
            }
            i += 7;
            skip_whitespace_and_token(&chars, &mut i, &mut result);
            result.push_str(replacement);
            continue;
        }
        if lower_matches_at(&lower, i, "token=") || lower_matches_at(&lower, i, "token:") {
            for c in &chars[i..i + 6] {
                result.push(*c);
            }
            i += 6;
            skip_whitespace_and_token(&chars, &mut i, &mut result);
            result.push_str(replacement);
            continue;
        }
        result.push(chars[i]);
        i += 1;
    }
    result
}

/// Copy the ASCII-whitespace run into `result`, then skip past the
/// non-whitespace token that follows (dropped from the output).
fn skip_whitespace_and_token(chars: &[char], i: &mut usize, result: &mut String) {
    let len = chars.len();
    while *i < len && chars[*i].is_ascii_whitespace() {
        result.push(chars[*i]);
        *i += 1;
    }
    while *i < len && !chars[*i].is_ascii_whitespace() {
        *i += 1;
    }
}

/// Check whether `lower` contains the ASCII `pat` (already-lowercased) at index `i`.
fn lower_matches_at(lower: &[char], i: usize, pat: &str) -> bool {
    let pb = pat.as_bytes();
    if i + pb.len() > lower.len() {
        return false;
    }
    for (k, &b) in pb.iter().enumerate() {
        if lower[i + k] != b as char {
            return false;
        }
    }
    true
}

/// Redact email addresses (simple heuristic: word@word.tld).
fn redact_emails(text: &str, replacement: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        if let Some(email_end) = try_match_email(&chars, i) {
            result.push_str(replacement);
            i = email_end;
            continue;
        }
        result.push(chars[i]);
        i += 1;
    }
    result
}

/// Try to match an email at position `start` in `chars`. Returns end index if matched.
fn try_match_email(chars: &[char], start: usize) -> Option<usize> {
    let len = chars.len();
    if start >= len {
        return None;
    }

    // Local part: alphanumeric + ._%+-
    let mut i = start;
    if !chars[i].is_alphanumeric() {
        return None;
    }
    while i < len && (chars[i].is_alphanumeric() || ".%+-".contains(chars[i])) {
        i += 1;
    }
    if i >= len || chars[i] != '@' {
        return None;
    }
    i += 1; // skip '@'

    // Domain: alphanumeric + .-
    if i >= len || !chars[i].is_alphanumeric() {
        return None;
    }
    while i < len && (chars[i].is_alphanumeric() || ".-".contains(chars[i])) {
        i += 1;
    }

    // Must end with a dot followed by 2+ alpha chars (TLD).
    let domain_end = i;
    if domain_end <= start + 2 {
        return None;
    }
    let dot_pos = (start..domain_end).rev().find(|&j| chars[j] == '.')?;
    let tld_len = domain_end - dot_pos - 1;
    if tld_len >= 2 && chars[dot_pos + 1..domain_end].iter().all(|c| c.is_alphabetic()) {
        Some(domain_end)
    } else {
        None
    }
}

/// Redact long hex sequences (16+ hex chars).
fn redact_hex(text: &str, replacement: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        if chars[i].is_ascii_hexdigit() {
            let start = i;
            while i < len && chars[i].is_ascii_hexdigit() {
                i += 1;
            }
            if i - start >= 16 {
                result.push_str(replacement);
            } else {
                for c in &chars[start..i] {
                    result.push(*c);
                }
            }
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redact_tokens_handles_non_ascii_without_panic() {
        let out = redact_tokens("x café bearer secret", "");
        assert!(out.contains("café"));
        assert!(!out.contains("secret"));
    }

    #[test]
    fn redact_tokens_preserves_bearer_redaction_on_ascii() {
        let out = redact_tokens("auth failed: bearer abc123token", "[REDACTED_TOKEN]");
        assert!(!out.contains("abc123token"));
        assert!(out.contains("[REDACTED_TOKEN]"));
        assert!(out.contains("auth failed: bearer "));
    }

    #[test]
    fn redact_tokens_redacts_token_equals_with_non_ascii_prefix() {
        let out = redact_tokens("café token=leak", "[REDACTED_TOKEN]");
        assert!(out.contains("café "));
        assert!(out.contains("token="));
        assert!(!out.contains("leak"));
        assert!(out.contains("[REDACTED_TOKEN]"));
    }

    #[test]
    fn redact_tokens_tolerates_tab_after_bearer() {
        let out = redact_tokens("auth failed: bearer\tabc123", "[REDACTED_TOKEN]");
        assert!(!out.contains("abc123"));
        assert!(out.contains("bearer\t"));
        assert!(out.contains("[REDACTED_TOKEN]"));
    }

    #[test]
    fn redact_tokens_tolerates_space_after_equals() {
        let out = redact_tokens("token= abc123", "[REDACTED_TOKEN]");
        assert!(out.contains("token= "));
        assert!(!out.contains("abc123"));
        assert!(out.contains("[REDACTED_TOKEN]"));
    }

    #[test]
    fn redact_users_path_mixed_case() {
        let msg = "error in /Users/admin/.env leaked";
        let out = sanitize_message(msg);
        assert!(!out.contains("/Users/admin/.env"));
        assert!(out.contains("[REDACTED_PATH]"));
    }

    #[test]
    fn sanitize_message_truncates_multibyte_on_char_boundary() {
        let msg = "a".to_string() + &"é".repeat(300);
        let out = sanitize_message(&msg);
        let suffix = "...[truncated]";
        assert!(out.ends_with(suffix));
        let cut = out.len() - suffix.len();
        assert!(cut <= SANITIZE_MAX_LEN);
        assert!(out.is_char_boundary(cut));
    }

    #[test]
    fn sanitize_message_all_multibyte_truncates_validly() {
        let msg = "é".repeat(400);
        let out = sanitize_message(&msg);
        let suffix = "...[truncated]";
        assert!(out.ends_with(suffix));
        let cut = out.len() - suffix.len();
        assert!(cut <= SANITIZE_MAX_LEN);
        assert!(out.is_char_boundary(cut));
    }

    #[test]
    fn sanitize_message_ascii_truncation_unchanged() {
        let msg = "x".repeat(600);
        let out = sanitize_message(&msg);
        assert!(out.ends_with("...[truncated]"));
        assert!(out.len() <= SANITIZE_MAX_LEN + "...[truncated]".len());
    }
}
