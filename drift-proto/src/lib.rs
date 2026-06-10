use std::collections::{HashMap, VecDeque};
use std::env;
use std::path::PathBuf;
use std::fmt;
use std::fs;
use std::io::{BufReader, BufWriter};
use std::time::Instant;

use serde::{Deserialize, Serialize};

use drift_auth::{AuthMessage, SignedAuthMessage, AggregateAuthMessage};

/// Coordinator-side tracking of node assignment state.
#[derive(Debug, Clone)]
pub struct NodeSlot {
    pub node_id: String,
    pub state: SlotState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlotState {
    InFlight { last_heartbeat: Instant },
    Available { since: Instant },
    Failed { unclaimed_shard: Option<ShardAssignment> },
}

impl SlotState {
    pub fn is_available(&self) -> bool {
        matches!(self, Self::Available { .. })
    }

    pub fn is_in_flight(&self) -> bool {
        matches!(self, Self::InFlight { .. })
    }
}

#[allow(dead_code)]
fn slot_state_is_in_flight(state: &SlotState) -> bool {
    matches!(state, SlotState::InFlight { .. })
}

fn set_slot_to_failed(slot: &mut NodeSlot, node_id: &str) -> Option<ShardAssignment> {
    if let SlotState::InFlight { .. } = &slot.state {
        let shard = ShardAssignment {
            node_id: node_id.to_string(),
            shard_index: 0u32,
            shard_start: 0u64,
            shard_end: 1000u64,
        };
        slot.state = SlotState::Failed { unclaimed_shard: Some(shard.clone()) };
        Some(shard)
    } else {
        None
    }
}

#[derive(Default)]
/// Coordinator maintains global state across all connected nodes.
pub struct CoordState {
    pub pending: VecDeque<ShardAssignment>,
   pub slots: HashMap<String, NodeSlot>,
}

impl CoordState {
    pub fn new(initial_nodes: &[NodeInfo]) -> Self {
        let mut pending = VecDeque::new();
        let mut slots = HashMap::new();

        if !initial_nodes.is_empty() {
            let first_node = &initial_nodes[0];
            pending.push_back(ShardAssignment {
                node_id: first_node.node_id.clone(),
                shard_index: 0u32,
                shard_start: 0u64,
                shard_end: 1000u64,
            });
        }

        for node_info in initial_nodes.iter() {
            let slot = NodeSlot {
                node_id: node_info.node_id.clone(),
                state: SlotState::Available { since: Instant::now() },
            };
            slots.insert(node_info.node_id.clone(), slot);
        }

        Self { pending, slots }
    }

    pub fn register_node(&mut self, node_info: &NodeInfo) {
        let slot = NodeSlot {
            node_id: node_info.node_id.clone(),
            state: SlotState::Available { since: Instant::now() },
        };
        self.slots.insert(node_info.node_id.clone(), slot);

        if self.pending.is_empty() && self.has_in_flight_nodes() {}
    }

    pub fn has_in_flight_nodes(&self) -> bool {
        self.slots.values().any(|s| s.state.is_in_flight())
    }

    pub fn get_available_slot_mut(&mut self, node_id: &str) -> Option<&mut NodeSlot> {
        self.slots.get_mut(node_id)
    }

    pub fn pop_pending_assignment(&mut self) -> Option<ShardAssignment> {
        self.pending.pop_front()
    }

    pub fn mark_completed(&mut self, node_id: &str) -> bool {
        if let Some(slot) = self.slots.get_mut(node_id) {
            match &slot.state {
                SlotState::InFlight { .. } => {
                    slot.state = SlotState::Available { since: Instant::now() };
                    return true;
                }
                _ => return false,
            }
        }
        false
    }

    pub fn mark_failed_if_needed(
        &mut self,
        node_id: &str,
        last_seen: Instant,
        timeout_threshold_secs: u64,
    ) -> Option<ShardAssignment> {
        let now = Instant::now();
        if now.duration_since(last_seen).as_secs() <= timeout_threshold_secs {
            return None;
        }

        if let Some(slot) = self.slots.get_mut(node_id) {
            if matches!(&slot.state, SlotState::InFlight { .. }) {
                return set_slot_to_failed(slot, node_id);
            }
        }
   None
        }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalShardState {
    pub shard_assignment: ShardAssignment,
    pub train_config: TrainConfig,
    pub last_checkpoint_step: u64,
    pub completion_percentage: f32,
}

impl LocalShardState {
    pub fn local_cache_path(node_id: &str) -> PathBuf {
        if let Some(home) = env::var_os("HOME") {
            return PathBuf::from(&home).join(".drift").join(format!("shard-{}", node_id));
        }
        PathBuf::from(format!("/tmp/drift-shard-{}", node_id))
    }

