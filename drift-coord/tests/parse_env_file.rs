use drift_coord::env::parse_env_file;
use std::fs;

fn temp_env_path(name: &str) -> String {
    format!("/tmp/drift-coord-test-{}", name)
}

#[test]
fn test_parse_env_file_basic_key_value() {
    let env_path = temp_env_path("basic");
    fs::write(&env_path, "FOO=bar\nBAZ=qux\n").unwrap();
    
    let vars = parse_env_file(&env_path).unwrap();
    
    assert_eq!(vars.get("FOO"), Some(&"bar".to_string()));
    assert_eq!(vars.get("BAZ"), Some(&"qux".to_string()));
}

#[test]
fn test_parse_env_file_empty_lines() {
    let env_path = temp_env_path("empty_lines");
    fs::write(&env_path, "FOO=bar\n\n\nBAZ=qux\n\n").unwrap();
    
    let vars = parse_env_file(&env_path).unwrap();
    
    assert_eq!(vars.len(), 2);
}

#[test]
fn test_parse_env_file_comments() {
    let env_path = temp_env_path("comments");
    fs::write(&env_path, "# comment\nFOO=bar\n# another comment\nBAZ=qux\n").unwrap();
    
    let vars = parse_env_file(&env_path).unwrap();
    
    assert_eq!(vars.len(), 2);
    assert_eq!(vars.get("FOO"), Some(&"bar".to_string()));
}

#[test]
fn test_parse_env_file_missing_file() {
    let result = parse_env_file("/nonexistent/path/.env");
    assert!(result.is_err());
}

#[test]
fn test_parse_env_file_duplicate_keys_uses_last() {
    let env_path = temp_env_path("duplicates");
    fs::write(&env_path, "FOO=first\nFOO=second\n").unwrap();
    
    let vars = parse_env_file(&env_path).unwrap();
    
    assert_eq!(vars.get("FOO"), Some(&"second".to_string()));
}

#[test]
fn test_parse_env_file_empty_value() {
    let env_path = temp_env_path("empty_value");
    fs::write(&env_path, "FOO=\n").unwrap();
    
    let vars = parse_env_file(&env_path).unwrap();
    
    assert_eq!(vars.get("FOO"), Some(&"".to_string()));
}
