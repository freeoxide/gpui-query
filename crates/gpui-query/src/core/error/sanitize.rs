//! Sanitization helpers for redacting sensitive data in error messages.
//!
//! Provides lightweight pattern-matching (no `regex` crate dependency) for
//! redacting connection strings, tokens, file paths, emails, and hex keys.

/// Maximum length for sanitized error messages (512 characters).
pub const SANITIZE_MAX_LEN: usize = 512;

/// Redact known sensitive patterns from a message string and truncate to
/// [`SANITIZE_MAX_LEN`].
pub(crate) fn sanitize_message(msg: &str) -> String {
    use std::borrow::Cow;

    // N6/N13: keep a Cow throughout. `replace_regex` returns `Cow<str>` and
    // short-circuits to `Cow::Borrowed` when the pattern cannot match, so a
    // clean message skips every allocation (the previous code unconditionally
    // wrapped each call in `Cow::Owned`, defeating the optimization).
    let mut out: Cow<str> = Cow::Borrowed(msg);

    // Redact database connection strings.
    out = replace_regex(out, r"(?i)(postgres|mysql|mongodb|redis)://\S+", "[REDACTED_CONNECTION]");
    // Redact bearer/token patterns.
    out = replace_regex(out, r"(?i)(bearer\s+|token[=:]\s*)\S+", "$1[REDACTED_TOKEN]");
    // Redact common filesystem paths.
    out = replace_regex(out, r"(?i)(/home/|/Users/|/etc/|/var/)\S+", "[REDACTED_PATH]");
    // Redact email addresses.
    out = replace_regex(
        out,
        r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b",
        "[REDACTED_EMAIL]",
    );
    // Redact long hex sequences (likely API keys or secrets).
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

/// Redact `pattern` with `replacement` across `input`.
///
/// Uses a simple approach without pulling in the `regex` crate: manual
/// scan-and-replace for each pattern. This keeps the dependency footprint
/// minimal for a utility that only runs on DevTools / logging paths.
///
/// N6: returns `Cow<str>` so clean messages skip the full-text copy entirely
/// (the input cow is returned unchanged when no match is possible). Each arm
/// first checks a cheap `contains` guard so the heavier scan only runs when
/// the pattern could plausibly match. Implemented on `Cow<'a, str>` (taking it
/// by value) so the borrowed variant keeps the lifetime of the original
/// message, allowing the chain of reassignments in `sanitize_message` to
/// compile.
fn replace_regex<'a>(
    mut input: std::borrow::Cow<'a, str>,
    pattern: &str,
    replacement: &str,
) -> std::borrow::Cow<'a, str> {
    // Operate on the current contents (borrowed or owned) via a single
    // `&str` view. When no redaction is needed, return `input` unchanged so
    // a borrowed input stays borrowed (N6/N13).
    let text: &str = &input;
    // Lightweight pattern matching without the regex crate.
    // We only handle the specific patterns used by `sanitize_message`.
    let owned = match pattern {
        // Database connection strings. N11: pre-compute the needles once.
        p if p.contains("postgres")
            || p.contains("mysql")
            || p.contains("mongodb")
            || p.contains("redis") =>
        {
            // N6: cheap guard — no scheme needle can match, so skip.
            if !contains_any_scheme(&text.to_ascii_lowercase()) {
                return input;
            }
            redact_url_schemes(text, &["postgres", "mysql", "mongodb", "redis"], replacement)
        }
        // Bearer/token patterns.
        p if p.contains("bearer") || p.contains("token") => {
            let lower = text.to_ascii_lowercase();
            if !lower.contains("bearer") && !lower.contains("token") {
                return input;
            }
            redact_tokens(text, replacement)
        }
        // Filesystem paths.
        p if p.contains("/home/")
            || p.contains("/Users/")
            || p.contains("/etc/")
            || p.contains("/var/") =>
        {
            let lower = text.to_ascii_lowercase();
            if !lower.contains("/home/")
                && !lower.contains("/users/")
                && !lower.contains("/etc/")
                && !lower.contains("/var/")
            {
                return input;
            }
            redact_paths(text, &["/home/", "/Users/", "/etc/", "/var/"], replacement)
        }
        // Email addresses.
        p if p.contains("@") && p.contains(".") => {
            if !text.contains('@') {
                return input;
            }
            redact_emails(text, replacement)
        }
        // Long hex sequences.
        p if p.contains("0-9a-f") => {
            if !has_long_hex_run(text) {
                return input;
            }
            redact_hex(text, replacement)
        }
        // N30: unknown pattern. Every pattern passed to `replace_regex`
        // must have a matching arm — surface a missing arm in debug builds
        // instead of silently no-oping.
        _ => {
            debug_assert!(false, "replace_regex: unrecognized pattern {pattern:?}");
            return input;
        }
    };
    // A redaction actually happened; install the owned result.
    input = std::borrow::Cow::Owned(owned);
    input
}