    pub fn save_to_disk(&self, node_id: &str) -> anyhow::Result<()> {
        let path = Self::local_cache_path(node_id);
        
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        let file = fs::File::create(&path)?;
        let writer = BufWriter::new(file);
        serde_json::to_writer(writer, self)?;
        Ok(())
    }

    pub fn load_from_disk(node_id: &str) -> anyhow::Result<Option<Self>> {
        let path = Self::local_cache_path(node_id);
        
        if !path.exists() {
            return Ok(None);
        }
        
        let file = fs::File::open(&path)?;
        let reader = BufReader::new(file);
        let state: LocalShardState = serde_json::from_reader(reader)?;
        Ok(Some(state))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordEndpointCache {
    pub did_hash_address: String,
    pub public_key_or_secret: Option<String>,
}

impl CoordEndpointCache {
    pub fn cache_path() -> PathBuf {
        if let Some(home) = env::var_os("HOME") {
            return PathBuf::from(&home).join(".drift").join("coordinator.toml");
        }
        PathBuf::from("/tmp/drift-coordinator.toml")
    }
}

/// Information about a node's GPU capabilities.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeInfo {
    pub node_id: String,
    pub gpu_name: String,
    pub gpu_vram_mb: u64,
    pub gpu_compute_capability: String,
    pub available: bool,
}

impl fmt::Display for NodeInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} | {} ({} MB VRAM, compute {})",
            &self.node_id[..12.min(self.node_id.len())],
            self.gpu_name,
            self.gpu_vram_mb,
            self.gpu_compute_capability,
        )
    }
}

impl fmt::Display for DriftMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NodeInfo(info) => write!(f, "NodeInfo({})", info),
            Self::TrainConfig(c) => {
                write!(f, "TrainConfig(epochs={}, batch={})", c.epochs, c.batch_size)
            }
            Self::ShardAssignment(s) => {
                write!(f, "ShardAssignment(node={}, idx={})", &s.node_id[..12.min(s.node_id.len())], s.shard_index)
            }
            Self::TrainProgress(p) => {
                write!(f, "TrainProgress(step={}, loss={:.4})", p.step, p.loss)
            }
            Self::CheckpointInfo(c) => write!(f, "CheckpointInfo(step={})", c.step),
            Self::Ping => write!(f, "Ping"),
            Self::Pong => write!(f, "Pong"),
            Self::Heartbeat { uptime_secs } => write!(f, "Heartbeat({}s)", uptime_secs),
            Self::TrainComplete => write!(f, "TrainComplete"),
            Self::AskForMoreWork => write!(f, "AskForMoreWork"),
            Self::NoMoreWork => write!(f, "NoMoreWork"),
            Self::AssignNext(s) => write!(f, "AssignNext(shard={})", s.shard_index),
            Self::AuthChallenge(msg) => write!(f, "AuthChallenge(node={}, seq={})", msg.node_id, msg.sequence),
            Self::AuthResponse(signed) => write!(f, "AuthResponse(node={}, sig_len={})", signed.node_id, signed.signature.len()),
            Self::AuthAggregate(agg) => write!(f, "AuthAggregate(threshold={}/{}, nodes={})", agg.threshold, agg.total_nodes, agg.node_ids.len()),
            Self::TrainingReady => write!(f, "TrainingReady"),
            Self::TrainingCancel(c) => write!(f, "TrainingCancel(reason={}, time={}, repo={})", c.reason, c.time, c.repo_url),
            Self::RepoCommit(rc) => write!(f, "RepoCommit(commit={}, repo={})", &rc.commit[..8.min(rc.commit.len())], rc.repo_url),
        }
    }
}

/// Training configuration sent from coordinator to nodes.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrainConfig {
    // Existing fields
    pub model_path: String,
    pub dataset_path: String,
    pub batch_size: u32,
    pub learning_rate: f64,
    pub epochs: u32,

    // New fields for distributed repo-based training
    /// URL of the repository containing the training script.
    /// Node should clone this and run the specified entrypoint.
    #[serde(default)]
    pub train_repo_url: Option<String>,

    /// Path within the cloned train_repo to execute (e.g., "train.py").
    /// If not set, falls back to existing --script CLI argument behavior.
    #[serde(default)]
    pub script_entrypoint: Option<String>,

    /// HuggingFace repo ID or Git URL for the dataset.
    /// Node should download/clone this before starting training.
    #[serde(default)]
    pub dataset_repo_url: Option<String>,

    /// URLs for datasets (multiple datasets supported).
    #[serde(default)]
    pub dataset_urls: Vec<String>,

    /// Optional path within dataset_repo for fine-tuning from local base model.
    #[serde(default)]
    pub model_artifact_ref: Option<String>,

   /// Enable multi-signature authentication
    #[serde(default)]
    pub enable_auth: bool,

    /// Threshold for signature aggregation (e.g., 3 for 3-of-n).
    pub auth_threshold: usize,

    /// Agreed-upon git commit hash (set by coordinator after verification).
    #[serde(default)]
    pub git_commit: Option<String>,

    /// GPU compute capability (e.g., 8.9 for CUDA 8.9).
    #[serde(default)]
    pub gpu_compute_capability: Option<f64>,
}

