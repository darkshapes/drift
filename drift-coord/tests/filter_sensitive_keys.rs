use drift_coord::env::filter_sensitive_keys;
use std::collections::HashMap;

#[test]
fn test_filter_sensitive_keys_basic() {
    let mut env_vars = HashMap::new();
    env_vars.insert("FOO".to_string(), "bar".to_string());
    env_vars.insert("BAZ".to_string(), "qux".to_string());
    
    let filtered = filter_sensitive_keys(env_vars);
    
    assert_eq!(filtered.len(), 2);
}

#[test]
fn test_filter_sensitive_keys_filters_api_keys() {
    let mut env_vars = HashMap::new();
    env_vars.insert("PUBLIC_VAR".to_string(), "safe".to_string());
    env_vars.insert("API_KEY".to_string(), "secret".to_string());
    env_vars.insert("OTHER_KEY".to_string(), "also_secret".to_string());
    
    let filtered = filter_sensitive_keys(env_vars);
    
    assert_eq!(filtered.len(), 1);
    assert!(filtered.contains_key("PUBLIC_VAR"));
    assert!(!filtered.contains_key("API_KEY"));
}

#[test]
fn test_filter_sensitive_keys_filters_secrets() {
    let mut env_vars = HashMap::new();
    env_vars.insert("SAFE_VAR".to_string(), "safe".to_string());
    env_vars.insert("MY_SECRET".to_string(), "hidden".to_string());
    env_vars.insert("SECRET".to_string(), "hidden2".to_string());
    
    let filtered = filter_sensitive_keys(env_vars);
    
    assert_eq!(filtered.len(), 1);
    assert!(!filtered.contains_key("MY_SECRET"));
    assert!(!filtered.contains_key("SECRET"));
}

#[test]
fn test_filter_sensitive_keys_filters_tokens() {
    let mut env_vars = HashMap::new();
    env_vars.insert("HOST".to_string(), "localhost".to_string());
    env_vars.insert("ACCESS_TOKEN".to_string(), "eyJhbGc".to_string());
    env_vars.insert("TOKEN".to_string(), "secret".to_string());
    
    let filtered = filter_sensitive_keys(env_vars);
    
    assert_eq!(filtered.len(), 1);
    assert!(!filtered.contains_key("ACCESS_TOKEN"));
    assert!(!filtered.contains_key("TOKEN"));
}

#[test]
fn test_filter_sensitive_keys_filters_passwords() {
    let mut env_vars = HashMap::new();
    env_vars.insert("DB_HOST".to_string(), "localhost".to_string());
    env_vars.insert("DB_PASSWORD".to_string(), "hunter2".to_string());
    env_vars.insert("PASSWORD".to_string(), "secret".to_string());
    env_vars.insert("MY_PASS".to_string(), "secret2".to_string());
    
    let filtered = filter_sensitive_keys(env_vars);
    
    assert_eq!(filtered.len(), 1);
    assert!(!filtered.contains_key("DB_PASSWORD"));
    assert!(!filtered.contains_key("PASSWORD"));
    assert!(!filtered.contains_key("MY_PASS"));
}

#[test]
fn test_filter_sensitive_keys_filters_auth() {
    let mut env_vars = HashMap::new();
    env_vars.insert("ENDPOINT".to_string(), "https://api.example.com".to_string());
    env_vars.insert("AUTH_KEY".to_string(), "secret".to_string());
    env_vars.insert("AUTH_TOKEN".to_string(), "secret2".to_string());
    env_vars.insert("BEARER_AUTH".to_string(), "secret3".to_string());
    
    let filtered = filter_sensitive_keys(env_vars);
    
    assert_eq!(filtered.len(), 1);
    assert!(!filtered.contains_key("AUTH_KEY"));
    assert!(!filtered.contains_key("AUTH_TOKEN"));
    assert!(!filtered.contains_key("BEARER_AUTH"));
}

#[test]
fn test_filter_sensitive_keys_empty() {
    let env_vars = HashMap::new();
    let filtered = filter_sensitive_keys(env_vars);
    assert_eq!(filtered.len(), 0);
}

#[test]
fn test_filter_sensitive_keys_preserves_original() {
    let mut env_vars = HashMap::new();
    env_vars.insert("API_KEY".to_string(), "secret".to_string());
    env_vars.insert("SAFE".to_string(), "value".to_string());
    
    let filtered = filter_sensitive_keys(env_vars.clone());
    
    assert_eq!(filtered.len(), 1);
    assert_eq!(env_vars.len(), 2);
}
