use anyhow::Result;
use drift_proto::{
    read_message, write_message, DriftMessage, NodeInfo, DRIFT_ALPN, DRIFT_RING_ALPN, RepoCommit, TrainConfig, ShardAssignment,
};
use iroh::{Endpoint, PublicKey};

use tokio::io::{AsyncBufReadExt, BufReader};
use tracing::{error, info, warn};

use crate::ipc::{self, PythonMessage};




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

    if is_m5 && tier == "ultra" { return "10".to_string(); }
    if !is_m5 && tier == "ultra" { return "8.9".to_string(); }
    if tier == "max" || (chip_letter == Some('M') && tier != "pro") { return "8.6".to_string(); }
    if tier == "pro" || tier == "base" || (!tier.is_empty()) { return "8.0".to_string(); }
    "7.5".to_string()
}

/// Check system architecture on MacOS.
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

/// Detect GPUs via nvidia-smi. Returns empty vec if unavailable.
async fn detect_gpus() -> Vec<(String, u64, String)> {
    let output = tokio::process::Command::new("nvidia-smi")
        .args([
            "--query-gpu=name,memory.total,compute_cap",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .await;

    match output {
        Ok(o) if o.status.success() => {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .filter(|l| !l.trim().is_empty())
                .filter_map(|line| {
                    let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
                    if parts.len() >= 3 {
                        Some((
                            parts[0].to_string(),
                            parts[1].parse::<u64>().unwrap_or(0),
                            parts[2].to_string(),
                        ))
                    } else {
                        None
                    }
                })
                .collect()
        }
        _ => vec![],
    }
}

pub async fn join(name: Option<String>) -> Result<()> {
    #[cfg(target_os = "macos")]
    fn is_mac() -> bool { true }

    #[cfg(not(target_os = "macos"))]
    fn is_mac() -> bool { false }

    let gpus: Vec<(String, u64, String)> = if is_mac() {
        match detect_arch().await {
            Some((cpu_brand, mem_bytes)) => {
                let vram_mb = mem_bytes / 1024 / 1024;
                let compute_capability = parse_compute_capability(&cpu_brand);
                let chip_name = if cpu_brand.is_empty() { "Apple Silicon" } else { &cpu_brand };
                vec![(chip_name.to_string(), vram_mb, compute_capability)]
            }
            None => vec![],
        }
    } else {
        detect_gpus().await
    };
    let (gpu_name, gpu_vram, gpu_cc) = gpus.first().cloned().unwrap_or((
        "CPU-only (no GPU detected)".to_string(),
        0,
        "0.0".to_string(),
    ));
    let total_vram: u64 = if gpus.is_empty() {
        0
    } else {
        gpus.iter().map(|g| g.1).sum()
    };

    let endpoint = Endpoint::builder()
        .alpns(vec![DRIFT_ALPN.to_vec(), DRIFT_RING_ALPN.to_vec()])
        .bind()
        .await?;

    let node_id = endpoint.id();
        let short_id = node_id.to_string();
    let display_name = name.unwrap_or_else(|| {
        if short_id.chars().count() > 12 {
            short_id.chars().take(12).collect::<String>()
        } else {
            short_id.clone()
        }
    });

    println!("drift node started");
    println!("  Node ID:  {}", node_id);
    println!("  Name:     {}", display_name);
    if gpus.len() <= 1 {
        println!("  GPU:      {} ({} MB VRAM)", gpu_name, gpu_vram);
    } else {
        println!("  GPUs:     {} devices ({} MB total VRAM)", gpus.len(), total_vram);
        for (i, (name, vram, _)) in gpus.iter().enumerate() {
            println!("    [{}] {} ({} MB)", i, name, vram);
        }
    }
    println!();
    println!("Share your Node ID with the coordinator to join training.");
    println!("Waiting for connections...");

    let node_info_msg = DriftMessage::NodeInfo(NodeInfo {
        node_id: node_id.to_string(),
        gpu_name,
        gpu_vram_mb: total_vram.max(gpu_vram),
        gpu_compute_capability: gpu_cc,
        available: true,
    });

    let accept_loop = async {
        loop {
            let incoming = match endpoint.accept().await {
                Some(incoming) => incoming,
                None => {
                    info!("endpoint closed");
                    break;
                }
            };

            let node_info = node_info_msg.clone();
            let ep = endpoint.clone();
            tokio::spawn(async move {
                match incoming.await {
                    Ok(conn) => {
                        if let Err(e) = handle_connection(conn, node_info, ep).await {
                            error!("connection error: {}", e);
                        }
                    }
                    Err(e) => error!("accept error: {}", e),
                }
            });
        }
    };

    tokio::select! {
        _ = accept_loop => {}
        _ = tokio::signal::ctrl_c() => {
            println!();
            println!("shutting down...");
        }
    }

    endpoint.close().await;
    Ok(())
}

async fn handle_connection(
    conn: iroh::endpoint::Connection,
    node_info_msg: DriftMessage,
    endpoint: Endpoint,
) -> Result<()> {
    let remote = conn.remote_id();
    info!(%remote, "coordinator connected");

    let (mut send, mut recv) = conn.accept_bi().await?;

    let msg = read_message(&mut recv).await?;
    if !matches!(msg, DriftMessage::Ping) {
        anyhow::bail!("expected Ping, got {}", msg);
    }

    write_message(&mut send, &node_info_msg).await?;
    info!("sent node info");

    let mut train_config = None;
    let mut repo_commit_sent = false;
    let mut shard_assignment = None;
    let standby_start = std::time::Instant::now();
    let mut training_ready_received = false;

    loop {
        if training_ready_received && train_config.is_some() && shard_assignment.is_some() {
            break;
        }

        if standby_start.elapsed() > std::time::Duration::from_secs(30) {
            return Err(anyhow::anyhow!("Standby timeout: no TrainingReady after 30s"));
        }

        match tokio::time::timeout(std::time::Duration::from_millis(100), read_message(&mut recv)).await {
            Ok(msg_result) => match msg_result {
                Ok(msg) => match msg {
                    DriftMessage::Ping => {
                        write_message(&mut send, &DriftMessage::Pong).await?;
                    }
                    DriftMessage::TrainConfig(config) => {
                         info!(model = %config.model_path, epochs = config.epochs, "received config");
                         // Forward TrainConfig to drift-node and receive signed RepoCommit
                         // For stage 3, we use a placeholder; will be replaced with actual forwarding in later stages.
                         let repo_url = config.train_repo_url.as_ref().ok_or_else(|| anyhow::anyhow!("No train_repo_url in config"))?;
                         let repo_path = find_local_repo(repo_url).ok_or_else(|| anyhow::anyhow!("Repo not found locally"))?;
                         let commit_hash = run_git_ls_remote(&repo_path).ok_or_else(|| anyhow::anyhow!("git ls-remote failed"))?;
let node_id_str = endpoint.id().to_string();
    let secret_key = endpoint.secret_key();
    let message = format!("{}|{}|{}", node_id_str, commit_hash, repo_url);
    let signature = secret_key.sign(message.as_bytes()).to_bytes().to_vec();
                         let repo_commit = RepoCommit {
                              commit: commit_hash,
                              repo_url: repo_url.to_string(),
                              signature,
                          };
                         write_message(&mut send, &DriftMessage::RepoCommit(repo_commit)).await?;
                         info!("sent RepoCommit to coordinator");
                         train_config = Some(config);
                     }
                    DriftMessage::TrainingReady => {
                        info!("TrainingReady received");
                        training_ready_received = true;

                        if let Some(ref mut config) = train_config {
                            if config.train_repo_url.is_some() && config.script_entrypoint.is_none() {
                                if let Some(repo_url) = &config.train_repo_url {
                                    let repo_path = find_local_repo(repo_url);
                                    if let Some(path) = repo_path {
                                    let base = if let Ok(home) = std::env::var("HOME") {
                                        std::path::PathBuf::from(home).join(".local/share")
                                    } else {
                                        std::path::PathBuf::from("/tmp")
                                    };
                                        match discover_script_entrypoint(&path) {
                                            Ok(entrypoint) => {
                                                config.script_entrypoint = Some(entrypoint);
                                                if let Ok(spawn_cmd) = resolve_entrypoint_to_spawn_cmd(
                                                    &path,
                                                    config.script_entrypoint.as_ref().unwrap(),
                                                    &base,
                                                ) {
                                                    config.training_spawn_cmd = Some(spawn_cmd);
                                                }
                                                let ep = config.script_entrypoint.as_ref().map(|s| s.as_str()).unwrap_or("<none>");
                                                info!(entrypoint = %ep, "script entrypoint discovered");
                                            }
                                            Err(e) => {
                                                error!(%e, "script discovery failed");
                                                let now = format!("{}", time::OffsetDateTime::now_utc());
                                                write_message(&mut send, &DriftMessage::TrainingCancel(drift_proto::TrainingCancel {
                                                    repo_url: config.train_repo_url.clone().unwrap_or_default(),
                                                    reason: e.to_string(),
                                                    time: now,
                                                })).await?;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    DriftMessage::TrainingCancel(cancel) => {
                        error!(reason = %cancel.reason, repo_url = %cancel.repo_url, "Training cancelled");
                        return Err(anyhow::anyhow!("Training cancelled: {}", cancel.reason));
                    }
                    DriftMessage::ShardAssignment(s) => {
                        info!(shard_index = s.shard_index, size = s.size(), "received shard");
                        shard_assignment = Some(s);
                    }
                    DriftMessage::Heartbeat { .. } => {
                        write_message(&mut send, &DriftMessage::Heartbeat { uptime_secs: 0 }).await?;
                    }
                    DriftMessage::TrainComplete => {
                        info!("training complete");
                        break;
                    }
                    other => {
                        info!(%other, "received message");
                    }
                },
                Err(e) => {
                    warn!("connection closed: {}", e);
                    break;
                }
            },
            Err(_) => {
                continue;
            }
        }
    }

    if training_ready_received {
        if let (Some(config), Some(shard)) = (train_config, shard_assignment) {
            info!("starting training after TrainingReady");
            run_training(&config, &mut send, &mut recv, Some(&shard)).await?;
        }
    }

    Ok(())
}

/// Execute training and stream progress back to coordinator.
/// Dispatches to real Python training if model_path is a .py file, otherwise simulates.
async fn run_training(
    config: &drift_proto::TrainConfig,
    coord_send: &mut iroh::endpoint::SendStream,
    coord_recv: &mut iroh::endpoint::RecvStream,
    shard: Option<&drift_proto::ShardAssignment>,
) -> Result<()> {
    match (config.train_repo_url.as_ref(), config.script_entrypoint.as_ref()) {
        (Some(_), None) => {
            anyhow::bail!("train_repo_url is set but script_entrypoint is missing. Ensure an ati_plug script is defined in pyproject.toml.");
        }
        (None, None) => {
            info!("train_repo_url and script_entrypoint are both None - running simulated training");
            simulate_training(config, coord_send).await
        }
        (_, Some(entrypoint)) => {
            info!(entrypoint = %entrypoint, "script_entrypoint found - preparing repo-based real training");
            run_real_training(config, coord_send, coord_recv, shard).await
        }
    }
}

/// Run real Python training via subprocess.
async fn run_real_training(
    config: &drift_proto::TrainConfig,
    coord_send: &mut iroh::endpoint::SendStream,
    _coord_recv: &mut iroh::endpoint::RecvStream,
    shard: Option<&drift_proto::ShardAssignment>,
) -> Result<()> {
    use std::process::Stdio;


    let master_port = 29500 + (std::process::id() % 1000);
    let mut cmd = if let Some(spawn_cmd) = &config.training_spawn_cmd {
        let mut c = tokio::process::Command::new("bash");
        c.arg("-c").arg(spawn_cmd);
        info!(spawn_cmd = %spawn_cmd, "using training_spawn_cmd");
        c
    } else {
        warn!("training_spawn_cmd not available - using legacy mode with model_path");
        let mut c = tokio::process::Command::new("python3");
        c.arg(&config.model_path);
        c
    };

    cmd.env("DRIFT_BATCH_SIZE", config.batch_size.to_string())
        .env("DRIFT_LEARNING_RATE", config.learning_rate.to_string())
        .env("DRIFT_EPOCHS", config.epochs.to_string())
        .env("MASTER_ADDR", "127.0.0.1")
        .env("MASTER_PORT", master_port.to_string());

    if let Some(dataset_path) = std::env::var_os("DRIFT_DATASET_PATH") {
        cmd.env("DRIFT_DATASET_PATH", dataset_path);
    }

    if let Some(s) = shard {
        cmd.env("DRIFT_SHARD_INDEX", s.shard_index.to_string())
            .env("DRIFT_SHARD_START", s.shard_start.to_string())
            .env("DRIFT_SHARD_END", s.shard_end.to_string());
    }

    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;

    let child_stdin = child.stdin.take().expect("piped stdin");
    let child_stdout = child.stdout.take().expect("piped stdout");
    let _stdin_writer = child_stdin;
    let mut stdout_reader = BufReader::new(child_stdout).lines();

    // 3. Wait for DRIFT_READY with timeout
    let ready_deadline = tokio::time::timeout(
        std::time::Duration::from_secs(60),
        async {
            while let Some(line) = stdout_reader.next_line().await? {
                if matches!(ipc::parse_python_line(&line), PythonMessage::Ready) {
                    return Ok::<_, anyhow::Error>(());
                }
            }
            anyhow::bail!("Python subprocess exited before sending DRIFT_READY")
        },
    )
    .await;

    match ready_deadline {
        Ok(Ok(())) => info!("Python subprocess ready"),
        Ok(Err(e)) => {
            let _ = child.kill().await;
            return Err(e);
        }
        Err(_) => {
            let _ = child.kill().await;
            anyhow::bail!("Python subprocess did not send DRIFT_READY within 60s — check stderr for errors");
        }
    }

    let _last_barrier_step: Option<u64> = None;

    while let Some(line) = stdout_reader.next_line().await? {
        let msg = ipc::parse_python_line(&line);
        match msg {
            PythonMessage::Ready => {
                // Already handled above, but harmless
            }
            PythonMessage::Progress { epoch, step, loss, throughput } => {
                let progress = drift_proto::TrainProgress {
                    node_id: format!("rank-{}", "N/A"),
                    epoch,
                    step,
                    loss,
                    throughput_samples_per_sec: throughput,
                };
                write_message(coord_send, &DriftMessage::TrainProgress(progress)).await?;
            }
            PythonMessage::Done => {
                info!("Python training complete");
                break;
            }
            PythonMessage::Unknown(line) => {
                // Log non-protocol lines from Python
                if !line.is_empty() {
                    info!(line, "python output");
                }
            }
        }
    }

    // 5. Wait for child exit with timeout
    match tokio::time::timeout(std::time::Duration::from_secs(30), child.wait()).await {
        Ok(Ok(status)) => {
            if !status.success() {
                warn!(code = ?status.code(), "Python subprocess exited with error");
            }
        }
        Ok(Err(e)) => {
            warn!("error waiting for Python subprocess: {}", e);
        }
        Err(_) => {
            warn!("Python subprocess did not exit within 30s, killing");
            let _ = child.kill().await;
        }
    }

    Ok(())
}

/// Simulate training progress with gradient synchronization.
async fn simulate_training(
    config: &drift_proto::TrainConfig,
    coord_send: &mut iroh::endpoint::SendStream,
) -> Result<()> {

    let steps_per_epoch = 5u64;
    let mut loss = 2.5_f64;

    for epoch in 0..config.epochs {
        for step_in_epoch in 0..steps_per_epoch {
            let global_step = epoch as u64 * steps_per_epoch + step_in_epoch;

            tokio::time::sleep(std::time::Duration::from_millis(50)).await;


            loss *= 0.98;
            let progress = drift_proto::TrainProgress {
                node_id: format!("rank-{}", "N/A"),
                epoch,
                step: global_step,
                loss,
                throughput_samples_per_sec: config.batch_size as f64 * 10.0,
            };
            write_message(coord_send, &DriftMessage::TrainProgress(progress)).await?;
        }
    }

    Ok(())
}

pub async fn status() -> Result<()> {
    #[cfg(target_os = "macos")]
    fn is_mac() -> bool { true }

    #[cfg(not(target_os = "macos"))]
    fn is_mac() -> bool { false }

    let gpus: Vec<(String, u64, String)> = if is_mac() {
        match detect_arch().await {
            Some((cpu_brand, mem_bytes)) => {
                let vram_mb = mem_bytes / 1024 / 1024;
                let compute_capability = parse_compute_capability(&cpu_brand);
                let chip_name = if cpu_brand.is_empty() { "Apple Silicon" } else { &cpu_brand };
                vec![(chip_name.to_string(), vram_mb, compute_capability)]
            }
            None => vec![],
        }
    } else {
        detect_gpus().await
    };

    // Driver version
    if let Ok(output) = tokio::process::Command::new("nvidia-smi")
        .args(["--query-gpu=driver_version", "--format=csv,noheader"])
        .output()
        .await
    {
        if output.status.success() {
            let v = String::from_utf8_lossy(&output.stdout);
            let v = v.trim();
            if !v.is_empty() {
                println!("  Driver: {}", v);
            }
        }
    }

    println!("drift node status");
    println!("---");
    if gpus.is_empty() {
        println!("  GPUs: none detected");
    } else {
        let total_vram: u64 = gpus.iter().map(|g| g.1).sum();
        for (i, (name, vram, cc)) in gpus.iter().enumerate() {
            println!("  GPU {}: {} ({} MB VRAM, compute {})", i, name, vram, cc);
        }
        if gpus.len() > 1 {
            println!("  Total: {} MB VRAM across {} devices", total_vram, gpus.len());
        }
    }

    Ok(())
}

pub fn find_local_repo(repo_url: &str) -> Option<std::path::PathBuf> {
    let repo_name = repo_url
        .split('/')
        .last()
        .unwrap_or("repo");

    if repo_name.is_empty() {
        return None;
    }

    let parts: Vec<&str> = repo_url.split('/').collect();
    let full_name = if parts.len() >= 2 {
        format!("{}/{}", parts[parts.len() - 2], repo_name)
    } else {
        repo_name.to_string()
    };

    if let Some(home) = std::env::var_os("HOME") {
        for dir in &["covn", "drift"] {
            let base = std::path::Path::new(&home).join(".local").join("share").join(dir);

            let full_path = base.join(&full_name);
            if full_path.exists() {
                return Some(full_path);
            }

            let short_path = base.join(&repo_name);
            if short_path.exists() {
                return Some(short_path);
            }
        }
    }

    None
}

use std::path::{Path, PathBuf};
use anyhow::Context;

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
                                if let Some(_) = value.as_str() {
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
                                    if let Some(_) = value.as_str() {
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
    if let Some(project_table) = value.as_table() {
        if let Some(scripts) = project_table.get("scripts") {
            if let Some(scripts_table) = scripts.as_table() {
                for (key, value) in scripts_table {
                    if key.ends_with("ati_plug") {
                        if let Some(_) = value.as_str() {
                            return Some(key.to_string());
                        }
                    }
                }
            }
        }
    }
    None
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

fn run_git_ls_remote(repo_path: &std::path::Path) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["ls-remote", &repo_path.to_string_lossy(), "HEAD"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .next()
        .map(|line| line.split_whitespace().next())
        .flatten()
        .map(|hash| hash.to_string())
}


