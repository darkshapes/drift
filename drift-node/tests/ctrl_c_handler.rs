use drift_proto::{LocalShardState, ShardAssignment, TrainConfig};
use std::path::PathBuf;
use std::fs;

fn cleanup(path: &PathBuf) {
    let _ = fs::remove_file(path);
}

#[test]
fn test_preserve_model_artifact_in_progress() {
    let node_id = "test_ctrlc_preserve_model".to_string();
    let cache_path = LocalShardState::local_cache_path(&node_id);
    cleanup(&cache_path);

    let mut config = TrainConfig::default();
    config.model_artifact = Some("hf://model".to_string());

    let state = LocalShardState {
        shard_assignment: ShardAssignment {
            node_id: node_id.clone(),
            shard_index: 0u32,
            shard_start: 0u64,
            shard_end: 1000u64,
        },
        train_config: config,
        last_checkpoint_step: 950u64,
        completion_percentage: 0.95f32,
    };

    state.save_to_disk(&node_id).unwrap();

    let loaded = LocalShardState::load_from_disk(&node_id).unwrap();
    assert!(loaded.is_some());

    let restored = loaded.unwrap();
    assert!(restored.completion_percentage > 0.9);

    cleanup(&cache_path);
}

#[test]
fn test_no_op_when_no_cached_state() {
    let node_id = "interrupt_no_cache_node";
    let result = LocalShardState::load_from_disk(node_id);
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

#[test]
fn test_force_flush_pending_writes() {
    let node_id = "test_ctrlc_flush_writes".to_string();
    let cache_path = LocalShardState::local_cache_path(&node_id);
    cleanup(&cache_path);

    let state = LocalShardState {
        shard_assignment: ShardAssignment {
            node_id: node_id.clone(),
            shard_index: 2u32,
            shard_start: 10000u64,
            shard_end: 20000u64,
        },
        train_config: TrainConfig::default(),
        last_checkpoint_step: 15000u64,
        completion_percentage: 0.75f32,
    };

    state.save_to_disk(&node_id).unwrap();
    assert!(cache_path.exists());

    cleanup(&cache_path);
}

#[test]
fn test_preserve_full_state_on_interrupt() {
    let node_id = "test_ctrlc_preserve_full".to_string();
    let cache_path = LocalShardState::local_cache_path(&node_id);
    cleanup(&cache_path);

    let mut config = TrainConfig::default();
    config.model_artifact = Some("/tmp/model.pt".to_string());
    config.repo_hash = Some("abc123".to_string());
    config.dataset_urls = vec!["/tmp/dataset".to_string()];

    let state = LocalShardState {
        shard_assignment: ShardAssignment {
            node_id: node_id.clone(),
            shard_index: 5u32,
            shard_start: 25000u64,
            shard_end: 50000u64,
        },
        train_config: config,
        last_checkpoint_step: 37500u64,
        completion_percentage: 0.75f32,
    };

    state.save_to_disk(&node_id).unwrap();

    let loaded = LocalShardState::load_from_disk(&node_id).unwrap();
    assert!(loaded.is_some());

    let restored = loaded.unwrap();
    assert_eq!(restored.train_config.model_artifact, Some("/tmp/model.pt".to_string()));
    assert_eq!(restored.train_config.repo_hash, Some("abc123".to_string()));
    assert_eq!(restored.train_config.dataset_urls.len(), 1);
    assert_eq!(restored.last_checkpoint_step, 37500u64);

    cleanup(&cache_path);
}
