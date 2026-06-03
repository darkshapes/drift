use drift_proto::{CoordState, SlotState, NodeSlot, NodeInfo, ShardAssignment};

#[test]
fn test_coord_state_default() {
    let state: CoordState = CoordState::default();
    assert_eq!(state.pending.len(), 0);
    assert_eq!(state.slots.len(), 0);
    assert!(!state.has_in_flight_nodes());
}

#[test]
fn test_slot_state_helpers() {
    use std::time::Instant;

    let available = SlotState::Available { since: Instant::now() };
    let in_flight = SlotState::InFlight { last_heartbeat: Instant::now() };

    assert!(available.is_available());
    assert!(!available.is_in_flight());

    assert!(in_flight.is_in_flight());
    assert!(!in_flight.is_available());
}

#[test]
fn test_node_slot_creation() {
    use std::time::Instant;

    let node_id = "node_abc123".to_string();
    let slot = NodeSlot {
        node_id,
        state: SlotState::Available { since: Instant::now() },
    };

    assert_eq!(slot.node_id, "node_abc123");
    assert!(matches!(slot.state, SlotState::Available { .. }));
}

#[test]
fn test_shard_assignment_size() {
    let shard = ShardAssignment {
        node_id: "node_x".to_string(),
        shard_index: 2u32,
        shard_start: 5000u64,
        shard_end: 15000u64,
    };

    assert_eq!(shard.size(), 10000u64);
}

#[test]
fn test_coord_state_with_initial_nodes() {
    let nodes = [
        NodeInfo {
            node_id: "alpha".to_string(),
            gpu_name: "GPU A".to_string(),
            gpu_vram_mb: 8192u64,
            gpu_compute_capability: "8.6".to_string(),
            available: true,
        },
        NodeInfo {
            node_id: "beta".to_string(),
            gpu_name: "GPU B".to_string(),
            gpu_vram_mb: 16384u64,
            gpu_compute_capability: "8.9".to_string(),
            available: true,
        },
    ];

    let state = CoordState::new(&nodes);

    assert_eq!(state.slots.len(), 2);
    assert_eq!(state.pending.len(), 1);
}

#[test]
fn test_mark_completed_transitions_in_flight_to_available() {
    use std::time::Instant;

    let mut state = CoordState::default();
    state.register_node(&NodeInfo {
        node_id: "gamma".to_string(),
        gpu_name: "Test".to_string(),
        gpu_vram_mb: 4096u64,
        gpu_compute_capability: "8.0".to_string(),
        available: false,
    });

    if let Some(slot) = state.slots.get_mut("gamma") {
        slot.state = SlotState::InFlight { last_heartbeat: Instant::now() };
    }

    let result = state.mark_completed("gamma");
    assert!(result);

    if let Some(slot) = state.slots.get("gamma") {
        assert!(matches!(slot.state, SlotState::Available { .. }));
    }
}

#[test]
fn test_mark_completed_returns_false_when_not_in_flight() {
    let mut state = CoordState::default();
    state.register_node(&NodeInfo {
        node_id: "delta".to_string(),
        gpu_name: "GPU D".to_string(),
        gpu_vram_mb: 8192u64,
        gpu_compute_capability: "8.9".to_string(),
        available: true,
    });

    let result = state.mark_completed("delta");
    assert!(!result);
}

#[cfg(test)]
mod auth_integration_tests {
    use drift_auth::{AuthMessage, SignedAuthMessage, AggregateAuthMessage};
    use drift_proto::{DriftMessage, TrainConfig};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn test_train_config_auth_fields_default() {
        let config = TrainConfig::default();
        assert!(!config.enable_auth);
        assert_eq!(config.auth_threshold, 0);
    }

    #[test]
    fn test_train_config_auth_fields_set() {
        let config = TrainConfig {
            model_path: "/model".to_string(),
            dataset_path: "/data".to_string(),
            batch_size: 32u32,
            learning_rate: 0.001,
            epochs: 100u32,
            train_repo_url: None,
            script_entrypoint: None,
            dataset_repo_url: None,
            model_artifact_ref: None,
            enable_auth: true,
            auth_threshold: 5usize,
            git_commit: None,
        };
        assert!(config.enable_auth);
        assert_eq!(config.auth_threshold, 5);
    }

    #[test]
    fn test_train_config_auth_serialization() {
        let config = TrainConfig {
            model_path: "/model".to_string(),
            dataset_path: "/data".to_string(),
            batch_size: 32u32,
            learning_rate: 0.001,
            epochs: 100u32,
            train_repo_url: None,
            script_entrypoint: None,
            dataset_repo_url: None,
            model_artifact_ref: None,
            enable_auth: true,
            auth_threshold: 3usize,
            git_commit: None,
        };
        let json = serde_json::to_string(&config).unwrap();
        let parsed: TrainConfig = serde_json::from_str(&json).unwrap();
        assert!(parsed.enable_auth);
        assert_eq!(parsed.auth_threshold, 3);
    }

    #[test]
    fn test_drift_message_auth_challenge_variant() {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let auth_msg = AuthMessage::with_values("node1", "abc123", now, 42u64, 1u64);
        let msg = DriftMessage::AuthChallenge(auth_msg);
        match msg {
            DriftMessage::AuthChallenge(inner) => {
                assert_eq!(inner.node_id, "node1");
                assert_eq!(inner.repo_hash, "abc123");
                assert_eq!(inner.sequence, 1u64);
            }
            _ => panic!("expected AuthChallenge variant"),
        }
    }

