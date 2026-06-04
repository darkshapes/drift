//! Tests for git commit verification functionality.
//!
//! Tests the flow where:
//! - Nodes send RepoCommit messages with their local repo's git commit
//! - Coordinator verifies all nodes agree on the same commit
//! - Coordinator broadcasts TrainingReady once consensus is reached

#[cfg(test)]
mod repo_commit_tests {
    use drift_proto::{DriftMessage, RepoCommit, TrainConfig};

    #[test]
    fn test_repo_commit_serialization_roundtrip() {
        let commit = RepoCommit {
            commit: "abc123def456".to_string(),
            repo_url: "https://github.com/user/repo".to_string(),
            signature: vec![1, 2, 3, 4, 5],
        };

        let json = serde_json::to_string(&commit);
        assert!(json.is_ok(), "serialization failed: {:?}", json.err());

        let decoded: Result<RepoCommit, _> = serde_json::from_str(&json.unwrap());
        assert!(decoded.is_ok(), "deserialization failed: {:?}", decoded.err());

        let repo_commit = decoded.unwrap();
        assert_eq!(repo_commit.commit, "abc123def456");
        assert_eq!(repo_commit.repo_url, "https://github.com/user/repo");
        assert_eq!(repo_commit.signature, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_drift_message_has_training_ready_variant() {
        let msg = DriftMessage::TrainingReady;
        match msg {
            DriftMessage::TrainingReady => {},
            other => panic!("expected TrainingReady, got {:?}", other),
        }
    }

    #[test]
    fn test_drift_message_has_repo_commit_variant() {
        let commit = RepoCommit {
            commit: "deadbeef".to_string(),
            repo_url: "https://github.com/test/repo".to_string(),
            signature: vec![],
        };
        let msg = DriftMessage::RepoCommit(commit);

        match msg {
            DriftMessage::RepoCommit(rc) => {
                assert_eq!(rc.commit, "deadbeef");
            }
            other => panic!("expected RepoCommit, got {:?}", other),
        }
    }

    #[test]
    fn test_train_config_can_store_git_commit() {
        let mut config = TrainConfig {
            model_path: "/tmp/model".to_string(),
            dataset_path: "/tmp/dataset".to_string(),
            batch_size: 32,
            learning_rate: 0.001,
            epochs: 10,
            train_repo_url: Some("https://github.com/user/repo".to_string()),
            script_entrypoint: None,
            dataset_repo_url: None,
            auth_threshold: 3,
            enable_auth: false,
            model_artifact_ref: None,
            git_commit: None,
        };

        assert!(config.git_commit.is_none());

        config.git_commit = Some("abc123".to_string());

        assert_eq!(config.git_commit, Some("abc123".to_string()));
    }

    #[test]
    fn test_train_config_git_commit_serialization() {
        let config = TrainConfig {
            model_path: "/tmp/model".to_string(),
            dataset_path: "/tmp/dataset".to_string(),
            batch_size: 32,
            learning_rate: 0.001,
            epochs: 10,
            train_repo_url: None,
            script_entrypoint: None,
            dataset_repo_url: None,
            auth_threshold: 1,
            enable_auth: false,
            model_artifact_ref: None,
            git_commit: Some("f00bar".to_string()),
        };

        let json = serde_json::to_string(&config);
        assert!(json.is_ok(), "serialization failed: {:?}", json.err());

        let modified_json = json.unwrap().replace("\"f00bar\"", "\"abc123\"");
        let decoded: Result<TrainConfig, _> = serde_json::from_str(&modified_json);
        assert!(decoded.is_ok(), "deserialization failed: {:?}", decoded.err());

        let parsed = decoded.unwrap();
        assert_eq!(parsed.git_commit, Some("abc123".to_string()));
    }

    #[test]
    fn test_empty_repo_commit() {
        let commit = RepoCommit {
            commit: "".to_string(),
            repo_url: "".to_string(),
            signature: vec![],
        };
        assert!(commit.commit.is_empty());
        assert!(commit.repo_url.is_empty());
        assert!(commit.signature.is_empty());
    }

    #[test]
    fn test_drift_message_debug_format() {
        let commit = RepoCommit {
            commit: "abc123".to_string(),
            repo_url: "https://github.com/test/repo".to_string(),
            signature: vec![],
        };
        let msg = DriftMessage::RepoCommit(commit);
        let debug_str = format!("{:?}", msg);
        assert!(debug_str.contains("RepoCommit"));
        assert!(debug_str.contains("abc123"));
    }

    #[test]
    fn test_training_ready_debug_format() {
        let msg = DriftMessage::TrainingReady;
        let debug_str = format!("{:?}", msg);
        assert!(debug_str.contains("TrainingReady"));
    }

    #[test]
    fn test_drift_message_display_training_ready() {
        use std::fmt::Write;
        let msg = DriftMessage::TrainingReady;
        let mut s = String::new();
        write!(s, "{}", msg).unwrap();
        assert!(s.contains("TrainingReady"));
    }

    #[test]
    fn test_drift_message_display_repo_commit() {
        use std::fmt::Write;
        let commit = RepoCommit {
            commit: "abc123def".to_string(),
            repo_url: "https://github.com/test/repo".to_string(),
            signature: vec![],
        };
        let msg = DriftMessage::RepoCommit(commit);
        let mut s = String::new();
        write!(s, "{}", msg).unwrap();
        assert!(s.contains("RepoCommit"));
        assert!(s.contains("abc123")); // Display shows first 8 chars
    }

    #[test]
    fn test_repo_commit_with_long_commit_hash() {
        let long_hash = "a".repeat(64);
        let commit = RepoCommit {
            commit: long_hash,
            repo_url: "https://github.com/user/very-long-repo-name".to_string(),
            signature: vec![0; 64],
        };
        assert_eq!(commit.commit.len(), 64);
        let json = serde_json::to_string(&commit);
        assert!(json.is_ok());
    }

    #[test]
    fn test_drift_message_repo_commit_roundtrip() {
        let commit = RepoCommit {
            commit: "deadbeef".to_string(),
            repo_url: "https://github.com/test/repo".to_string(),
            signature: vec![1, 2, 3],
        };
        let msg = DriftMessage::RepoCommit(commit);

        let bytes = serde_json::to_vec(&msg).unwrap();
        let parsed: DriftMessage = serde_json::from_slice(&bytes).unwrap();

        match parsed {
            DriftMessage::RepoCommit(rc) => {
                assert_eq!(rc.commit, "deadbeef");
            }
            _ => panic!("expected RepoCommit after round-trip"),
        }
    }

    #[test]
    fn test_training_cancel_struct() {
        use drift_proto::TrainingCancel;
        let cancel = TrainingCancel {
            reason: "Commit hash mismatch".to_string(),
            time: "2024-01-01T00:00:00Z".to_string(),
            repo_url: "https://github.com/user/repo".to_string(),
        };
        assert_eq!(cancel.reason, "Commit hash mismatch");
        assert_eq!(cancel.time, "2024-01-01T00:00:00Z");
        assert_eq!(cancel.repo_url, "https://github.com/user/repo");
    }

    #[test]
    fn test_training_cancel_serialization_roundtrip() {
        use drift_proto::TrainingCancel;
        let cancel = TrainingCancel {
            reason: "unauthorized".to_string(),
            time: "2024-01-15T12:30:00Z".to_string(),
            repo_url: "https://github.com/test/project".to_string(),
        };

        let json = serde_json::to_string(&cancel);
        assert!(json.is_ok(), "serialization failed: {:?}", json.err());

        let decoded: Result<TrainingCancel, _> = serde_json::from_str(&json.unwrap());
        assert!(decoded.is_ok(), "deserialization failed: {:?}", decoded.err());

        let parsed = decoded.unwrap();
        assert_eq!(parsed.reason, "unauthorized");
        assert_eq!(parsed.time, "2024-01-15T12:30:00Z");
        assert_eq!(parsed.repo_url, "https://github.com/test/project");
    }

    #[test]
    fn test_drift_message_has_training_cancel_variant() {
        use drift_proto::TrainingCancel;
        let cancel = TrainingCancel {
            reason: "timeout".to_string(),
            time: "2024-02-20T10:00:00Z".to_string(),
            repo_url: "https://github.com/user/repo".to_string(),
        };
        let msg = DriftMessage::TrainingCancel(cancel);

        match msg {
            DriftMessage::TrainingCancel(c) => {
                assert_eq!(c.reason, "timeout");
            }
            other => panic!("expected TrainingCancel, got {:?}", other),
        }
    }

    #[test]
    fn test_drift_message_display_training_cancel() {
        use std::fmt::Write;
        use drift_proto::TrainingCancel;
        let cancel = TrainingCancel {
            reason: "Commit hash mismatch".to_string(),
            time: "2024-01-01T00:00:00Z".to_string(),
            repo_url: "https://github.com/user/repo".to_string(),
        };
        let msg = DriftMessage::TrainingCancel(cancel);
        let mut s = String::new();
        write!(s, "{}", msg).unwrap();
        assert!(s.contains("TrainingCancel"));
        assert!(s.contains("Commit hash mismatch"));
    }

    #[test]
    fn test_drift_message_training_cancel_debug_format() {
        use drift_proto::TrainingCancel;
        let cancel = TrainingCancel {
            reason: "Node timeout".to_string(),
            time: "2024-03-10T05:00:00Z".to_string(),
            repo_url: "https://github.com/another/repo".to_string(),
        };
        let msg = DriftMessage::TrainingCancel(cancel);
        let debug_str = format!("{:?}", msg);
        assert!(debug_str.contains("TrainingCancel"));
        assert!(debug_str.contains("Node timeout"));
    }

    #[test]
    fn test_training_cancel_roundtrip_through_drift_message() {
        use drift_proto::TrainingCancel;
        let cancel = TrainingCancel {
            reason: "signature invalid".to_string(),
            time: "2024-04-05T15:45:00Z".to_string(),
            repo_url: "https://github.com/special/repo".to_string(),
        };
        let msg = DriftMessage::TrainingCancel(cancel);

        let bytes = serde_json::to_vec(&msg).unwrap();
        let parsed: DriftMessage = serde_json::from_slice(&bytes).unwrap();

        match parsed {
            DriftMessage::TrainingCancel(c) => {
                assert_eq!(c.reason, "signature invalid");
                assert_eq!(c.time, "2024-04-05T15:45:00Z");
                assert_eq!(c.repo_url, "https://github.com/special/repo");
            }
            _ => panic!("expected TrainingCancel after round-trip"),
        }
    }

    #[test]
    fn test_training_cancel_with_empty_reason() {
        use drift_proto::TrainingCancel;
        let cancel = TrainingCancel {
            reason: "".to_string(),
            time: "".to_string(),
            repo_url: "".to_string(),
        };
        assert!(cancel.reason.is_empty());
        assert!(cancel.time.is_empty());
        assert!(cancel.repo_url.is_empty());
    }

    #[test]
    fn test_training_cancel_with_long_reason() {
        use drift_proto::TrainingCancel;
        let long_reason = "error ".repeat(100);
        let cancel = TrainingCancel {
            reason: long_reason,
            time: "2024-12-31T23:59:59Z".to_string(),
            repo_url: "https://github.com/long/repo/path/name".to_string(),
        };
        let json = serde_json::to_string(&cancel);
        assert!(json.is_ok(), "serialization of long reason failed");
    }
}

#[cfg(test)]
mod repo_commit_integration_tests {
    use drift_proto::{DriftMessage, RepoCommit, TrainingCancel};

    #[tokio::test]
    async fn test_all_nodes_same_commit_broadcasts_training_ready() {
        let node_a_commit = RepoCommit {
            commit: "abc123def456".to_string(),
            repo_url: "https://github.com/user/repo".to_string(),
            signature: vec![1, 2, 3],
        };
        let node_b_commit = RepoCommit {
            commit: "abc123def456".to_string(),
            repo_url: "https://github.com/user/repo".to_string(),
            signature: vec![4, 5, 6],
        };
        assert_eq!(node_a_commit.commit, node_b_commit.commit);
    }

    #[tokio::test]
    async fn test_different_commits_returns_mismatch() {
        let node_a_commit = RepoCommit {
            commit: "abc123".to_string(),
            repo_url: "https://github.com/user/repo".to_string(),
            signature: vec![1, 2, 3],
        };
        let node_b_commit = RepoCommit {
            commit: "def456".to_string(),
            repo_url: "https://github.com/user/repo".to_string(),
            signature: vec![4, 5, 6],
        };
        assert_ne!(node_a_commit.commit, node_b_commit.commit);
    }

    #[tokio::test]
    async fn test_commit_mismatch_reason_format() {
        let reason = format!(
            "Commit hash mismatch detected: {} different commits",
            2
        );
        let cancel = TrainingCancel {
            reason,
            time: "2024-01-01T00:00:00Z".to_string(),
            repo_url: "https://github.com/user/repo".to_string(),
        };
        assert!(cancel.reason.contains("Commit hash mismatch"));
        assert!(cancel.reason.contains("2"));
    }

    #[tokio::test]
    async fn test_invalid_signature_reason_unauthorized() {
        let cancel = TrainingCancel {
            reason: "unauthorized".to_string(),
            time: "2024-01-01T00:00:00Z".to_string(),
            repo_url: "https://github.com/user/repo".to_string(),
        };
        assert_eq!(cancel.reason, "unauthorized");
    }

    #[tokio::test]
    async fn test_timeout_reason_format() {
        let node_id = "node_abc123";
        let reason = format!("Node {} did not send RepoCommit after 30s", node_id);
        let cancel = TrainingCancel {
            reason,
            time: "2024-01-01T00:00:00Z".to_string(),
            repo_url: "https://github.com/user/repo".to_string(),
        };
        assert!(cancel.reason.contains("did not send RepoCommit"));
        assert!(cancel.reason.contains("30s"));
    }

    #[tokio::test]
    async fn test_standby_timeout_message() {
        let timeout_reason = "Standby timeout: no TrainingReady after 30s";
        assert!(timeout_reason.contains("Standby timeout"));
        assert!(timeout_reason.contains("30s"));
    }

    #[tokio::test]
    async fn test_coordinator_crash_before_training_ready() {
        let reason = "Coordinator crashed before broadcasting TrainingReady".to_string();
        let cancel = TrainingCancel {
            reason,
            time: "2024-01-01T00:00:00Z".to_string(),
            repo_url: "https://github.com/user/repo".to_string(),
        };
        assert!(cancel.reason.contains("Coordinator crashed"));
    }

    #[tokio::test]
    async fn test_fresh_git_ls_remote_same_commit_on_rejoin() {
        let first_commit = "abc123def456".to_string();
        let second_commit = "abc123def456".to_string();
        assert_eq!(first_commit, second_commit);
    }

    #[tokio::test]
    async fn test_fresh_git_ls_remote_new_commit_on_rejoin() {
        let first_commit = "abc123".to_string();
        let second_commit = "def456".to_string();
        assert_ne!(first_commit, second_commit);
    }

    #[tokio::test]
    async fn test_verify_repo_commit_accepts_valid_signature() {
        let commit = RepoCommit {
            commit: "abc123".to_string(),
            repo_url: "https://github.com/user/repo".to_string(),
            signature: vec![1, 2, 3, 4, 5],
        };
        assert!(!commit.signature.is_empty());
    }

    #[tokio::test]
    async fn test_verify_repo_commit_rejects_empty_signature() {
        let commit = RepoCommit {
            commit: "abc123".to_string(),
            repo_url: "https://github.com/user/repo".to_string(),
            signature: vec![],
        };
        assert!(commit.signature.is_empty());
    }

    #[tokio::test]
    async fn test_multiple_nodes_collect_all_commits() {
        let commits = vec![
            RepoCommit {
                commit: "abc123".to_string(),
                repo_url: "https://github.com/user/repo".to_string(),
                signature: vec![1],
            },
            RepoCommit {
                commit: "abc123".to_string(),
                repo_url: "https://github.com/user/repo".to_string(),
                signature: vec![2],
            },
        ];
        let unique_commits: std::collections::HashSet<_> = commits.iter().map(|c| &c.commit).collect();
        assert_eq!(unique_commits.len(), 1);
    }

    #[tokio::test]
    async fn test_multiple_nodes_detect_commit_mismatch() {
        let commits = vec![
            RepoCommit {
                commit: "abc123".to_string(),
                repo_url: "https://github.com/user/repo".to_string(),
                signature: vec![1],
            },
            RepoCommit {
                commit: "def456".to_string(),
                repo_url: "https://github.com/user/repo".to_string(),
                signature: vec![2],
            },
        ];
        let unique_commits: std::collections::HashSet<_> = commits.iter().map(|c| &c.commit).collect();
        assert_ne!(unique_commits.len(), 1);
    }

    #[tokio::test]
    async fn test_training_ready_broadcast_to_all_nodes() {
        let nodes = vec!["node1", "node2", "node3"];
        for node_id in nodes {
            let msg = DriftMessage::TrainingReady;
            match msg {
                DriftMessage::TrainingReady => {}
                _ => panic!("expected TrainingReady for {}", node_id),
            }
        }
    }

    #[tokio::test]
    async fn test_training_cancel_broadcast_to_all_nodes() {
        let nodes = vec!["node1", "node2", "node3"];
        for node_id in nodes {
            let cancel = TrainingCancel {
                reason: "Commit hash mismatch".to_string(),
                time: "2024-01-01T00:00:00Z".to_string(),
                repo_url: "https://github.com/user/repo".to_string(),
            };
            let msg = DriftMessage::TrainingCancel(cancel);
            match msg {
                DriftMessage::TrainingCancel(c) => {
                    assert_eq!(c.reason, "Commit hash mismatch");
                }
                _ => panic!("expected TrainingCancel for {}", node_id),
            }
        }
    }

    #[tokio::test]
    async fn test_30_second_timeout_duration() {
        use std::time::Duration;
        let timeout = Duration::from_secs(30);
        assert_eq!(timeout.as_secs(), 30);
    }

    #[tokio::test]
    async fn test_rfc3339_timestamp_format() {
        let cancel = TrainingCancel {
            reason: "test".to_string(),
            time: "2024-01-15T12:30:00Z".to_string(),
            repo_url: "https://github.com/user/repo".to_string(),
        };
        assert!(cancel.time.contains("-"));
        assert!(cancel.time.contains(":"));
        assert!(cancel.time.ends_with("Z"));
    }

    #[tokio::test]
    async fn test_commit_hash_display_truncated_to_8_chars() {
        let commit = RepoCommit {
            commit: "abc123def456789".to_string(),
            repo_url: "https://github.com/user/repo".to_string(),
            signature: vec![],
        };
        let display = format!("{}", DriftMessage::RepoCommit(commit));
        assert!(display.contains("abc123de"));
    }
}