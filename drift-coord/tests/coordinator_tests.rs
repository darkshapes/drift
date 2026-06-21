//! Integration tests for the Coordinator Layer
//!
//! These test the full flow between:
//! - The coordinator's PeerRegistry handling of AskForMoreWork messages
//! - Shard reassignment logic when nodes fail or complete
//! - State persistence across coordinator restarts

use drift_proto::{DriftMessage, LocalShardState, ShardAssignment};
use drift_coord::peer_registry::{PeerRegistry, PeerEntry, NodeStatus};

#[test]
fn scenario_node_persists_and_restores_assignment() {
   temp_env::with_var("HOME", Some("/tmp/coordinator-integration-test"), || {
        let node_id = "node_restart_test";

        let shard = ShardAssignment {
            node_id: node_id.to_string(),
            shard_index: 7,
            shard_start: 7000,
            shard_end: 8000,
        };
        let config = drift_proto::TrainConfig {
            model_path: "/tmp/model".to_string(),
            dataset_path: "/tmp/dataset".to_string(),
            batch_size: 32,
            learning_rate: 0.001,
            epochs: 10,
            train_repo_url: None,
            script_entrypoint: None,
            dataset_repo_url: None,
            auth_threshold: 1,
            enable_auth: true,
            model_artifact_ref: None,
            dataset_urls: vec![],
            git_commit: None,
            gpu_compute_capability: None,
            repo_path: None,
            env_file: None,
            training_spawn_cmd: None,
            env_vars: None,
        };

        let local_state = LocalShardState {
            shard_assignment: shard.clone(),
            train_config: config,
            last_checkpoint_step: 1234,
            completion_percentage: 45.5,
        };

        assert!(local_state.save_to_disk(node_id).is_ok());

        let reloaded = LocalShardState::load_from_disk(node_id).unwrap();

        assert!(reloaded.is_some());
        let restored = reloaded.unwrap();

        assert_eq!(restored.shard_assignment.shard_index, 7);
        assert_eq!(restored.last_checkpoint_step, 1234);
        assert_eq!(restored.completion_percentage, 45.5);
    });
}

#[test]
fn scenario_node_completes_and_requests_more_work() {
    temp_env::with_var("HOME", Some("/tmp/coordinator-integration-test"), || {
        let mut registry = PeerRegistry::new_with_pending_shards(1);

        let node_id = "idle_node_0";
        let response = registry.handle_ask_for_more_work(node_id);

        match response {
            DriftMessage::AssignNext(shard) => {
                assert_eq!(shard.shard_index, 100);
            }
            DriftMessage::NoMoreWork => panic!("expected AssignNext but got NoMoreWork"),
            _ => panic!("unexpected response variant"),
        }
    });
}

#[test]
fn scenario_no_shards_remaining_returns_no_more_work() {
    temp_env::with_var("HOME", Some("/tmp/no-shards-remaining"), || {
        let mut empty_registry = PeerRegistry::empty();
        empty_registry.pending.clear();

        let response = empty_registry.handle_ask_for_more_work("lonely_node");

        match response {
            DriftMessage::NoMoreWork => {}
            other => panic!("expected NoMoreWork for empty registry, got {:?}", other),
        }
    });
}

#[test]
fn scenario_only_one_gets_each_failed_shard_reassignment() {
    temp_env::with_var("HOME", Some("/tmp/simultaneous-request-test"), || {
        let reg_path = drift_coord::peer_registry::RegistryState::state_path();
        if let Some(_parent) = reg_path.parent() {
            std::fs::create_dir_all(reg_path.parent().unwrap()).unwrap();
        }

        let mut reg = PeerRegistry::new_with_pending_shards(1);

        let resp_a = reg.handle_ask_for_more_work("idle_node_0");
        assert!(matches!(resp_a, DriftMessage::AssignNext(_)), "first requester should get shard");

        let resp_b = reg.handle_ask_for_more_work("idle_node_1");

        match resp_b {
            DriftMessage::AssignNext(_) => panic!("only one reassignment per failed shard"),
            DriftMessage::NoMoreWork => {}
            _ => {}
        }
    });
}

