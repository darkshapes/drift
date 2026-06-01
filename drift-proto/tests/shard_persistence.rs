use drift_proto::{ShardAssignment, LocalShardState};
use std::fs;
use std::path::PathBuf;

const TEST_NODE_ID: &str = "test_persistence_node";

fn cleanup_test_file(path: &PathBuf) {
    let _ = fs::remove_file(path);
}

#[test]
fn test_shard_assignment_save_and_load_roundtrip() {
    let node_id = TEST_NODE_ID.to_string();
    
    let original = ShardAssignment {
        node_id: node_id.clone(),
        shard_index: 5u32,
        shard_start: 10000u64,
        shard_end: 20000u64,
    };

    let cache_path = LocalShardState::local_cache_path(&node_id);
    
    cleanup_test_file(&cache_path);

    original.save_to_disk(&node_id).unwrap();
    
    assert!(cache_path.exists(), "File should exist after save");

    let loaded = ShardAssignment::load_from_disk(&node_id).unwrap();
    
    assert!(loaded.is_some(), "Should load successfully");
    
    if let Some(state) = loaded {
        assert_eq!(state.shard_assignment.node_id, original.node_id);
        assert_eq!(state.shard_assignment.shard_index, original.shard_index);
        assert_eq!(state.shard_assignment.shard_start, original.shard_start);
        assert_eq!(state.shard_assignment.shard_end, original.shard_end);
    }

    cleanup_test_file(&cache_path);
}

#[test]
fn test_load_from_nonexistent_returns_none() {
    let result = ShardAssignment::load_from_disk("nonexistent_node_xyz");
    
    assert!(result.is_ok());
    assert!(!result.unwrap().is_some());
}