use drift_proto::{LocalShardState, ShardAssignment, TrainConfig};
use std::fs;
use std::path::PathBuf;

fn cleanup_test_file(path: &PathBuf) {
    let _ = fs::remove_file(path);
}

#[test]
fn test_load_from_disk_nonexistent_returns_none() {
    let result = LocalShardState::load_from_disk("nonexistent_node_xyz");
    assert!(result.is_ok());
    assert!(!result.unwrap().is_some());
}

#[test]
fn test_load_from_disk_existing_returns_state() {
    let node_id = "test_startup_load_existing".to_string();
    let cache_path = LocalShardState::local_cache_path(&node_id);
    cleanup_test_file(&cache_path);

    let original = LocalShardState {
        shard_assignment: ShardAssignment {
            node_id: node_id.clone(),
            shard_index: 5u32,
            shard_start: 10000u64,
            shard_end: 20000u64,
        },
        train_config: TrainConfig::default(),
        last_checkpoint_step: 42u64,
        completion_percentage: 0.85f32,
    };

    original.save_to_disk(&node_id).unwrap();
    let loaded = LocalShardState::load_from_disk(&node_id).unwrap();

    assert!(loaded.is_some());
    let state = loaded.unwrap();
    assert_eq!(state.last_checkpoint_step, 42u64);
    assert_eq!(state.completion_percentage, 0.85f32);

    cleanup_test_file(&cache_path);
}

#[test]
fn test_local_cache_path_contains_node_id() {
    let path = LocalShardState::local_cache_path("test_node_123");
    let path_str = path.display().to_string();
    assert!(path_str.contains("test_node_123"));
    assert!(path_str.contains(".drift"));
}

#[test]
fn test_resume_vs_reassign_decision_resume_when_cache_valid() {
    let node_id = "test_startup_resume_valid".to_string();
    let cache_path = LocalShardState::local_cache_path(&node_id);
    cleanup_test_file(&cache_path);

    let state = LocalShardState {
        shard_assignment: ShardAssignment {
            node_id: node_id.clone(),
            shard_index: 0u32,
            shard_start: 0u64,
            shard_end: 1000u64,
        },
        train_config: TrainConfig::default(),
        last_checkpoint_step: 500u64,
        completion_percentage: 0.5f32,
    };

    state.save_to_disk(&node_id).unwrap();
    let loaded = LocalShardState::load_from_disk(&node_id).unwrap();

    assert!(loaded.is_some());
    let cached = loaded.unwrap();

    let should_resume = cached.last_checkpoint_step > 0;
    assert!(should_resume, "Should resume when checkpoint step > 0");

    cleanup_test_file(&cache_path);
}

#[test]
fn test_resume_vs_reassign_decision_reassign_when_cache_corrupt() {
    let node_id = "test_startup_reassign_corrupt".to_string();
    let cache_path = LocalShardState::local_cache_path(&node_id);
    cleanup_test_file(&cache_path);

    if let Some(parent) = cache_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(&cache_path, "invalid json{{{").unwrap();

    let result = LocalShardState::load_from_disk(&node_id);
    assert!(result.is_err(), "Should error on corrupt cache");

    cleanup_test_file(&cache_path);
}