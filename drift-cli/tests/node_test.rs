use drift_cli::node::find_local_repo;
use std::env;
use std::path::PathBuf;

fn local_share_path() -> PathBuf {
    match env::var_os("HOME") {
        Some(home) => PathBuf::from(&home).join(".local").join("share"),
        None => PathBuf::from("/tmp").join(".local").join("share"),
    }
}

#[test]
fn test_path_construction() {
    let home = env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let expected = PathBuf::from(home).join(".local").join("share");
    let actual = local_share_path();
    assert_eq!(expected, actual, "Path construction should use ~/.local/share");
}

#[test]
fn test_repo_parsing() {
    let url = "https://github.com/owner/repo_name";
    let result = find_local_repo(url);
    assert!(result.is_some() || result.is_none(), "Should parse URL without panicking");
}

#[test]
fn test_empty_url_handling() {
    let result = find_local_repo("");
    assert!(result.is_none(), "Empty URL should return None");
}

#[test]
fn test_single_slash_url() {
    let result = find_local_repo("repo_name");
    assert!(result.is_none(), "Single component URL should not find path");
}