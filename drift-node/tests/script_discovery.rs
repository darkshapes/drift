use drift_node::script_discovery;
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

fn temp_test_path(suffix: &str) -> PathBuf {
    let ts = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join("drift-test").join(format!("{}-{}", ts, suffix))
}

fn cleanup_test_dir(path: &PathBuf) {
    let _ = fs::remove_dir_all(path);
}

#[test]
fn test_discover_script_entrypoint_happy_path_project_scripts() {
    let repo_path = temp_test_path("project_scripts");
    fs::create_dir_all(&repo_path).unwrap();

    fs::write(
        repo_path.join("pyproject.toml"),
        r#"[project]
name = "my-training"

[project.scripts]
ati_plug = "src.main:ati"
"#,
    )
    .unwrap();

    let result = script_discovery::discover_script_entrypoint(&repo_path);
    assert!(result.is_ok(), "should parse [project.scripts]");
    let script_key = result.unwrap();
    assert_eq!(script_key, "ati_plug");

    cleanup_test_dir(&repo_path);
}

#[test]
fn test_discover_script_entrypoint_happy_path_uv_scripts() {
    let repo_path = temp_test_path("uv_scripts");
    fs::create_dir_all(&repo_path).unwrap();

    fs::write(
        repo_path.join("pyproject.toml"),
        r#"[project]
name = "my-training"

[tool.uv.scripts]
ati_plug = "src.main:ati"
"#,
    )
    .unwrap();

    let result = script_discovery::discover_script_entrypoint(&repo_path);
    assert!(result.is_ok(), "should parse [tool.uv.scripts]");
    let script_key = result.unwrap();
    assert_eq!(script_key, "ati_plug");

    cleanup_test_dir(&repo_path);
}

#[test]
fn test_discover_script_entrypoint_prefers_project_over_uv() {
    let repo_path = temp_test_path("prefers_project");
    fs::create_dir_all(&repo_path).unwrap();

    fs::write(
        repo_path.join("pyproject.toml"),
        r#"[project]
name = "my-training"

[project.scripts]
ati_plug = "src.main:project_ati"

[tool.uv.scripts]
ati_plug = "src.main:uv_ati"
"#,
    )
    .unwrap();

    let result = script_discovery::discover_script_entrypoint(&repo_path);
    assert!(result.is_ok());
    let script_key = result.unwrap();
    assert_eq!(script_key, "ati_plug");

    cleanup_test_dir(&repo_path);
}

#[test]
fn test_repo_path_from_url_github() {
    let url = "https://github.com/user/my-repo";
    let suffix = script_discovery::repo_suffix_from_url(url);
    assert_eq!(suffix, "my-repo");
}

#[test]
fn test_repo_path_from_url_with_git_extension() {
    let url = "https://github.com/user/my-repo.git";
    let suffix = script_discovery::repo_suffix_from_url(url);
    assert_eq!(suffix, "my-repo");
}

#[test]
fn test_detect_venv_activation_with_venv() {
    let repo_path = temp_test_path("has_venv");
    fs::create_dir_all(&repo_path.join(".venv").join("bin")).unwrap();
    fs::write(repo_path.join(".venv").join("bin").join("activate"), "").unwrap();

    let base = std::env::temp_dir().join("drift-test");
    let result = script_discovery::detect_venv_activation(&repo_path, &base);
    assert!(result.is_some(), "should find venv");

    cleanup_test_dir(&repo_path);
}

#[test]
fn test_detect_venv_activation_without_venv() {
    let repo_path = temp_test_path("no_venv");
    fs::create_dir_all(&repo_path).unwrap();

    let base = std::env::temp_dir().join("drift-test");
    let result = script_discovery::detect_venv_activation(&repo_path, &base);
    assert!(result.is_none(), "should not find venv");

    cleanup_test_dir(&repo_path);
}

