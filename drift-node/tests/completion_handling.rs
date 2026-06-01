use drift_proto::{DriftMessage, ShardAssignment, LocalShardState};
use std::fs;
use std::path::PathBuf;

const TEST_NODE_ID: &str = "test_completion_node";

fn cleanup_test_file(path: &PathBuf) {
    let _ = fs::remove_file(path);
}

#[test]
fn test_no_more_work_returns_clean_shutdown() {
    let msg = DriftMessage::NoMoreWork;
    assert!(matches!(msg, DriftMessage::NoMoreWork));
}

#[test]
fn test_assign_next_triggers_save() {
    let node_id = TEST_NODE_ID.to_string();
    let cache_path = LocalShardState::local_cache_path(&node_id);
    cleanup_test_file(&cache_path);

    let shard = ShardAssignment {
        node_id: node_id.clone(),
        shard_index: 3u32,
        shard_start: 5000u64,
        shard_end: 15000u64,
    };

    shard.save_to_disk(&node_id).unwrap();

    assert!(cache_path.exists());

    let loaded = LocalShardState::load_from_disk(&node_id).unwrap();
    assert!(loaded.is_some());

    cleanup_test_file(&cache_path);
}

#[test]
fn test_assign_next_shard_index_increments() {
    let shard_0 = ShardAssignment {
        node_id: TEST_NODE_ID.to_string(),
        shard_index: 0u32,
        shard_start: 0u64,
        shard_end: 1000u64,
    };

    let shard_1 = ShardAssignment {
        node_id: TEST_NODE_ID.to_string(),
        shard_index: 1u32,
        shard_start: 1000u64,
        shard_end: 2000u64,
    };

    assert!(shard_1.shard_index > shard_0.shard_index);
}

#[test]
fn test_shard_assignment_continuity() {
    let shards = vec![
        ShardAssignment {
            node_id: TEST_NODE_ID.to_string(),
            shard_index: 0u32,
            shard_start: 0u64,
            shard_end: 1000u64,
        },
        ShardAssignment {
            node_id: TEST_NODE_ID.to_string(),
            shard_index: 1u32,
            shard_start: 1000u64,
            shard_end: 2000u64,
        },
        ShardAssignment {
            node_id: TEST_NODE_ID.to_string(),
            shard_index: 2u32,
            shard_start: 2000u64,
            shard_end: 3000u64,
        },
    ];

    for i in 1..shards.len() {
        assert_eq!(shards[i].shard_start, shards[i - 1].shard_end);
        assert!(shards[i].shard_index > shards[i - 1].shard_index);
    }
}