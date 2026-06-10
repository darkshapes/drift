use drift_node::script_discovery;
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

fn temp_test_path(suffix: &str) -> PathBuf {
    let ts = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_nanos();
    std::env::temp_dir().join("drift-test-fail").join(format!("{}-{}", ts, suffix))
}

fn cleanup_test_dir(path: &PathBuf) {
    let _ = fs::remove_dir_all(path);
}

#[test]
fn test_discover_script_entrypoint_missing_ati_plug() {
    let repo_path = temp_test_path("missing_ati_plug");
    fs::create_dir_all(&repo_path).unwrap();

    fs::write(
        repo_path.join("pyproject.toml"),
        r#"[project]
name = "my-training"

[project.scripts]
some_other_script = "src.main:other"
"#,
    )
    .unwrap();

    let result = script_discovery::discover_script_entrypoint(&repo_path);
    assert!(result.is_err(), "should error when ati_plug not found");

    cleanup_test_dir(&repo_path);
}

#[test]
fn test_discover_script_entrypoint_missing_pyproject_toml() {
    let repo_path = temp_test_path("missing_pyproject");

    let result = script_discovery::discover_script_entrypoint(&repo_path);
    assert!(result.is_err(), "should error when pyproject.toml missing");
}

#[test]
fn test_discover_script_entrypoint_invalid_toml() {
    let repo_path = temp_test_path("invalid_toml");
    fs::create_dir_all(&repo_path).unwrap();

    fs::write(
        repo_path.join("pyproject.toml"),
        "not valid toml {{{{",
    )
    .unwrap();

    let result = script_discovery::discover_script_entrypoint(&repo_path);
    assert!(result.is_err(), "should error on invalid TOML");

    cleanup_test_dir(&repo_path);
}