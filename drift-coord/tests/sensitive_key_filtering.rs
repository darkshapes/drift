use drift_coord::env::filter_sensitive_keys;
use std::collections::HashMap;

#[test]
fn test_sensitive_keys_filtered() {
    let patterns = vec![
        "SECRET_KEY",
        "API_SECRET",
        "GITHUB_TOKEN",
        "AWS_SECRET",
        "PASSWORD",
        "AUTH_TOKEN",
    ];

    for pattern in patterns {
        let mut vars = HashMap::new();
        vars.insert(pattern.to_string(), "secret_value".to_string());
        vars.insert("SAFE_VAR".to_string(), "safe_value".to_string());

        let filtered = filter_sensitive_keys(vars);

        assert!(!filtered.contains_key(pattern), "expected {} to be filtered", pattern);
        assert!(filtered.contains_key("SAFE_VAR"));
    }
}

#[test]
fn test_sensitive_substrings_filtered_case_sensitive() {
    let mut vars = HashMap::new();
    vars.insert("SECRET_KEY".to_string(), "secret".to_string());
    vars.insert("safe_key".to_string(), "safe".to_string());

    let filtered = filter_sensitive_keys(vars);

    assert_eq!(filtered.len(), 1);
    assert!(filtered.contains_key("safe_key"));
    assert!(!filtered.contains_key("SECRET_KEY"));
}

#[test]
fn test_wildcard_pattern_matching() {
    let mut vars = HashMap::new();
    vars.insert("MY_API_KEY".to_string(), "secret".to_string());
    vars.insert("DATABASE_SECRET".to_string(), "secret".to_string());
    vars.insert("SESSION_TOKEN".to_string(), "secret".to_string());
    vars.insert("PUBLIC_VAR".to_string(), "safe".to_string());

    let filtered = filter_sensitive_keys(vars);

    assert_eq!(filtered.len(), 1);
    assert!(filtered.contains_key("PUBLIC_VAR"));
}

#[test]
fn test_sensitive_substrings_filtered() {
    let sensitive_substrings = vec![
        "KEY",
        "SECRET",
        "TOKEN",
        "PASSWORD",
        "PASS",
        "AUTH",
    ];

    for substr in sensitive_substrings {
        let mut vars = HashMap::new();
        let key = format!("SOME_{}", substr);
        vars.insert(key.clone(), "secret".to_string());

        let filtered = filter_sensitive_keys(vars);

        assert!(!filtered.contains_key(&key), "expected {} to be filtered", key);
    }
}

#[test]
fn test_empty_map_after_filtering_all_sensitive() {
    let mut vars = HashMap::new();
    vars.insert("API_KEY_1".to_string(), "v1".to_string());
    vars.insert("API_KEY_2".to_string(), "v2".to_string());
    vars.insert("SECRET_TOKEN".to_string(), "v3".to_string());

    let filtered = filter_sensitive_keys(vars);

    assert!(filtered.is_empty());
}

#[test]
fn test_filter_preserves_non_sensitive_keys() {
    let mut vars = HashMap::new();
    vars.insert("HOST".to_string(), "localhost".to_string());
    vars.insert("PORT".to_string(), "8080".to_string());
    vars.insert("LOG_LEVEL".to_string(), "info".to_string());
    vars.insert("WORKERS".to_string(), "4".to_string());

    let filtered = filter_sensitive_keys(vars);

    assert_eq!(filtered.len(), 4);
    let preserved_keys = ["HOST", "PORT", "LOG_LEVEL", "WORKERS"];
    for k in preserved_keys {
        assert!(filtered.get(k).is_some(), "expected {} to be preserved", k);
    }
}
