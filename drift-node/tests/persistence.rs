use drift_proto::{LocalShardState, ShardAssignment, TrainConfig};
use std::fs;
use std::path::PathBuf;

fn cleanup(path: &PathBuf) {
    let _ = fs::remove_file(path);
}

#[test]
fn test_node_restart_reloads_same_assignment() {
    let node_id = "node_integ_restart_a".to_string();
    let cache_path = LocalShardState::local_cache_path(&node_id);
    cleanup(&cache_path);

    let original = ShardAssignment {
        node_id: node_id.clone(),
        shard_index: 7u32,
        shard_start: 35000u64,
        shard_end: 70000u64,
    };

    original.save_to_disk(&node_id).unwrap();

    let state = LocalShardState::load_from_disk(&node_id).unwrap();
    assert!(state.is_some());

    let loaded = state.unwrap();
    assert_eq!(loaded.shard_assignment.shard_index, 7u32);
    assert_eq!(loaded.shard_assignment.shard_start, 35000u64);
    assert_eq!(loaded.shard_assignment.shard_end, 70000u64);

    cleanup(&cache_path);
}

#[test]
fn test_node_restart_reloads_checkpoint_progress() {
    let node_id = "node_integ_restart_b".to_string();
    let cache_path = LocalShardState::local_cache_path(&node_id);
    cleanup(&cache_path);

    let mut config = TrainConfig::default();
    config.epochs = 5u32;
    config.batch_size = 64u32;

    let state = LocalShardState {
        shard_assignment: ShardAssignment {
            node_id: node_id.clone(),
            shard_index: 0u32,
            shard_start: 0u64,
            shard_end: 1000u64,
        },
        train_config: config,
        last_checkpoint_step: 750u64,
        completion_percentage: 0.75f32,
    };

    state.save_to_disk(&node_id).unwrap();

    let loaded = LocalShardState::load_from_disk(&node_id).unwrap();
    assert!(loaded.is_some());

    let restored = loaded.unwrap();
    assert_eq!(restored.last_checkpoint_step, 750u64);
    assert_eq!(restored.completion_percentage, 0.75f32);
    assert_eq!(restored.train_config.batch_size, 64u32);

    cleanup(&cache_path);
}

#[test]
fn test_multiple_nodes_save_independent_state() {
    let node_a = "node_integ_multi_a".to_string();
    let node_b = "node_integ_multi_b".to_string();
    let cache_a = LocalShardState::local_cache_path(&node_a);
    let cache_b = LocalShardState::local_cache_path(&node_b);
    cleanup(&cache_a);
    cleanup(&cache_b);

    let shard_a = ShardAssignment {
        node_id: node_a.clone(),
        shard_index: 0u32,
        shard_start: 0u64,
        shard_end: 1000u64,
    };

    let shard_b = ShardAssignment {
        node_id: node_b.clone(),
        shard_index: 1u32,
        shard_start: 1000u64,
        shard_end: 2000u64,
    };

    shard_a.save_to_disk(&node_a).unwrap();
    shard_b.save_to_disk(&node_b).unwrap();

    let loaded_a = LocalShardState::load_from_disk(&node_a).unwrap();
    let loaded_b = LocalShardState::load_from_disk(&node_b).unwrap();

    assert!(loaded_a.is_some());
    assert!(loaded_b.is_some());

    assert_eq!(loaded_a.unwrap().shard_assignment.shard_index, 0u32);
    assert_eq!(loaded_b.unwrap().shard_assignment.shard_index, 1u32);

    cleanup(&cache_a);
    cleanup(&cache_b);
}

#[test]
fn test_overwrite_cleans_old_state() {
    let node_id = "node_integ_overwrite".to_string();
    let cache_path = LocalShardState::local_cache_path(&node_id);
    cleanup(&cache_path);

    let first = LocalShardState {
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
    first.save_to_disk(&node_id).unwrap();

    let second = LocalShardState {
        shard_assignment: ShardAssignment {
            node_id: node_id.clone(),
            shard_index: 1u32,
            shard_start: 1000u64,
            shard_end: 2000u64,
        },
        train_config: TrainConfig::default(),
        last_checkpoint_step: 1500u64,
        completion_percentage: 0.75f32,
    };
    second.save_to_disk(&node_id).unwrap();

    let loaded = LocalShardState::load_from_disk(&node_id).unwrap();
    assert!(loaded.is_some());

    let state = loaded.unwrap();
    assert_eq!(state.shard_assignment.shard_index, 1u32);
    assert_eq!(state.last_checkpoint_step, 1500u64);

    cleanup(&cache_path);
}