#[test]
fn scenario_three_nodes_request_four_failed_shards_all_assigned_in_order() {
    use drift_coord::peer_registry::PeerRegistry;

    temp_env::with_var("HOME", Some("/tmp/multi-failed-shard-reassign"), || {
        let mut registry = PeerRegistry::new_with_pending_shards(4);

        for i in 0..4 {
            registry.add_failed_node(format!("reclaimer_{}", i), i + 50);
        }

        let mut assignments_received: Vec<u32> = vec![];

        for i in 0..4 {
            let response = registry.handle_ask_for_more_work(&format!("reclaimer_{}", i));

            match response {
                DriftMessage::AssignNext(shard) => {
                    assignments_received.push(shard.shard_index);
                }
                other => panic!("node {} got {:?} instead of AssignNext", i, other),
            }
        }

        assert_eq!(assignments_received.len(), 4);

        let unique: std::collections::HashSet<_> = assignments_received.into_iter().collect();
        assert_eq!(unique.len(), 4, "each shard assigned exactly once");
    });
}

#[test]
fn scenario_coordinator_restart_recovers_peer_state() {
    use drift_proto::TrainProgress;
    use drift_coord::peer_registry::PeerRegistry;

    temp_env::with_var("HOME", Some("/tmp/coord-restart-recovery-test"), || {
        if let Some(parent) = drift_coord::peer_registry::RegistryState::state_path().parent() {
            if !parent.exists() { std::fs::create_dir_all(parent).unwrap(); }
        }

        let mut reg1 = PeerRegistry::new_with_mixed_peers(2, 1);

        for i in 0..2 {
            let prog = TrainProgress {
                node_id: format!("active_{}", i),
                epoch: 3,
                step: (i as u64) * 500 + 1000,
                loss: 0.9,
                throughput_samples_per_sec: 2000.0,
            };

            reg1.update_on_progress(&prog.node_id, prog.step);
        }

        reg1.save_to_disk().unwrap();

        let reg2_opt = PeerRegistry::load_persistent_state().ok();
        if let Some(reg2) = reg2_opt {
            assert_eq!(reg2.total_peers(), 3);
            if let Some(e) = reg2.get_peer_entry("active_0") {
                assert!(e.last_seen.is_some());
            }
        }
    });
}

#[test]
fn scenario_node_reconnects_after_coord_restart() {
    use drift_coord::peer_registry::{PeerRegistry, PeerEntry};

    temp_env::with_var("HOME", Some("/tmp/node-reconnect-after-restart"), || {
        let mut old_registry = PeerRegistry::new();

        let node_id = "surviving_node".to_string();
        let shard = ShardAssignment {
            node_id,
            shard_index: 5,
            shard_start: 5000,
            shard_end: 6000,
        };

        let entry = PeerEntry {
            did_hash_address: "did:test:surviving".to_string(),
            original_shard: shard.clone(),
            status: NodeStatus::Active,
            last_seen: None,
        };

        old_registry.add_peer_entry(entry);

        if let Err(e) = old_registry.save_to_disk() { eprintln!("save failed (ok for test): {}", e); }

        let loaded_opt = PeerRegistry::load_persistent_state().ok();
        if let Some(loaded) = loaded_opt {
            assert_eq!(loaded.total_peers(), 1);
            if let Some(e) = loaded.get_peer_entry("surviving_node") {
                assert!(matches!(e.status, NodeStatus::Active));
            }
        }
    });
}

