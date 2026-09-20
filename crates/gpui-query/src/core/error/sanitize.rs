//! Redaction of sensitive patterns from error messages, without a `regex`
//! dependency.

use std::borrow::Cow;

pub const SANITIZE_MAX_LEN: usize = 512;

const SCHEME_NEEDLES: [&str; 4] = ["postgres://", "mysql://", "mongodb://", "redis://"];

const PATH_NEEDLES: [&str; 4] = ["/home/", "/users/", "/etc/", "/var/"];

pub(crate) fn sanitize_message(msg: &str) -> String {
    let out = redact_connections(Cow::Borrowed(msg));
    let out = redact_tokens(out);
    let out = redact_paths(out);
    let out = redact_emails(out);
    let out = redact_hex_runs(out);

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

/// ASCII-case-insensitive `contains` without allocating a lowercased copy.
fn contains_ascii_ci(haystack: &str, needle: &str) -> bool {
    haystack
        .as_bytes()
        .windows(needle.len())
        .any(|w| w.eq_ignore_ascii_case(needle.as_bytes()))
}

fn redact_connections(input: Cow<'_, str>) -> Cow<'_, str> {
    if !SCHEME_NEEDLES.iter().any(|n| contains_ascii_ci(&input, n)) {
        return input;
    }
    // ASCII lowercasing preserves byte offsets, so `lower` indexes are valid in `input`.
    let lower = input.to_ascii_lowercase();
    redact_until_whitespace(&input, &lower, &SCHEME_NEEDLES, "[REDACTED_CONNECTION]").into()
}

fn redact_paths(input: Cow<'_, str>) -> Cow<'_, str> {
    if !PATH_NEEDLES.iter().any(|n| contains_ascii_ci(&input, n)) {
        return input;
    }
    let lower = input.to_ascii_lowercase();
    redact_until_whitespace(&input, &lower, &PATH_NEEDLES, "[REDACTED_PATH]").into()
}

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

fn redact_tokens(input: Cow<'_, str>) -> Cow<'_, str> {
    if !contains_ascii_ci(&input, "bearer") && !contains_ascii_ci(&input, "token") {
        return input;
    }
    let chars: Vec<char> = input.chars().collect();
    let lower: Vec<char> = chars.iter().map(|c| c.to_ascii_lowercase()).collect();
    let len = chars.len();
    let mut result = String::with_capacity(input.len());
    let mut i = 0;
    while i < len {
        match try_match_token(&chars, &lower, i) {
            Some((verbatim_end, resume)) => {
                for c in &chars[i..verbatim_end] {
                    result.push(*c);
                }
                result.push_str("[REDACTED_TOKEN]");
                i = resume;
            }
            None => {
                result.push(chars[i]);
                i += 1;
            }
        }
    }
    result.into()
}

/// Grammar `keyword [ws|:|=]* token`; `token` requires at least one `:`/`=` in the run while `bearer` accepts any, and the tuple is (verbatim prefix end, resume index past the redacted token).
fn try_match_token(chars: &[char], lower: &[char], i: usize) -> Option<(usize, usize)> {
    let (keyword_len, sep_required) = if lower_matches_at(lower, i, "bearer") {
        (6, false)
    } else if lower_matches_at(lower, i, "token") {
        (5, true)
    } else {
        return None;
    };
    let len = chars.len();
    let mut j = i + keyword_len;
    let mut saw_ws = false;
    let mut saw_sep = false;
    while j < len {
        let c = chars[j];
        if c.is_ascii_whitespace() {
            saw_ws = true;
        } else if c == ':' || c == '=' {
            saw_sep = true;
        } else {
            break;
        }
        j += 1;
    }
    if !saw_sep && (sep_required || !saw_ws) {
        return None;
    }

    let verbatim_end = j;
    while j < len && !chars[j].is_ascii_whitespace() {
        j += 1;
    }
    Some((verbatim_end, j))
}

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

fn redact_emails(input: Cow<'_, str>) -> Cow<'_, str> {
    if !input.contains('@') {
        return input;
    }
    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();
    let mut result = String::with_capacity(input.len());
    let mut i = 0;
    while i < len {
        if let Some(email_end) = try_match_email(&chars, i) {
            result.push_str("[REDACTED_EMAIL]");
            i = email_end;
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }
    result.into()
}

/// TLD contract: >= 2 chars, all-alphanumeric, letter-first or >= 2 letters (`c0m`/`c0`/`0rg` redact; `2x`, `1.2.10` pass); a trailing FQDN dot is trimmed for the slice but stays inside the redaction.
fn try_match_email(chars: &[char], start: usize) -> Option<usize> {
    let len = chars.len();
    if start >= len {
        return None;
    }

    let mut i = start;
    if !chars[i].is_alphanumeric() && chars[i] != '_' {
        return None;
    }
    while i < len && (chars[i].is_alphanumeric() || "_.%+-".contains(chars[i])) {
        i += 1;
    }
    if i >= len || chars[i] != '@' {
        return None;
    }
    i += 1;

    if i >= len || !chars[i].is_alphanumeric() {
        return None;
    }
    while i < len && (chars[i].is_alphanumeric() || ".-".contains(chars[i])) {
        i += 1;
    }

    let scan_end = i;
    while i > start && chars[i - 1] == '.' {
        i -= 1;
    }

    let domain_end = i;
    if domain_end <= start + 2 {
        return None;
    }
    let dot_pos = (start..domain_end).rev().find(|&j| chars[j] == '.')?;
    let tld = &chars[dot_pos + 1..domain_end];
    let letters = tld.iter().filter(|c| c.is_alphabetic()).count();
    if tld.len() >= 2
        && tld.iter().all(|c| c.is_alphanumeric())
        && (tld[0].is_alphabetic() || letters >= 2)
    {
        Some(scan_end)
    } else {
        None
    }
}

fn redact_hex_runs(input: Cow<'_, str>) -> Cow<'_, str> {
    if !has_long_hex_run(&input) {
        return input;
    }
    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();
    let mut result = String::with_capacity(input.len());
    let mut i = 0;
    while i < len {
        if chars[i].is_ascii_hexdigit() {
            let start = i;
            while i < len && chars[i].is_ascii_hexdigit() {
                i += 1;
            }
            if i - start >= 16 {
                result.push_str("[REDACTED_HEX]");
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
    result.into()
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redact_tokens_handles_non_ascii_without_panic() {
        let out = redact_tokens(Cow::Borrowed("x café bearer secret"));
        assert!(out.contains("café"));
        assert!(!out.contains("secret"));
        assert!(out.contains("[REDACTED_TOKEN]"));
    }

    #[test]
    fn redact_tokens_preserves_bearer_redaction_on_ascii() {
        let out = redact_tokens(Cow::Borrowed("auth failed: bearer abc123token"));
        assert!(!out.contains("abc123token"));
        assert!(out.contains("[REDACTED_TOKEN]"));
        assert!(out.contains("auth failed: bearer "));
    }

    #[test]
    fn redact_tokens_redacts_token_equals_with_non_ascii_prefix() {
        let out = redact_tokens(Cow::Borrowed("café token=leak"));
        assert!(out.contains("café "));
        assert!(out.contains("token="));
        assert!(!out.contains("leak"));
        assert!(out.contains("[REDACTED_TOKEN]"));
    }

    #[test]
    fn redact_tokens_tolerates_tab_after_bearer() {
        let out = redact_tokens(Cow::Borrowed("auth failed: bearer\tabc123"));
        assert!(!out.contains("abc123"));
        assert!(out.contains("bearer\t"));
        assert!(out.contains("[REDACTED_TOKEN]"));
    }

    #[test]
    fn redact_tokens_tolerates_space_after_equals() {
        let out = redact_tokens(Cow::Borrowed("token= abc123"));
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
