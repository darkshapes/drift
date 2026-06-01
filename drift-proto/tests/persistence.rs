use drift_proto::{LocalShardState, CoordEndpointCache, TrainConfig, ShardAssignment};

#[test]
fn test_local_shard_state_creation() {
    let shard_assignment = ShardAssignment::default();
    let train_config = TrainConfig::default();

    let state = LocalShardState {
        shard_assignment,
        train_config,
        last_checkpoint_step: 42u64,
        completion_percentage: 0.85f32,
    };

    assert_eq!(state.last_checkpoint_step, 42u64);
    assert_eq!(state.completion_percentage, 0.85f32);
}

#[test]
fn test_local_shard_state_local_cache_path() {
    let path_str = LocalShardState::local_cache_path("node123").display().to_string();
    
    assert!(path_str.contains("node123"), "Path should contain node_id");
    assert!(path_str.contains(".drift"), "Path should be in .drift directory");
}

#[test]
fn test_coord_endpoint_cache_creation() {
    let cache = CoordEndpointCache {
        did_hash_address: "did:test123".to_string(),
        public_key_or_secret: Some("secret456".to_string()),
    };

    assert_eq!(cache.did_hash_address, "did:test123");
    assert_eq!(cache.public_key_or_secret, Some("secret456".to_string()));
}

#[test]
fn test_coord_endpoint_cache_cache_path() {
    let path_str = CoordEndpointCache::cache_path().display().to_string();
    
    assert!(path_str.contains("coordinator.toml"), "Path should end with coordinator.toml");
}