    #[test]
    fn test_drift_message_auth_response_variant() {
        use ed25519_dalek::Signer;
        use rand::rngs::OsRng;

        let mut rng = OsRng;
        let keypair = ed25519_dalek::SigningKey::generate(&mut rng);
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let auth_msg = AuthMessage::with_values("node1", "abc123", now, 42u64, 1u64);
        let signed = SignedAuthMessage::sign(&auth_msg, &keypair).unwrap();

        let pk = iroh::PublicKey::from_bytes(keypair.verifying_key().as_bytes()).unwrap();
        let msg = DriftMessage::AuthResponse(signed);
        match msg {
            DriftMessage::AuthResponse(inner) => {
                assert_eq!(inner.node_id, "node1");
                assert!(inner.verify(&pk).is_ok());
            }
            _ => panic!("expected AuthResponse variant"),
        }
    }

    #[test]
    fn test_drift_message_auth_aggregate_variant() {
        use ed25519_dalek::Signer;
        use rand::rngs::OsRng;

        let mut rng = OsRng;
        let keypair = ed25519_dalek::SigningKey::generate(&mut rng);
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let auth_msg = AuthMessage::with_values("node1", "abc123", now, 42u64, 1u64);

        let signed = SignedAuthMessage::sign(&auth_msg, &keypair).unwrap();
        let aggregate = AggregateAuthMessage::create(vec![signed], 1, 1).unwrap();

        let msg = DriftMessage::AuthAggregate(aggregate);
        match msg {
            DriftMessage::AuthAggregate(inner) => {
                assert_eq!(inner.threshold, 1);
                assert_eq!(inner.total_nodes, 1);
                assert_eq!(inner.node_ids.len(), 1);
            }
            _ => panic!("expected AuthAggregate variant"),
        }
    }

    #[test]
    fn test_drift_message_auth_challenge_serialization() {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let auth_msg = AuthMessage::with_values("node1", "abc123", now, 42u64, 1u64);
        let msg = DriftMessage::AuthChallenge(auth_msg);

        let bytes = serde_json::to_vec(&msg).unwrap();
        let parsed: DriftMessage = serde_json::from_slice(&bytes).unwrap();

        match parsed {
            DriftMessage::AuthChallenge(inner) => {
                assert_eq!(inner.node_id, "node1");
                assert_eq!(inner.repo_hash, "abc123");
            }
            _ => panic!("expected AuthChallenge after round-trip"),
        }
    }

    #[test]
    fn test_auth_message_with_auth_config_disabled() {
        let config = TrainConfig::default();
        assert!(!config.enable_auth);
    }

    #[test]
    fn test_auth_message_with_auth_config_threshold_defaults() {
        let config = TrainConfig::default();
        assert_eq!(config.auth_threshold, 0);
    }

    #[test]
    fn test_auth_handshake_flow_auth_disabled() {
      let config = TrainConfig {
            model_path: "/model".to_string(),
            dataset_path: "/data".to_string(),
            batch_size: 32u32,
            learning_rate: 0.001,
            epochs: 100u32,
            train_repo_url: None,
            script_entrypoint: None,
            dataset_repo_url: None,
            model_artifact_ref: None,
            enable_auth: true,
            auth_threshold: 3usize,
            git_commit: None,
        };
        assert!(!config.enable_auth);
    }

    #[test]
    fn test_auth_handshake_flow_auth_enabled_with_threshold() {
        let config = TrainConfig {
            model_path: "/model".to_string(),
            dataset_path: "/data".to_string(),
            batch_size: 32u32,
            learning_rate: 0.001,
            epochs: 100u32,
            train_repo_url: None,
            script_entrypoint: None,
            dataset_repo_url: None,
            model_artifact_ref: None,
            enable_auth: true,
            auth_threshold: 3usize,
            git_commit: None,
        };
        assert!(config.enable_auth);
        assert_eq!(config.auth_threshold, 3);
    }

    #[test]
    fn test_auth_handshake_full_flow() {
        use ed25519_dalek::Signer;
        use rand::rngs::OsRng;

        let mut rng = OsRng;
        let keypair = ed25519_dalek::SigningKey::generate(&mut rng);
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

        let auth_msg = AuthMessage::with_values("node1", "repo123", now, 100u64, 5u64);
        let challenge = DriftMessage::AuthChallenge(auth_msg);

        let mut signed_auth_msg: Option<SignedAuthMessage> = None;
        match challenge {
            DriftMessage::AuthChallenge(inner) => {
                assert!(inner.is_timestamp_valid(300));
                let signed = SignedAuthMessage::sign(&inner, &keypair).unwrap();
                signed_auth_msg = Some(signed);
            }
            _ => panic!("expected AuthChallenge"),
        }

        let response = DriftMessage::AuthResponse(signed_auth_msg.unwrap());
        let mut aggregate_opt: Option<AggregateAuthMessage> = None;
        match response {
            DriftMessage::AuthResponse(signed) => {
                let agg = AggregateAuthMessage::create(vec![signed], 1, 1).unwrap();
                aggregate_opt = Some(agg);
            }
            _ => panic!("expected AuthResponse"),
        }

        let final_msg = DriftMessage::AuthAggregate(aggregate_opt.unwrap());
        let pk = iroh::PublicKey::from_bytes(keypair.verifying_key().as_bytes()).unwrap();
        match final_msg {
            DriftMessage::AuthAggregate(agg) => {
                assert_eq!(agg.node_ids.len(), 1);
                assert_eq!(agg.threshold, 1);
                assert!(agg.verify(&[pk]).is_ok());
            }
            _ => panic!("expected AuthAggregate"),
        }
    }
}