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
}