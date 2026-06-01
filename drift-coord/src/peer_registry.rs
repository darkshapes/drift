use drift_proto::{DriftMessage, ShardAssignment};
use std::collections::{HashMap, VecDeque};
use serde::{Deserialize, Serialize};

/// Serializable registry state (excludes runtime-only fields).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryState {
    pub peers: HashMap<String, PeerEntry>,
}

impl Default for RegistryState {
    fn default() -> Self {
        Self { peers: HashMap::new() }
    }
}

impl PeerRegistry {
    #[allow(dead_code)]
    pub fn new_with_pending_shards(count: usize) -> Self {
        let mut pending = VecDeque::new();
        for i in 0..count {
            pending.push_back(ShardAssignment {
                node_id: format!("pending_node_{}", i),
                shard_index: 100 + i as u32,
                shard_start: (100_u64 + i as u64) * 1000,
                shard_end: ((100_u64 + i as u64) * 1000).saturating_add(999),
            });
        }
        Self {
            state: RegistryState::default(),
            pending,
        }
    }

    #[allow(dead_code)]
    pub fn empty() -> Self {
        Self::new()
    }

    #[allow(dead_code)]
    pub fn new_with_failed_and_idles(failed_count: usize, idle_count: usize) -> Self {
        let mut peers = HashMap::new();
        for i in 0..failed_count {
            let entry = PeerEntry {
                did_hash_address: format!("did:test:failed_{}", i),
                original_shard: ShardAssignment {
                    node_id: format!("failed_node_{}", i),
                    shard_index: (50 + i) as u32,
                    shard_start: (i as u64) * 10000,
                    shard_end: ((i as u64) + 1) * 10000,
                },
                status: NodeStatus::Failed { unclaimed_since: time::OffsetDateTime::now_utc() },
                last_seen: None,
            };
            peers.insert(format!("failed_node_{}", i), entry);
        }
        for i in 0..idle_count {
            let entry = PeerEntry {
                did_hash_address: format!("did:test:idle_{}", i),
                original_shard: ShardAssignment {
                    node_id: format!("idle_node_{}", i),
                    shard_index: i as u32,
                    shard_start: (i as u64) * 1000,
                    shard_end: ((i as u64)).saturating_add(1) * 1000,
                },
                status: NodeStatus::Idle,
                last_seen: Some(time::OffsetDateTime::now_utc()),
            };
            peers.insert(format!("idle_node_{}", i), entry);
        }
        Self {
            state: RegistryState { peers },
            pending: VecDeque::new(),
        }
    }

    #[allow(dead_code)]
    pub fn new_with_mixed_peers(active_count: usize, stale_count: usize) -> Self {
        let mut peers = HashMap::new();
        for i in 0..active_count {
            let entry = PeerEntry {
                did_hash_address: format!("did:test:active_{}", i),
                original_shard: ShardAssignment {
                    node_id: format!("active_{}", i),
                    shard_index: i as u32,
                    shard_start: (i as u64) * 10000,
                    shard_end: ((i as u64) + 1) * 10000,
                },
                status: NodeStatus::Active,
                last_seen: Some(time::OffsetDateTime::now_utc()),
            };
            peers.insert(format!("active_{}", i), entry);
        }
        for i in 0..stale_count {
            let entry = PeerEntry {
                did_hash_address: format!("did:test:stale_{}", i),
                original_shard: ShardAssignment {
                    node_id: format!("stale_{}", i),
                    shard_index: (active_count + i) as u32,
                    shard_start: ((active_count as u64) + (i as u64)) * 10000,
                    shard_end: (((active_count as u64) + (i as u64)) + 1) * 10000,
                },
                status: NodeStatus::Stale { since: time::OffsetDateTime::now_utc() },
                last_seen: None,
            };
            peers.insert(format!("stale_{}", i), entry);
        }
        Self {
            state: RegistryState { peers },
            pending: VecDeque::new(),
        }
    }

    #[allow(dead_code)]
    pub fn add_failed_node(&mut self, node_id: String, shard_idx: usize) {
        let entry = PeerEntry {
            did_hash_address: "did:test".to_string(),
            original_shard: ShardAssignment {
                node_id: node_id.clone(),
                shard_index: shard_idx as u32,
                shard_start: 0u64,
                shard_end: 1000u64,
            },
            status: NodeStatus::Failed { unclaimed_since: time::OffsetDateTime::now_utc() },
            last_seen: None,
        };
        self.state.peers.insert(node_id, entry);
    }