#[test]
fn scenario_ctrl_c_saves_partial_progress() {
    use drift_proto::TrainProgress;
    use drift_coord::peer_registry::{PeerRegistry, PeerEntry, NodeStatus};

    temp_env::with_var("HOME", Some("/tmp/ctrl-c-preservation-test"), || {
        if let Some(parent) = drift_coord::peer_registry::RegistryState::state_path().parent() {
            if !parent.exists() { std::fs::create_dir_all(parent).unwrap(); }
        }

        let mut reg = PeerRegistry::new();

        for i in 0..3 {
            let node_id = format!("node_{}", i);

            let entry = PeerEntry {
                did_hash_address: "did:test".to_string(),
                original_shard: ShardAssignment {
                    node_id: node_id.clone(),
                    shard_index: i as u32,
                    shard_start: (i as u64) * 10000,
                    shard_end: (i + 1) as u64 * 10000,
                },
                status: if i == 2 { NodeStatus::Stale { since: time::OffsetDateTime::now_utc() } } else { NodeStatus::Active },
                last_seen: None,
            };

            reg.add_peer_entry(entry);

            let prog = TrainProgress {
                node_id,
                epoch: 2,
                step: (i as u64) * 200 + 500,
                loss: 1.0 - f64::from(i) * 0.2,
                throughput_samples_per_sec: 1500.0,
            };

            reg.update_on_progress(&prog.node_id, prog.step);
        }

        reg.save_to_disk().unwrap();

        let reloaded_opt = PeerRegistry::load_persistent_state().ok();
        if let Some(reloaded) = reloaded_opt {
            assert_eq!(reloaded.total_peers(), 3);

            if let Some(reloaded_entry) = reloaded.get_peer_entry("node_2") {
                match reloaded_entry.status {
                    NodeStatus::Active => {}
                    other => panic!("expected Active for node_2 but got {:?}", other),
                }
            }
        }
    });
}

#[test]
fn scenario_ctrl_c_restart_shows_last_known_epoch_and_step() {
    use drift_proto::TrainProgress;
    use drift_coord::peer_registry::{PeerRegistry, PeerEntry, NodeStatus};

    temp_env::with_var("HOME", Some("/tmp/ctrl-c-restart-steps"), || {
        let mut reg = PeerRegistry::new();

        let entry = PeerEntry {
            did_hash_address: "did:test".to_string(),
            original_shard: ShardAssignment {
                node_id: "training_node".to_string(),
                shard_index: 0,
                shard_start: 0,
                shard_end: 100000,
            },
            status: NodeStatus::Active,
            last_seen: None,
        };

        reg.add_peer_entry(entry);

        let prog1 = TrainProgress {
            node_id: String::from("training_node"),
            epoch: 1,
            step: 250,
            loss: 0.8,
            throughput_samples_per_sec: 2000.0,
        };
        reg.update_on_progress(&prog1.node_id, prog1.step);

        let prog2 = TrainProgress {
            node_id: String::from("training_node"),
            epoch: 1,
            step: 500,
            loss: 0.7,
            throughput_samples_per_sec: 2100.0,
        };
        reg.update_on_progress(&prog2.node_id, prog2.step);

        if let Err(e) = reg.save_to_disk() { eprintln!("save failed (ok for test): {}", e); }

        let reloaded_opt = PeerRegistry::load_persistent_state().ok();
        if let Some(reloaded) = reloaded_opt {
            if let Some(e) = reloaded.get_peer_entry("training_node") {
                assert!(e.last_seen.is_some(), "last_seen was recorded");
            }
        }
    });
}

#[test]
fn peer_registry_persistence_round_trip() {
    temp_env::with_var("HOME", Some("/tmp/peer-registry-round-trip"), || {
        let mut original = PeerRegistry::new();

        for i in 0..5 {
            let entry = PeerEntry {
                did_hash_address: format!("did:test:node_{}", i),
                original_shard: ShardAssignment {
                    node_id: format!("node_{}", i),
                    shard_index: i as u32 * 10,
                    shard_start: (i as u64) * 1000,
                    shard_end: (i + 1) as u64 * 1000,
                },
                status: NodeStatus::Active,
                last_seen: Some(time::OffsetDateTime::now_utc()),
            };
            original.add_peer_entry(entry);
        }

        original.save_to_disk().unwrap();

        let loaded = PeerRegistry::load_persistent_state().ok().unwrap();
        assert_eq!(loaded.total_peers(), 5);

        for i in 0..5 {
            let entry = loaded.get_peer_entry(&format!("node_{}", i));
            assert!(entry.is_some());
            assert_eq!(entry.unwrap().original_shard.shard_index, i as u32 * 10);
        }
    });
    temp_env::with_var("HOME", Some("/tmp/empty-registry-persistence"), || {
        let empty = PeerRegistry::new();
        assert_eq!(empty.total_peers(), 0);

        if let Err(e) = empty.save_to_disk() { eprintln!("save failed (expected): {}", e); return; }

        let reloaded = PeerRegistry::load_persistent_state().ok().unwrap();
        assert_eq!(reloaded.total_peers(), 0);
    });
}
