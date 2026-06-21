use drift_proto::TrainConfig;

#[test]
fn test_train_config_empty_dataset_urls() {
    let config = TrainConfig {
        model_artifact: None,
        repo_hash: None,
        dataset_urls: vec![],
    };
    assert!(config.dataset_urls.is_empty());
}

#[test]
fn test_train_config_single_dataset_url() {
    let config = TrainConfig {
        model_artifact: Some("hf://model".to_string()),
        repo_hash: Some("abc123".to_string()),
        dataset_urls: vec!["https://huggingface.co/datasets/user/dataset".to_string()],
    };
    assert_eq!(config.dataset_urls.len(), 1);
    assert_eq!(config.dataset_urls[0], "https://huggingface.co/datasets/user/dataset");
}

#[test]
fn test_train_config_multiple_dataset_urls() {
    let config = TrainConfig {
        model_artifact: Some("local:///path/to/model".to_string()),
        repo_hash: Some("def456".to_string()),
        dataset_urls: vec![
            "https://huggingface.co/datasets/user/dataset1".to_string(),
            "https://huggingface.co/datasets/user/dataset2".to_string(),
            "/local/path/dataset3".to_string(),
        ],
    };
    assert_eq!(config.dataset_urls.len(), 3);
    assert_eq!(config.dataset_urls[0], "https://huggingface.co/datasets/user/dataset1");
    assert_eq!(config.dataset_urls[1], "https://huggingface.co/datasets/user/dataset2");
    assert_eq!(config.dataset_urls[2], "/local/path/dataset3");
}

#[test]
fn test_train_config_dataset_urls_serialization_roundtrip() {
    let config = TrainConfig {
        model_artifact: Some("hf://model".to_string()),
        repo_hash: Some("xyz789".to_string()),
        dataset_urls: vec![
            "https://huggingface.co/datasets/user/dataset1".to_string(),
            "https://huggingface.co/datasets/user/dataset2".to_string(),
        ],
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
        model_artifact: None,
        repo_hash: None,
        dataset_urls: vec!["https://example.com/dataset".to_string()],
    };

    let json = serde_json::to_string(&config).unwrap();
    assert!(json.contains("dataset_urls"));
    assert!(json.contains("https://example.com/dataset"));
}

#[test]
fn test_train_config_refactored_full() {
    let config = TrainConfig {
        model_artifact: Some("hf://model".to_string()),
        repo_hash: Some("abc123".to_string()),
        dataset_urls: vec!["https://data.example.com/set1".to_string(), "https://data.example.com/set2".to_string()],
    };

    assert_eq!(config.dataset_urls.len(), 2);
    assert!(config.model_artifact.is_some());
    assert!(config.repo_hash.is_some());
}

#[test]
fn test_train_config_dataset_urls_empty_vs_none() {
    let config = TrainConfig {
        model_artifact: None,
        repo_hash: None,
        dataset_urls: vec![],
    };

    assert!(config.dataset_urls.is_empty());
    assert_eq!(config.dataset_urls, Vec::<String>::new());
}