#[test]
fn test_resolve_entrypoint_with_venv() {
    let repo_path = temp_test_path("entry_with_venv");
    fs::create_dir_all(&repo_path.join(".venv").join("bin")).unwrap();
    fs::write(repo_path.join(".venv").join("bin").join("activate"), "").unwrap();

    let base = std::env::temp_dir().join("drift-test");
    let cmd = script_discovery::resolve_entrypoint_to_spawn_cmd(&repo_path, "ati_plug", &base);
    assert!(cmd.is_ok());
    let cmd_str = cmd.unwrap();
    assert!(cmd_str.contains("source"), "should include source");
    assert!(cmd_str.contains(".venv/bin/activate"), "should reference venv activate");
    assert!(cmd_str.contains("ati_plug"), "should include script key");

    cleanup_test_dir(&repo_path);
}

#[test]
fn test_detect_venv_activation_with_covn_venv() {
    let base = temp_test_path("covn_venv_base");
    let covn_repo = base.join("covn").join("my-repo");
    fs::create_dir_all(&covn_repo.join(".venv").join("bin")).unwrap();
    fs::write(covn_repo.join(".venv").join("bin").join("activate"), "").unwrap();

    let result = script_discovery::detect_venv_activation(&covn_repo, &base);
    assert!(result.is_some(), "should find venv in standard covn location");
    let path = result.unwrap();
    assert!(path.contains("covn"), "should be in covn path");
    assert!(path.contains(".venv/bin/activate"), "should reference venv activate");

    cleanup_test_dir(&base);
}

#[test]
fn test_detect_venv_activation_with_drift_venv() {
    let base = temp_test_path("drift_venv_base");
    let drift_repo = base.join("drift").join("another-repo");
    fs::create_dir_all(&drift_repo.join(".venv").join("bin")).unwrap();
    fs::write(drift_repo.join(".venv").join("bin").join("activate"), "").unwrap();

    let result = script_discovery::detect_venv_activation(&drift_repo, &base);
    assert!(result.is_some(), "should find venv in standard drift location");
    let path = result.unwrap();
    assert!(path.contains("drift"), "should be in drift path");
    assert!(path.contains(".venv/bin/activate"), "should reference venv activate");

    cleanup_test_dir(&base);
}

#[test]
fn test_resolve_entrypoint_with_covn_venv() {
    let base = temp_test_path("entry_with_covn_venv");
    let covn_repo = base.join("covn").join("my-repo");
    fs::create_dir_all(&covn_repo.join(".venv").join("bin")).unwrap();
    fs::write(covn_repo.join(".venv").join("bin").join("activate"), "").unwrap();

    let cmd = script_discovery::resolve_entrypoint_to_spawn_cmd(&covn_repo, "ati_plug", &base);
    assert!(cmd.is_ok());
    let cmd_str = cmd.unwrap();
    assert!(cmd_str.contains("source"), "should include source");
    assert!(cmd_str.contains("covn"), "should reference covn path");
    assert!(cmd_str.contains(".venv/bin/activate"), "should reference venv activate");
    assert!(cmd_str.contains("ati_plug"), "should include script key");

    cleanup_test_dir(&base);
}

#[test]
fn test_detect_venv_activation_prefers_local_over_standard() {
    let base = temp_test_path("prefer_local");
    let covn_repo = base.join("covn").join("my-repo");
    fs::create_dir_all(&covn_repo.join(".venv").join("bin")).unwrap();
    fs::write(covn_repo.join(".venv").join("bin").join("activate"), "").unwrap();
    let standard_venv = base.join("drift").join("my-repo").join(".venv").join("bin").join("activate");
    fs::create_dir_all(standard_venv.parent().unwrap()).unwrap();
    fs::write(&standard_venv, "").unwrap();

    let result = script_discovery::detect_venv_activation(&covn_repo, &base);
    assert!(result.is_some());
    let path = result.unwrap();
    assert!(path.contains("covn"), "should prefer local venv");

    cleanup_test_dir(&base);
}

#[test]
fn test_resolve_entrypoint_without_venv() {
    let repo_path = temp_test_path("entry_no_venv");
    fs::create_dir_all(&repo_path).unwrap();

    let cmd = script_discovery::resolve_entrypoint_to_spawn_cmd(&repo_path, "ati_plug", &std::env::temp_dir());
    assert!(cmd.is_ok());
    let cmd_str = cmd.unwrap();
    assert!(!cmd_str.contains("source"), "should not include source");
    assert_eq!(cmd_str, "ati_plug");

    cleanup_test_dir(&repo_path);
}