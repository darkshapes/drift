use drift_node::script_discovery;
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

fn temp_test_path(suffix: &str) -> PathBuf {
    let ts = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join("drift-test-entrypoint").join(format!("{}-{}", ts, suffix))
}

#[test]
fn test_resolve_entrypoint_to_spawn_cmd_simple_module() {
    let repo_path = temp_test_path("simple_module");
    fs::create_dir_all(repo_path.join("src")).unwrap();
    
    let entrypoint = "src.main:ati";
    let base = std::env::temp_dir().join("drift-test");
    let resolved = script_discovery::resolve_entrypoint_to_spawn_cmd(&repo_path, entrypoint, &base);
    
    assert!(resolved.is_ok(), "should resolve entrypoint");
    let cmd = resolved.unwrap();
    
    assert!(cmd.contains("python"), "should use python");
    assert!(cmd.contains("-c"), "should use -c flag");
    assert!(cmd.contains("from src.main import ati"), "should import module");
}

#[test]
fn test_resolve_entrypoint_to_spawn_cmd_includes_pythonpath() {
    let repo_path = temp_test_path("full_path");
    let base = std::env::temp_dir().join("drift-test");
    let resolved = script_discovery::resolve_entrypoint_to_spawn_cmd(&repo_path, "src.main:ati", &base);
    
    assert!(resolved.is_ok());
    let cmd = resolved.unwrap();
    assert!(cmd.contains("PYTHONPATH="), "should set PYTHONPATH");
}

#[test]
fn test_resolve_entrypoint_to_spawn_cmd_invalid_format() {
    let repo_path = temp_test_path("invalid_format");
    let entrypoint = "not-valid";
    let base = std::env::temp_dir().join("drift-test");
    let resolved = script_discovery::resolve_entrypoint_to_spawn_cmd(&repo_path, entrypoint, &base);
    
    assert!(resolved.is_err(), "should error on invalid format");
}

#[test]
fn test_resolve_entrypoint_to_spawn_cmd_empty() {
    let repo_path = temp_test_path("empty");
    let entrypoint = "";
    let base = std::env::temp_dir().join("drift-test");
    let resolved = script_discovery::resolve_entrypoint_to_spawn_cmd(&repo_path, entrypoint, &base);
    
    assert!(resolved.is_err(), "should error on empty entrypoint");
}