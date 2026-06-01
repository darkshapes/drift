use anyhow::Result;
use tracing::{info, warn};

#[cfg(target_os = "macos")]
fn is_mac() -> bool { true }

#[cfg(not(target_os = "macos"))]
fn is_mac() -> bool { false }

async fn detect_arch() -> Option<(String, u64)> {
    let output = tokio::process::Command::new("sysctl")
        .arg("-n")
        .arg("hw.machine")
        .output()
        .await;

    match output {
        Ok(o) if o.status.success() => {
            let arch = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if arch.starts_with("arm") {
                let cpu_brand = detect_cpu_brand().await;
                let mem_output = tokio::process::Command::new("sysctl")
                    .args(["-n", "hw.memsize"])
                    .output()
                    .await;
                match mem_output {
                    Ok(m) if m.status.success() => {
                        let mem_bytes: u64 =
                            String::from_utf8_lossy(&m.stdout)
                                .trim()
                                .parse()
                                .unwrap_or(0);
                        Some((cpu_brand, mem_bytes))
                    }
                    _ => None,
                }
            } else {
                None
            }
        }
        _ => None,
    }
}

async fn detect_cpu_brand() -> String {
    let output = tokio::process::Command::new("sysctl")
        .args(["-n", "machdep.cpu.brand_string"])
        .output()
        .await
        .ok();

    match output {
        Some(o) if o.status.success() => {
            String::from_utf8_lossy(&o.stdout).trim().to_string()
        }
        _ => String::new(),
    }
}

fn parse_compute_capability(cpu_brand: &str) -> String {
    let mut chip_letter = None;
    let mut has_five = false;

    for c in cpu_brand.chars() {
        if c == 'M' || c == 'A' {
            chip_letter = Some(c);
        } else if c.is_ascii_digit() && c == '5' {
            has_five = true;
        }
    }

    let tier = cpu_brand.split(' ').last().unwrap_or("").to_lowercase();
    let is_m5 = chip_letter == Some('M') && has_five;

    if is_m5 && tier == "ultra" {
        return "10".to_string();
    }
    if !is_m5 && tier == "ultra" {
        return "8.9".to_string();
    }
    if tier == "max" || (chip_letter == Some('M') && tier != "pro") {
        return "8.6".to_string();
    }
    if tier == "pro" || tier == "base" || (!tier.is_empty()) {
        return "8.0".to_string();
    }
    "7.5".to_string()
}

pub async fn detect_gpu_info() -> Vec<GpuInfo> {
    if !is_mac() {
        return vec![];
    }

    let arch_data = detect_arch().await;

    if let Some((cpu_brand, mem_bytes)) = arch_data {
        let vram_mb: u64 = mem_bytes / 1024 / 1024;
        let compute_capability = parse_compute_capability(&cpu_brand);
        let chip_name = if cpu_brand.is_empty() { "Apple Silicon" } else { &cpu_brand };

        return vec![GpuInfo {
            name: chip_name.to_string(),
            vram_mb,
            compute_capability,
        }];
    }

    vec![]
}

#[derive(Debug, Clone)]
pub struct GpuInfo {
    pub name: String,
    pub vram_mb: u64,
    pub compute_capability: String,
}

/// Detect GPUs by running nvidia-smi.
pub async fn detect_gpus() -> Result<Vec<GpuInfo>> {
    let output = tokio::process::Command::new("nvidia-smi")
        .args([
            "--query-gpu=name,memory.total,compute_cap",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .await;

    match output {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let gpus: Vec<GpuInfo> = stdout
                .lines()
                .filter(|line| !line.trim().is_empty())
                .filter_map(|line| {
                    let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
                    if parts.len() >= 3 {
                        let vram = parts[1].parse::<u64>().unwrap_or(0);
                        Some(GpuInfo {
                            name: parts[0].to_string(),
                            vram_mb: vram,
                            compute_capability: parts[2].to_string(),
                        })
                    } else {
                        warn!("unexpected nvidia-smi output line: {}", line);
                        None
                    }
                })
                .collect();

            if gpus.is_empty() {
                warn!("nvidia-smi returned no GPUs");
            } else {
                for gpu in &gpus {
                    info!(
                        name = %gpu.name,
                        vram_mb = gpu.vram_mb,
                        compute = %gpu.compute_capability,
                        "detected GPU"
                    );
                }
            }

            Ok(gpus)
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("nvidia-smi failed: {}", stderr);
            Ok(vec![])
        }
        Err(e) => {
            warn!("nvidia-smi not found: {} (no NVIDIA GPU detected)", e);
            Ok(vec![])
        }
    }
}

/// Query the NVIDIA driver version, if available.
pub async fn driver_version() -> Option<String> {
    let output = tokio::process::Command::new("nvidia-smi")
        .args(["--query-gpu=driver_version", "--format=csv,noheader"])
        .output()
        .await
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let version = String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()?
        .trim()
        .to_string();

    if version.is_empty() {
        None
    } else {
        Some(version)
    }
}

/// Return a placeholder GPU for systems without NVIDIA GPUs (e.g. development).
pub fn placeholder_gpu() -> GpuInfo {
    GpuInfo {
        name: "CPU-only (no GPU detected)".to_string(),
        vram_mb: 0,
        compute_capability: "0.0".to_string(),
    }
}