/// N6 guard: whether `lower` (already lowercased) contains any of the
/// recognized URL-scheme needles.
fn contains_any_scheme(lower: &str) -> bool {
    // N11: pre-computed const table of the literal "scheme://" needles so we
    // avoid re-allocating `format!("{scheme}://")` per scheme per loop
    // iteration in `redact_url_schemes`, and reuse it here for the cheap guard.
    const SCHEME_NEEDLES: [&str; 4] = ["postgres://", "mysql://", "mongodb://", "redis://"];
    SCHEME_NEEDLES.iter().any(|needle| lower.contains(needle))
}

/// N6 guard: whether `text` contains a run of 16+ hex digits.
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

/// Redact URL-like connection strings starting with any of `schemes`.
fn redact_url_schemes(text: &str, _schemes: &[&str], replacement: &str) -> String {
    // N11: pre-computed const table of the literal "scheme://" needles instead
    // of re-allocating `format!("{scheme}://")` per scheme per loop iteration.
    const SCHEME_NEEDLES: [&str; 4] = ["postgres://", "mysql://", "mongodb://", "redis://"];

    let lower = text.to_ascii_lowercase();
    let mut result = String::with_capacity(text.len());
    let mut offset = 0;
    loop {
        let mut earliest: Option<usize> = None;
        for needle in SCHEME_NEEDLES {
            if let Some(rel) = lower[offset..].find(needle) {
                let abs = offset + rel;
                match earliest {
                    None => earliest = Some(abs),
                    Some(ep) if abs < ep => earliest = Some(abs),
                    _ => {}
                }
            }
        }
        match earliest {
            Some(abs_pos) => {
                let end = text[abs_pos..]
                    .find(|c: char| c.is_whitespace())
                    .map_or(text.len(), |i| abs_pos + i);
                result.push_str(&text[offset..abs_pos]);
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

/// Redact bearer/token patterns.
fn redact_tokens(text: &str, replacement: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let lower: Vec<char> = chars.iter().map(|c| c.to_ascii_lowercase()).collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        if lower_matches_at(&lower, i, "bearer ") {
            for c in &chars[i..i + 7] {
                result.push(*c);
            }
            i += 7;
            while i < len && !chars[i].is_ascii_whitespace() {
                i += 1;
            }
            result.push_str(replacement);
            continue;
        }
        if lower_matches_at(&lower, i, "token=") || lower_matches_at(&lower, i, "token:") {
            for c in &chars[i..i + 6] {
                result.push(*c);
            }
            i += 6;
            while i < len && !chars[i].is_ascii_whitespace() {
                i += 1;
            }
            result.push_str(replacement);
            continue;
        }
        result.push(chars[i]);
        i += 1;
    }
    result
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

/// Redact filesystem paths starting with any of `prefixes`.
fn redact_paths(text: &str, prefixes: &[&str], replacement: &str) -> String {
    let lower = text.to_ascii_lowercase();
    let mut result = String::with_capacity(text.len());
    let mut offset = 0;
    loop {
        let mut earliest: Option<usize> = None;
        for prefix in prefixes {
            if let Some(rel) = lower[offset..].find(prefix) {
                let abs = offset + rel;
                match earliest {
                    None => earliest = Some(abs),
                    Some(ep) if abs < ep => earliest = Some(abs),
                    _ => {}
                }
            }
        }
        match earliest {
            Some(abs_pos) => {
                let end = text[abs_pos..]
                    .find(|c: char| c.is_whitespace())
                    .map_or(text.len(), |i| abs_pos + i);
                result.push_str(&text[offset..abs_pos]);
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

/// Redact email addresses (simple heuristic: word@word.word).
fn redact_emails(text: &str, replacement: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut i = 0;
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();

    while i < len {
        // Try to match an email starting at position i.
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
    if i >= len || !chars[i].is_alphanumeric() {
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
    if domain_end > start + 2 {
        // Walk backwards to find last dot in the matched portion.
        let mut last_dot = None;
        for j in (start..domain_end).rev() {
            if chars[j] == '.' {
                last_dot = Some(j);
                break;
            }
        }
        if let Some(dot_pos) = last_dot {
            let tld_len = domain_end - dot_pos - 1;
            if tld_len >= 2 && chars[dot_pos + 1..domain_end].iter().all(|c| c.is_alphabetic()) {
                return Some(domain_end);
            }
        }
    }
    None
}

/// Redact long hex sequences (16+ hex chars).
fn redact_hex(text: &str, replacement: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut i = 0;
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();

    while i < len {
        if chars[i].is_ascii_hexdigit() {
            // Count consecutive hex chars.
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
