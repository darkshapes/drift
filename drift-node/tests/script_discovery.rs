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
    let entrypoint = result.unwrap();
    assert_eq!(entrypoint, "src.main:ati");

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
    let entrypoint = result.unwrap();
    assert_eq!(entrypoint, "src.main:ati");

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
    let entrypoint = result.unwrap();
    assert_eq!(entrypoint, "src.main:project_ati");

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