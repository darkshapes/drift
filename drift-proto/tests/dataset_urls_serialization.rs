use drift_proto::TrainConfig;

#[test]
fn test_train_config_empty_dataset_urls() {
    let config = TrainConfig {
        model_path: "/tmp/model".to_string(),
        dataset_path: "/tmp/dataset".to_string(),
        batch_size: 32,
        learning_rate: 0.001,
        epochs: 10,
        train_repo_url: None,
        script_entrypoint: None,
        dataset_repo_url: None,
        auth_threshold: 3,
        enable_auth: false,
        model_artifact_ref: None,
        git_commit: None,
        dataset_urls: vec![],
        gpu_compute_capability: None,
        repo_path: None,
        env_file: None,
        training_spawn_cmd: None,
        env_vars: None,
    };
    assert!(config.dataset_urls.is_empty());
}

#[test]
fn test_train_config_single_dataset_url() {
    let config = TrainConfig {
        model_path: "/tmp/model".to_string(),
        dataset_path: "/tmp/dataset".to_string(),
        batch_size: 32,
        learning_rate: 0.001,
        epochs: 10,
        train_repo_url: None,
        script_entrypoint: None,
        dataset_repo_url: None,
        auth_threshold: 3,
        enable_auth: false,
        model_artifact_ref: None,
        git_commit: None,
        dataset_urls: vec!["https://huggingface.co/datasets/user/dataset".to_string()],
        gpu_compute_capability: None,
        repo_path: None,
        env_file: None,
        training_spawn_cmd: None,
        env_vars: None,
    };
    assert_eq!(config.dataset_urls.len(), 1);
    assert_eq!(config.dataset_urls[0], "https://huggingface.co/datasets/user/dataset");
}

#[test]
fn test_train_config_multiple_dataset_urls() {
    let config = TrainConfig {
        model_path: "/tmp/model".to_string(),
        dataset_path: "/tmp/dataset".to_string(),
        batch_size: 32,
        learning_rate: 0.001,
        epochs: 10,
        train_repo_url: None,
        script_entrypoint: None,
        dataset_repo_url: None,
        auth_threshold: 3,
        enable_auth: false,
        model_artifact_ref: None,
        git_commit: None,
        dataset_urls: vec![
            "https://huggingface.co/datasets/user/dataset1".to_string(),
            "https://huggingface.co/datasets/user/dataset2".to_string(),
            "/local/path/dataset3".to_string(),
        ],
        gpu_compute_capability: None,
        repo_path: None,
        env_file: None,
        training_spawn_cmd: None,
        env_vars: None,
    };
    assert_eq!(config.dataset_urls.len(), 3);
    assert_eq!(config.dataset_urls[0], "https://huggingface.co/datasets/user/dataset1");
    assert_eq!(config.dataset_urls[1], "https://huggingface.co/datasets/user/dataset2");
    assert_eq!(config.dataset_urls[2], "/local/path/dataset3");
}

#[test]
fn test_train_config_dataset_urls_serialization_roundtrip() {
    let config = TrainConfig {
        model_path: "/tmp/model".to_string(),
        dataset_path: "/tmp/dataset".to_string(),
        batch_size: 32,
        learning_rate: 0.001,
        epochs: 10,
        train_repo_url: None,
        script_entrypoint: None,
        dataset_repo_url: None,
        auth_threshold: 3,
        enable_auth: false,
        model_artifact_ref: None,
        git_commit: None,
        dataset_urls: vec![
            "https://huggingface.co/datasets/user/dataset1".to_string(),
            "https://huggingface.co/datasets/user/dataset2".to_string(),
        ],
        gpu_compute_capability: None,
        repo_path: None,
        env_file: None,
        training_spawn_cmd: None,
        env_vars: None,
    };

    let json = serde_json::to_string(&config);
    assert!(json.is_ok(), "serialization failed: {:?}", json.err());

    let decoded: Result<TrainConfig, _> = serde_json::from_str(&json.unwrap());
    assert!(decoded.is_ok(), "deserialization failed: {:?}", decoded.err());

    let parsed = decoded.unwrap();
    assert_eq!(parsed.dataset_urls, config.dataset_urls);
}

#[test]
fn test_train_config_dataset_urls_json_format() {
    let config = TrainConfig {
        model_path: "/tmp/model".to_string(),
        dataset_path: "/tmp/dataset".to_string(),
        batch_size: 32,
        learning_rate: 0.001,
        epochs: 10,
        train_repo_url: None,
        script_entrypoint: None,
        dataset_repo_url: None,
        auth_threshold: 3,
        enable_auth: false,
        model_artifact_ref: None,
        git_commit: None,
        dataset_urls: vec!["https://example.com/dataset".to_string()],
        gpu_compute_capability: None,
        repo_path: None,
        env_file: None,
        training_spawn_cmd: None,
        env_vars: None,
    };

    let json = serde_json::to_string(&config).unwrap();
    assert!(json.contains("dataset_urls"));
    assert!(json.contains("https://example.com/dataset"));
}

#[test]
fn test_train_config_dataset_urls_with_other_fields() {
    let config = TrainConfig {
        model_path: "/tmp/model".to_string(),
        dataset_path: "/tmp/dataset".to_string(),
        batch_size: 32,
        learning_rate: 0.001,
        epochs: 10,
        train_repo_url: Some("https://github.com/user/repo".to_string()),
        script_entrypoint: Some("src.main:ati".to_string()),
        dataset_repo_url: Some("https://huggingface.co/datasets/user/dataset".to_string()),
        auth_threshold: 3,
        enable_auth: true,
        model_artifact_ref: Some("model.safetensors".to_string()),
        git_commit: Some("abc123".to_string()),
        dataset_urls: vec!["https://data.example.com/set1".to_string(), "https://data.example.com/set2".to_string()],
        gpu_compute_capability: Some(8.9),
        repo_path: None,
        env_file: None,
        training_spawn_cmd: None,
        env_vars: None,
    };

    assert_eq!(config.dataset_urls.len(), 2);
    assert!(config.train_repo_url.is_some());
    assert!(config.script_entrypoint.is_some());
    assert!(config.git_commit.is_some());
    assert!(config.gpu_compute_capability.is_some());
}

#[test]
fn test_train_config_dataset_urls_empty_vs_none() {
    let config = TrainConfig {
        model_path: "/tmp/model".to_string(),
        dataset_path: "/tmp/dataset".to_string(),
        batch_size: 32,
        learning_rate: 0.001,
        epochs: 10,
        train_repo_url: None,
        script_entrypoint: None,
        dataset_repo_url: None,
        auth_threshold: 3,
        enable_auth: false,
        model_artifact_ref: None,
        git_commit: None,
        dataset_urls: vec![],
        gpu_compute_capability: None,
        repo_path: None,
        env_file: None,
        training_spawn_cmd: None,
        env_vars: None,
    };

    assert!(config.dataset_urls.is_empty());
    assert_eq!(config.dataset_urls, Vec::<String>::new());
}
