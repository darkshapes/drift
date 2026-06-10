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
                        if let Some(ati_plug) = scripts_table.get("ati_plug") {
                            if let Some(s) = ati_plug.as_str() {
                                return Some(s.to_string());
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
                                if let Some(ati_plug) = scripts_table.get("ati_plug") {
                                    if let Some(s) = ati_plug.as_str() {
                                        return Some(s.to_string());
                                    }
                                }
                            }
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

pub fn resolve_entrypoint_to_spawn_cmd(_repo_path: &Path, entrypoint: &str) -> Result<String> {
    if entrypoint.is_empty() {
        anyhow::bail!("empty entrypoint");
    }
    if !entrypoint.contains(':') {
        anyhow::bail!("invalid entrypoint format: {}", entrypoint);
    }
    let parts: Vec<&str> = entrypoint.split(':').collect();
    let module = parts.first().unwrap_or(&"");
    let func = parts.get(1).unwrap_or(&"");
    if module.is_empty() || func.is_empty() {
        anyhow::bail!("invalid entrypoint format: {}", entrypoint);
    }
    let repo_str = _repo_path.to_str().unwrap_or("");
    Ok(format!("PYTHONPATH={} python -c \"from {} import {}; {}()\"", repo_str, module, func, func))
}

pub async fn clone_repo_to_drift_cache(url: &str, base: &Path) -> Result<PathBuf> {
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