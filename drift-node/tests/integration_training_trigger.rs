use drift_node::script_discovery;
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

fn temp_test_path(suffix: &str) -> PathBuf {
    let ts = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join("drift-integration").join(format!("{}-{}", ts, suffix))
}

fn cleanup_test_dir(path: &PathBuf) {
    let _ = fs::remove_dir_all(path);
}

#[test]
fn integration_train_repo_url_without_entrypoint_fails() {
    let repo_path = temp_test_path("train_without_entrypoint");
    fs::create_dir_all(&repo_path).unwrap();

    fs::write(
        repo_path.join("pyproject.toml"),
        r#"[project]
name = "test-repo"
"#,
    )
    .unwrap();

    let entrypoint = script_discovery::discover_script_entrypoint(&repo_path);
    assert!(entrypoint.is_err(), "should fail when _ati_plug missing");

    cleanup_test_dir(&repo_path);
}

#[test]
fn integration_train_repo_url_with_valid_entrypoint_succeeds() {
    let repo_path = temp_test_path("train_with_entrypoint");
    fs::create_dir_all(&repo_path).unwrap();

    fs::write(
        repo_path.join("pyproject.toml"),
        r#"[project]
name = "test-repo"

[project.scripts]
ati_plug = "src.train:run"
"#,
    )
    .unwrap();

    let script_key = script_discovery::discover_script_entrypoint(&repo_path);
    assert!(script_key.is_ok(), "should find _ati_plug");
    assert_eq!(script_key.unwrap(), "ati_plug");

    cleanup_test_dir(&repo_path);
}

#[test]
fn integration_full_flow_with_venv_activation() {
    let repo_path = temp_test_path("full_flow_venv");
    fs::create_dir_all(&repo_path.join(".venv").join("bin")).unwrap();
    fs::write(repo_path.join(".venv").join("bin").join("activate"), "# venv activation\n").unwrap();

    fs::write(
        repo_path.join("pyproject.toml"),
        r#"[project]
name = "test-repo"

[project.scripts]
ati_plug = "src.train:run"
"#,
    )
    .unwrap();

    let script_key = script_discovery::discover_script_entrypoint(&repo_path).unwrap();
    assert_eq!(script_key, "ati_plug");

    let base = std::env::temp_dir().join("drift-test");
    let has_venv = script_discovery::detect_venv_activation(&repo_path, &base);
    assert!(has_venv.is_some(), "should detect venv");

    let base = std::env::temp_dir().join("drift-test");
    let spawn_cmd = script_discovery::resolve_entrypoint_to_spawn_cmd(&repo_path, &script_key, &base);
    assert!(spawn_cmd.is_ok(), "should resolve spawn cmd");

    let cmd_str = spawn_cmd.unwrap();
    assert!(cmd_str.contains("source"), "should include venv activation");
    assert!(cmd_str.contains(".venv/bin/activate"), "should reference venv activate");
    assert!(cmd_str.contains("ati_plug"), "should include script key");

    cleanup_test_dir(&repo_path);
}

#[test]
fn integration_full_flow_without_venv() {
    let repo_path = temp_test_path("full_flow_no_venv");
    fs::create_dir_all(&repo_path).unwrap();

    fs::write(
        repo_path.join("pyproject.toml"),
        r#"[project]
name = "test-repo"

[project.scripts]
ati_plug = "src.train:run"
"#,
    )
    .unwrap();

    let script_key = script_discovery::discover_script_entrypoint(&repo_path).unwrap();

    let base = std::env::temp_dir().join("drift-test");
    let has_venv = script_discovery::detect_venv_activation(&repo_path, &base);
    assert!(has_venv.is_none(), "should not detect venv");

    let base = std::env::temp_dir().join("drift-test");
    let spawn_cmd = script_discovery::resolve_entrypoint_to_spawn_cmd(&repo_path, &script_key, &base);
    assert!(spawn_cmd.is_ok(), "should resolve spawn cmd");

    let cmd_str = spawn_cmd.unwrap();
    assert!(!cmd_str.contains("source"), "should not include venv activation");
    assert_eq!(cmd_str, "ati_plug");

    cleanup_test_dir(&repo_path);
}

#[test]
fn integration_invalid_repo_trats_simulation_trigger() {
    let repo_path = temp_test_path("invalid_repo");
    fs::create_dir_all(&repo_path).unwrap();

    let base = std::env::temp_dir().join("drift-test");
    let has_venv = script_discovery::detect_venv_activation(&repo_path, &base);
    assert!(has_venv.is_none(), "no venv in invalid repo");

    let entrypoint = script_discovery::discover_script_entrypoint(&repo_path);
    assert!(entrypoint.is_err(), "no _ati_plug in invalid repo");

    cleanup_test_dir(&repo_path);
}

#[test]
fn integration_venv_activation_path_exists() {
    let repo_path = temp_test_path("venv_activation_path");
    fs::create_dir_all(&repo_path.join(".venv").join("bin")).unwrap();
    let activate_path = repo_path.join(".venv").join("bin").join("activate");
    fs::write(&activate_path, "").unwrap();

    let base = std::env::temp_dir().join("drift-test");
    let detected = script_discovery::detect_venv_activation(&repo_path, &base);
    assert!(detected.is_some(), "should find activate script");

    let detected_str = detected.unwrap();
    assert_eq!(detected_str, activate_path.display().to_string());

    cleanup_test_dir(&repo_path);
}

#[test]
fn integration_venv_activation_path_not_exists() {
    let repo_path = temp_test_path("no_venv_activation_path");
    fs::create_dir_all(&repo_path).unwrap();

    let base = std::env::temp_dir().join("drift-test");
    let detected = script_discovery::detect_venv_activation(&repo_path, &base);
    assert!(detected.is_none(), "should not find activate script");

    cleanup_test_dir(&repo_path);
}

#[test]
fn integration_spawn_cmd_with_venv_includes_activation() {
    let repo_path = temp_test_path("spawn_with_venv");
    fs::create_dir_all(&repo_path.join(".venv").join("bin")).unwrap();
    fs::write(repo_path.join(".venv").join("bin").join("activate"), "# activation\n").unwrap();

    let base = std::env::temp_dir().join("drift-test");
    let spawn_cmd = script_discovery::resolve_entrypoint_to_spawn_cmd(&repo_path, "ati_plug", &base);
    assert!(spawn_cmd.is_ok());

    let cmd = spawn_cmd.unwrap();
    assert!(cmd.starts_with("source"), "should start with source");
    assert!(cmd.contains("&&"), "should chain commands");
    assert!(cmd.contains("ati_plug"), "should include script key");

    cleanup_test_dir(&repo_path);
}

#[test]
fn integration_spawn_cmd_without_venv_simple_python() {
    let repo_path = temp_test_path("spawn_no_venv");
    fs::create_dir_all(&repo_path).unwrap();

    let base = std::env::temp_dir().join("drift-test");
    let spawn_cmd = script_discovery::resolve_entrypoint_to_spawn_cmd(&repo_path, "ati_plug", &base);
    assert!(spawn_cmd.is_ok());

    let cmd = spawn_cmd.unwrap();
    assert_eq!(cmd, "ati_plug");
    assert!(!cmd.starts_with("source"), "should not start with source");

    cleanup_test_dir(&repo_path);
}
