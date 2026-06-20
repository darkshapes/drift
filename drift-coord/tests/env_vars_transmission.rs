use drift_coord::env::{parse_env_file, filter_sensitive_keys};
use drift_proto::TrainConfig;
use std::collections::HashMap;
use std::fs;

fn temp_env_path(name: &str) -> String {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("/tmp/drift-coord-test-{}-{}", ts, name)
}

#[test]
fn test_env_vars_parsed_and_filtered() {
    let env_path = temp_env_path("transmission");
    fs::write(&env_path, "FOO=bar\nBAZ=qux\nSECRET_KEY=abc123\n").unwrap();

    let vars = parse_env_file(&env_path).unwrap();
    let filtered = filter_sensitive_keys(vars);

    assert!(filtered.contains_key("FOO"));
    assert!(filtered.contains_key("BAZ"));
    assert!(!filtered.contains_key("SECRET_KEY"));
}

#[test]
fn test_env_vars_attached_to_train_config() {
    let mut config = TrainConfig::default();
    let mut env_vars = HashMap::new();
    env_vars.insert("FOO".to_string(), "bar".to_string());

    config.update_env_vars(env_vars);

    assert!(config.env_vars.is_some());
    let vars = config.env_vars.unwrap();
    assert_eq!(vars.get("FOO"), Some(&"bar".to_string()));
}

#[test]
fn test_env_vars_empty_after_filtering_sensitive() {
    let env_path = temp_env_path("all_sensitive");
    fs::write(&env_path, "API_SECRET=abc\nPASSWORD=pass\nTOKEN=xyz\n").unwrap();

    let vars = parse_env_file(&env_path).unwrap();
    let filtered = filter_sensitive_keys(vars);

    assert!(filtered.is_empty());
}

#[test]
fn test_env_vars_partial_filtering() {
    let env_path = temp_env_path("partial");
    fs::write(&env_path, "PUBLIC_VAR=hello\nGITHUB_TOKEN=secret\nSAFE_VAR=value\n").unwrap();

    let vars = parse_env_file(&env_path).unwrap();
    let filtered = filter_sensitive_keys(vars);

    assert_eq!(filtered.len(), 2);
    assert!(filtered.contains_key("PUBLIC_VAR"));
    assert!(filtered.contains_key("SAFE_VAR"));
    assert!(!filtered.contains_key("GITHUB_TOKEN"));
}

#[test]
fn test_env_vars_none_when_not_set() {
    let config = TrainConfig::default();
    assert!(config.env_vars.is_none());
}
