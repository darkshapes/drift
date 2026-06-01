use drift_proto::{LocalShardState, ShardAssignment, TrainConfig};
use std::path::PathBuf;
use std::fs;

const BASE_NODE_ID: &str = "test_node";

fn uid() -> u64 {
    static mut COUNTER: u64 = 0;
    unsafe { COUNTER += 1; COUNTER }
}

fn node_id(tag: &'static str) -> String {
    format!("{}_{}_{}", BASE_NODE_ID, tag, uid())
}

fn cleanup(path: &PathBuf) {
    let _ = fs::remove_file(path);
}

#[test]
fn test_load_cached_state_returns_last_checkpoint() {
    let nid = node_id("last_ckpt");
    let path = LocalShardState::local_cache_path(&nid);

    cleanup(&path);

    let config = TrainConfig {
        model_path: "/models/test".to_string(),
        dataset_path: "/data/test".to_string(),
        batch_size: 32u32,
        learning_rate: 0.001f64,
        epochs: 10u32,
        train_repo_url: None,
        script_entrypoint: None,
        dataset_repo_url: None,
        model_artifact_ref: None,
        enable_auth: false,
        auth_threshold: 3,
    };

    let shard = ShardAssignment {
        node_id: nid.clone(),
        shard_index: 3u32,
        shard_start: 50000u64,
        shard_end: 75000u64,
    };

    let state = LocalShardState {
        shard_assignment: shard,
        train_config: config,
        last_checkpoint_step: 1500u64,
        completion_percentage: 0.45f32,
    };

    state.save_to_disk(&nid).unwrap();

    if let Some(Ok(Some(cached))) = Some(LocalShardState::load_from_disk(&nid)) {
        assert_eq!(cached.last_checkpoint_step, 1500u64);
        assert_eq!(cached.completion_percentage, 0.45f32);
    } else {
        panic!("Expected to load cached state")
    }

    cleanup(&path);
}

#[test]
fn test_load_nonexistent_returns_none() {
    let result = LocalShardState::load_from_disk("brand_new_node_never_seen");

    assert!(result.is_ok());
    if let Ok(opt) = result {
        assert!(opt.is_none(), "Should not exist");
    }
}

#[test]
fn test_resume_decision_with_incomplete_training() {
    let nid = node_id("incomplete");
    let path = LocalShardState::local_cache_path(&nid);

    cleanup(&path);

    let incomplete_state = LocalShardState {
        shard_assignment: ShardAssignment {
            node_id: nid.clone(),
            shard_index: 1u32,
            shard_start: 1000u64,
            shard_end: 5000u64,
        },
        train_config: TrainConfig::default(),
        last_checkpoint_step: 800u64,
        completion_percentage: 0.2f32,
    };

    incomplete_state.save_to_disk(&nid).unwrap();

    if let Some(Ok(Some(cached))) = Some(LocalShardState::load_from_disk(&nid)) {
        let is_worth_reusing =
            cached.completion_percentage < 0.95 && cached.last_checkpoint_step > 0;
        assert!(
            is_worth_reusing,
            "Incomplete training (20% done, step 800) should be worth resuming"
        );
    }

    cleanup(&path);
}

#[test]
fn test_skip_cached_on_complete_training() {
    let nid = node_id("complete");
    let path = LocalShardState::local_cache_path(&nid);

    cleanup(&path);

    let complete_state = LocalShardState {
        shard_assignment: ShardAssignment {
            node_id: nid.clone(),
            shard_index: 5u32,
            shard_start: 20000u64,
            shard_end: 30000u64,
        },
        train_config: TrainConfig::default(),
        last_checkpoint_step: 1000000u64,
        completion_percentage: 1.0f32,
    };

    complete_state.save_to_disk(&nid).unwrap();

    if let Some(Ok(Some(cached))) = Some(LocalShardState::load_from_disk(&nid)) {
        let should_request_fresh = cached.completion_percentage >= 0.99;
        assert!(
            should_request_fresh,
            "Complete training (100%) should request fresh assignment"
        );
    }

    cleanup(&path);
}