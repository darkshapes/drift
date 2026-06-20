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
    
    let script_key = "ati_plug";
    let base = std::env::temp_dir().join("drift-test");
    let resolved = script_discovery::resolve_entrypoint_to_spawn_cmd(&repo_path, script_key, &base);
    
    assert!(resolved.is_ok(), "should resolve script key");
    let cmd = resolved.unwrap();
    
    assert!(!cmd.contains("python"), "should not use python");
    assert!(!cmd.contains("-c"), "should not use -c flag");
    assert!(cmd.contains("ati_plug"), "should contain script key");
}

#[test]
fn test_resolve_entrypoint_to_spawn_cmd_includes_pythonpath() {
    let repo_path = temp_test_path("full_path");
    let base = std::env::temp_dir().join("drift-test");
    let resolved = script_discovery::resolve_entrypoint_to_spawn_cmd(&repo_path, "ati_plug", &base);
    
    assert!(resolved.is_ok());
    let cmd = resolved.unwrap();
    assert!(!cmd.contains("PYTHONPATH="), "should not set PYTHONPATH - uses script key directly");
}

#[test]
fn test_resolve_entrypoint_to_spawn_cmd_with_script_key() {
    let repo_path = temp_test_path("script_key_test");
    let script_key = "ati_plug";
    let base = std::env::temp_dir().join("drift-test");
    let resolved = script_discovery::resolve_entrypoint_to_spawn_cmd(&repo_path, script_key, &base);
    
    assert!(resolved.is_ok(), "should resolve script key");
    assert_eq!(resolved.unwrap(), "ati_plug");
}

#[test]
fn test_resolve_entrypoint_to_spawn_cmd_empty() {
    let repo_path = temp_test_path("empty");
    let entrypoint = "";
    let base = std::env::temp_dir().join("drift-test");
    let resolved = script_discovery::resolve_entrypoint_to_spawn_cmd(&repo_path, entrypoint, &base);
    
    assert!(resolved.is_err(), "should error on empty entrypoint");
}