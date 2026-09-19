use crate::core::QueryError;

#[test]
fn sanitized_redacts_bearer_equals_with_surrounding_whitespace() {
    let clean = QueryError::response("auth failed: bearer = s3cr3tval").sanitized();
    assert!(!clean.message().contains("s3cr3tval"));
    assert!(clean.message().contains("[REDACTED_TOKEN]"));
}

#[test]
fn sanitized_redacts_bearer_colon_mixed_case_with_whitespace() {
    let clean = QueryError::response("BEARER : sekret9").sanitized();
    assert!(!clean.message().contains("sekret9"));
    assert!(clean.message().contains("[REDACTED_TOKEN]"));
}

#[test]
fn sanitized_redacts_bearer_tab_equals_tab_value() {
    let clean = QueryError::response("bearer\t=\tval42").sanitized();
    assert!(!clean.message().contains("val42"));
    assert!(clean.message().contains("[REDACTED_TOKEN]"));
}

#[test]
fn sanitized_redacts_token_equals_with_whitespace() {
    let clean = QueryError::response("request rejected: token = leak123").sanitized();
    assert!(!clean.message().contains("leak123"));
    assert!(clean.message().contains("[REDACTED_TOKEN]"));
}

#[test]
fn sanitized_redacts_token_colon_with_whitespace() {
    let clean = QueryError::response("Token : val456").sanitized();
    assert!(!clean.message().contains("val456"));
    assert!(clean.message().contains("[REDACTED_TOKEN]"));
}

#[test]
fn sanitized_redacts_bearer_token_after_multiple_spaces() {
    let clean = QueryError::response("Bearer  tokensecret").sanitized();
    assert!(!clean.message().contains("tokensecret"));
    assert!(clean.message().contains("[REDACTED_TOKEN]"));
}

#[test]
fn sanitized_redacts_token_after_bearer_colon() {
    let clean = QueryError::response("auth failed: bearer: abc123secret").sanitized();
    assert!(!clean.message().contains("abc123secret"));
    assert!(clean.message().contains("[REDACTED_TOKEN]"));
}

#[test]
fn sanitized_redacts_email_local_part_containing_underscore() {
    let clean = QueryError::response("login failed for alice_bob@example.com").sanitized();
    assert!(!clean.message().contains("alice"));
    assert!(clean.message().contains("[REDACTED_EMAIL]"));
}