/// Shard assignment for a specific node.
#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShardAssignment {
    pub node_id: String,
    pub shard_index: u32,
    pub shard_start: u64,
    pub shard_end: u64,
}

impl ShardAssignment {
    pub fn size(&self) -> u64 {
        self.shard_end.saturating_sub(self.shard_start)
    }

    pub fn save_to_disk(&self, node_id: &str) -> anyhow::Result<()> {
        let state = LocalShardState {
            shard_assignment: self.clone(),
            train_config: TrainConfig::default(),
            last_checkpoint_step: 0,
            completion_percentage: 0.0,
        };
        state.save_to_disk(node_id)?;
        Ok(())
    }

    pub fn load_from_disk(node_id: &str) -> anyhow::Result<Option<LocalShardState>> {
        LocalShardState::load_from_disk(node_id)
    }
}

/// Training progress report from a node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrainProgress {
    pub node_id: String,
    pub epoch: u32,
    pub step: u64,
    pub loss: f64,
    pub throughput_samples_per_sec: f64,
}

/// Checkpoint metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CheckpointInfo {
    pub step: u64,
    pub path: String,
    pub nodes_contributed: Vec<String>,
}

/// Git commit verification message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RepoCommit {
    pub commit: String,
    pub repo_url: String,
    pub signature: Vec<u8>,
}

/// Training cancellation message from coordinator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrainingCancel {
    pub reason: String,
    pub time: String,
    pub repo_url: String,
}

impl fmt::Display for TrainingCancel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TrainingCancel(reason={}, time={}, repo={})", self.reason, self.time, self.repo_url)
    }
}

/// All messages exchanged between drift nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DriftMessage {
    NodeInfo(NodeInfo),
    TrainConfig(TrainConfig),
    ShardAssignment(ShardAssignment),
    TrainProgress(TrainProgress),
    CheckpointInfo(CheckpointInfo),
    Ping,
    Pong,
    /// Periodic heartbeat with uptime in seconds.
    Heartbeat { uptime_secs: u64 },
    /// Coordinator signals training is complete.
    TrainComplete,
    
    /// Coordinator signals nodes to begin training (after commit verification).
    TrainingReady,

    /// Coordinator broadcasts: commit verification failed, abort.
    TrainingCancel(TrainingCancel),

    /// Node sends commit info for verification.
    RepoCommit(RepoCommit),
    
    AskForMoreWork,
    NoMoreWork,
    AssignNext(ShardAssignment),

    /// Coordinator sends to node: "please authenticate"
    AuthChallenge(AuthMessage),

    /// Node sends signed auth message to coordinator
    AuthResponse(SignedAuthMessage),

    /// Coordinator broadcasts aggregate back to all nodes
    AuthAggregate(AggregateAuthMessage),
}

/// ALPN protocol identifier for drift coordinator<->node traffic.
pub const DRIFT_ALPN: &[u8] = b"drift/0";

/// ALPN protocol identifier for node<->node ring all-reduce traffic.
pub const DRIFT_RING_ALPN: &[u8] = b"drift-ring/0";

/// Maximum allowed message size (64 MB).
pub const MAX_MESSAGE_SIZE: usize = 64 * 1024 * 1024;

/// Serialize a DriftMessage to bytes (length-prefixed JSON).
pub fn encode_message(msg: &DriftMessage) -> anyhow::Result<Vec<u8>> {
    let json = serde_json::to_vec(msg)?;
    let len = (json.len() as u32).to_be_bytes();
    let mut buf = Vec::with_capacity(4 + json.len());
    buf.extend_from_slice(&len);
    buf.extend_from_slice(&json);
    Ok(buf)
}

/// Read a length-prefixed JSON message from a recv stream.
pub async fn read_message(recv: &mut iroh::endpoint::RecvStream) -> anyhow::Result<DriftMessage> {
    let mut len_buf = [0u8; 4];
    recv.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf) as usize;

    if len > MAX_MESSAGE_SIZE {
        anyhow::bail!("message too large: {} bytes", len);
    }

    let mut buf = vec![0u8; len];
    recv.read_exact(&mut buf).await?;
    let msg: DriftMessage = serde_json::from_slice(&buf)?;
    Ok(msg)
}

/// Write a length-prefixed JSON message to a send stream.
pub async fn write_message(
    send: &mut iroh::endpoint::SendStream,
    msg: &DriftMessage,
) -> anyhow::Result<()> {
    let bytes = encode_message(msg)?;
    send.write_all(&bytes).await?;
    Ok(())
}
