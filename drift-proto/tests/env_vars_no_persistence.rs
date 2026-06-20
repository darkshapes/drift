use drift_proto::{TrainConfig, DriftMessage};
use std::collections::HashMap;

#[test]
fn test_env_vars_not_serialized_in_train_config() {
    let mut config = TrainConfig::default();
    let mut env_vars = HashMap::new();
    env_vars.insert("SECRET".to_string(), "value".to_string());
    config.update_env_vars(env_vars);

    let json = serde_json::to_string(&config).unwrap();

    assert!(!json.contains("env_vars"), "env_vars should not appear in JSON");
    assert!(!json.contains("SECRET"), "sensitive value should not appear in JSON");
}

#[test]
fn test_env_vars_round_trip_is_none() {
    let mut config = TrainConfig::default();
    let mut env_vars = HashMap::new();
    env_vars.insert("VAR".to_string(), "value".to_string());
    config.update_env_vars(env_vars);

    let json = serde_json::to_string(&config).unwrap();
    let parsed: TrainConfig = serde_json::from_str(&json).unwrap();

    assert!(parsed.env_vars.is_none());
}

#[test]
fn test_train_config_checkpoints_do_not_contain_env_vars() {
    let mut config = TrainConfig {
        model_path: "/model".to_string(),
        dataset_path: "/data".to_string(),
        batch_size: 32u32,
        learning_rate: 0.001,
        epochs: 100u32,
        train_repo_url: None,
        script_entrypoint: None,
        dataset_repo_url: None,
        model_artifact_ref: None,
        enable_auth: false,
        auth_threshold: 0usize,
        git_commit: None,
        dataset_urls: vec![],
        gpu_compute_capability: None,
        repo_path: None,
        env_file: None,
        env_vars: None,
        training_spawn_cmd: None,
    };

    let mut env_vars = HashMap::new();
    env_vars.insert("PRODUCTION_SECRET".to_string(), "highly_secret".to_string());
    config.update_env_vars(env_vars);

    let json = serde_json::to_string(&config).unwrap();

    assert!(json.contains("model_path"), "expected model_path in JSON");
    assert!(!json.contains("env_vars"), "env_vars must be skipped");
    assert!(!json.contains("PRODUCTION_SECRET"), "secrets must not leak");
}