    #[allow(dead_code)]
    pub fn get_peer_entry(&self, node_id: &str) -> Option<&PeerEntry> {
        self.state.peers.get(node_id)
    }
}

impl RegistryState {
    pub fn save_to_disk(&self) -> anyhow::Result<()> {
        let path = Self::state_path();

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let file = std::fs::File::create(path)?;
        let writer = std::io::BufWriter::new(file);
        serde_json::to_writer(writer, self)?;
        Ok(())
    }

    pub fn load_from_disk() -> anyhow::Result<Self> {
        let path = Self::state_path();

        if !path.exists() {
            return Ok(Self::default());
        }

        let file = std::fs::File::open(path)?;
        let reader = std::io::BufReader::new(file);
        let state = serde_json::from_reader(reader)?;
        Ok(state)
    }

    pub fn pop_failed_shard(&mut self) -> Option<ShardAssignment> {
        for (_, entry) in self.peers.iter_mut() {
            if matches!(entry.status, NodeStatus::Failed { .. }) {
                entry.status = NodeStatus::Idle;
                return Some(entry.original_shard.clone());
            }
        }
        None
    }

    pub fn state_path() -> std::path::PathBuf {
        if let Some(home) = std::env::var_os("HOME") {
            return std::path::PathBuf::from(&home).join(".drift").join("nodes.toml");
        }
        std::path::PathBuf::from("/tmp/drift-nodes.toml")
    }
}

/// Runtime registry holding both persistent and pending shard state.
#[derive(Debug, Clone)]
pub struct PeerRegistry {
    pub state: RegistryState,
    pub pending: VecDeque<ShardAssignment>,
}

impl PeerRegistry {
    pub fn new() -> Self {
        Self {
            state: RegistryState::default(),
            pending: VecDeque::new(),
        }
    }

    pub fn total_peers(&self) -> usize {
        self.state.peers.len()
    }

    pub fn add_peer_entry(&mut self, entry: PeerEntry) {
        let node_id = &entry.original_shard.node_id;
        self.state.peers.insert(node_id.clone(), entry);
    }

    pub fn handle_ask_for_more_work(&mut self, node_id: &str) -> DriftMessage {
        if !self.pending.is_empty() {
            DriftMessage::AssignNext(self.pop_pending_assignment().unwrap())
        } else if let Some(shard) = self.state.pop_failed_shard() {
            DriftMessage::AssignNext(shard)
        } else {
            DriftMessage::NoMoreWork
        }
    }

    pub fn update_on_progress(&mut self, node_id: &str, step: u64) {
        if let Some(entry) = self.state.peers.get_mut(node_id) {
            entry.last_seen = Some(time::OffsetDateTime::now_utc());
            entry.status = NodeStatus::Active;
        }
    }

    pub fn load_persistent_state() -> anyhow::Result<Self> {
        match RegistryState::load_from_disk() {
            Ok(loaded) => Ok(Self { state: loaded, pending: VecDeque::new() }),
            Err(e) => {
                tracing::warn!("could not load peer registry from disk: {}", e);
                Ok(Self::new())
            }
        }
    }

    pub fn save_to_disk(&self) -> anyhow::Result<()> {
        self.state.save_to_disk()
    }

    pub fn pop_pending_assignment(&mut self) -> Option<ShardAssignment> {
        self.pending.pop_front()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerEntry {
    /// iroh peer ID for reaching this node.
    pub did_hash_address: String,

    /// The shard originally assigned to this node at start of run.
    pub original_shard: ShardAssignment,

    /// Current status within the training run.
    pub status: NodeStatus,

    /// Last time we received any message from this node.
    #[serde(default)]
    pub last_seen: Option<time::OffsetDateTime>,
}

impl PeerEntry {
    pub fn new(did_hash_address: String, original_shard: ShardAssignment) -> Self {
        Self {
            did_hash_address,
            original_shard,
            status: NodeStatus::Active,
            last_seen: Some(time::OffsetDateTime::now_utc()),
        }
    }
}

/// Possible states for a peer's participation in training.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeStatus {
    Active,
    Idle,
    Stale { since: time::OffsetDateTime },
    Failed { unclaimed_since: time::OffsetDateTime },
    Done,
}