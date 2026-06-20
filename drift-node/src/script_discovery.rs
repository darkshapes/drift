use std::path::{Path, PathBuf};
use anyhow::{Context, Result};

pub fn discover_script_entrypoint(repo_path: &Path) -> Result<String> {
    let pyproject_path = repo_path.join("pyproject.toml");
    let content = std::fs::read_to_string(&pyproject_path)
        .with_context(|| format!("reading {:?}", pyproject_path))?;
    let value: toml::Value = toml::from_str(&content)
        .with_context(|| format!("parsing {:?}", pyproject_path))?;
    if let Some(ati_plug) = find_ati_plug(&value) {
        return Ok(ati_plug);
    }
    anyhow::bail!("ati_plug not found in {:?}", pyproject_path);
}

fn find_ati_plug(value: &toml::Value) -> Option<String> {
    if let Some(project_table) = value.as_table() {
        if let Some(project) = project_table.get("project") {
            if let Some(project_table) = project.as_table() {
                if let Some(scripts) = project_table.get("scripts") {
                    if let Some(scripts_table) = scripts.as_table() {
                        for (key, value) in scripts_table {
                            if key.ends_with("ati_plug") {
                                if let Some(_s) = value.as_str() {
                                    return Some(key.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    if let Some(project_table) = value.as_table() {
        if let Some(tool) = project_table.get("tool") {
            if let Some(tool_table) = tool.as_table() {
                if let Some(uv) = tool_table.get("uv") {
                if let Some(uv_table) = uv.as_table() {
                    if let Some(scripts) = uv_table.get("scripts") {
                        if let Some(scripts_table) = scripts.as_table() {
                        for (key, value) in scripts_table {
                            if key.ends_with("ati_plug") {
                                if let Some(_s) = value.as_str() {
                                    return Some(key.to_string());
                                }
                            }
                        }
                        }
                    }
                }
                }
            }
        }
    }
    // Check root-level [scripts] section
    if let Some(project_table) = value.as_table() {
        if let Some(scripts) = project_table.get("scripts") {
            if let Some(scripts_table) = scripts.as_table() {
                for (key, value) in scripts_table {
                    if key.ends_with("ati_plug") {
                        if let Some(_s) = value.as_str() {
                            return Some(key.to_string());
                        }
                    }
                }
            }
        }
    }
    None
}

pub fn repo_suffix_from_url(url: &str) -> String {
    let url = url.trim_end_matches(".git");
    let last_slash = url.rfind('/').map(|i| i + 1).unwrap_or(0);
    url[last_slash..].to_string()
}

pub fn detect_venv_activation(repo_path: &Path, base: &Path) -> Option<String> {
    let local_venv = repo_path.join(".venv").join("bin").join("activate");
    if local_venv.exists() {
        return Some(local_venv.display().to_string());
    }
    let repo_name = repo_path.file_name().map(|p| p.to_str()).unwrap_or(None).unwrap_or("");
    for dir in &["covn", "drift"] {
        let standard_venv = base.join(dir).join(repo_name).join(".venv").join("bin").join("activate");
        if standard_venv.exists() {
            return Some(standard_venv.display().to_string());
        }
    }
    None
}

pub fn find_preprovisioned_repo(url: &str, base: &Path) -> Result<PathBuf> {
    let suffix = repo_suffix_from_url(url);
    let covn_path = base.join("covn").join(&suffix);
    if covn_path.exists() {
        return Ok(covn_path);
    }
    let drift_path = base.join("drift").join(&suffix);
    if drift_path.exists() {
        return Ok(drift_path);
    }
    anyhow::bail!(
        "repo not found in ~/.local/share/covn/ or ~/.local/share/drift/: {}",
        suffix
    );
}

pub fn resolve_entrypoint_to_spawn_cmd(_repo_path: &Path, script_key: &str, base: &Path) -> Result<String> {
    if script_key.is_empty() {
        anyhow::bail!("empty script key");
    }
    if let Some(activate) = detect_venv_activation(_repo_path, base) {
        Ok(format!("source {} && {}", activate, script_key))
    } else {
        Ok(script_key.to_string())
    }
}

pub async fn clone_repo_to_drift_cache(url: &str, base: &Path) -> Result<PathBuf> {
    if let Ok(existing) = find_preprovisioned_repo(url, base) {
        return Ok(existing);
    }
    let suffix = repo_suffix_from_url(url);
    let dest = base.join("drift").join(suffix);
    let output = tokio::process::Command::new("git")
        .args(["clone", "--depth", "1", url, dest.to_str().unwrap_or("")])
        .output()
        .await
        .context("git clone failed")?;
    if !output.status.success() {
        anyhow::bail!("git clone failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    Ok(dest)
}