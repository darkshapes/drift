use drift_node::network::{create_endpoint, handle_connection};
use drift_proto::{DriftMessage, NodeInfo, TrainConfig, read_message, write_message, DRIFT_ALPN, TrainingCancel};
use drift_auth::crypto::verify_repo_commit;
use iroh::EndpointAddr;
use tokio::time::Duration;

#[tokio::test]
async fn test_repo_commit_signed_with_iroh_key() {
    let (node_endpoint, node_addr) = create_endpoint().await.unwrap();
    let node_id = node_endpoint.id().to_string();
    let node_pubkey = node_endpoint.id();

    let (coord_endpoint, coord_addr) = create_endpoint().await.unwrap();

    let temp_dir = std::env::temp_dir().join("drift-test-repo");
    std::fs::create_dir_all(&temp_dir).unwrap();
    std::process::Command::new("git")
       .arg("init")
       .arg(&temp_dir)
       .output()
       .unwrap();
    std::process::Command::new("git")
       .arg("-C")
       .arg(&temp_dir)
       .arg("config")
       .arg("user.email")
       .arg("test@test.com")
       .output()
       .unwrap();
    std::process::Command::new("git")
       .arg("-C")
       .arg(&temp_dir)
       .arg("config")
       .arg("user.name")
       .arg("Test")
       .output()
       .unwrap();
    let readme = temp_dir.join("README.md");
    std::fs::write(&readme, "# test").unwrap();
    std::process::Command::new("git")
       .arg("-C")
       .arg(&temp_dir)
       .arg("add")
       .arg("README.md")
       .output()
       .unwrap();
    std::process::Command::new("git")
       .arg("-C")
       .arg(&temp_dir)
       .arg("commit")
       .arg("-m")
       .arg("initial")
       .output()
       .unwrap();

    let repo_url = temp_dir.display().to_string();

    let node_id_for_coord = node_id.clone();
    let node_pubkey_for_coord = node_pubkey.clone();
    let coord_task = tokio::spawn(async move {
        let conn = coord_endpoint.connect(node_addr, DRIFT_ALPN).await.unwrap();
        let (mut send, mut recv) = conn.accept_bi().await.unwrap();

        write_message(&mut send, &DriftMessage::Ping).await.unwrap();
        let _node_info = read_message(&mut recv).await.unwrap();

        let config = TrainConfig {
            train_repo_url: Some(repo_url.clone()),
            ..Default::default()
        };
        write_message(&mut send, &DriftMessage::TrainConfig(config)).await.unwrap();

         let msg = match tokio::time::timeout(
            Duration::from_secs(60),
            read_message(&mut recv),
        ).await {
            Ok(Ok(msg)) => msg,
            Ok(Err(e)) => panic!("read error: {}", e),
            Err(_) => {
                let cancel = DriftMessage::TrainingCancel(TrainingCancel {
                    reason: "timeout waiting for RepoCommit".to_string(),
                    time: "0".to_string(),
                    repo_url: repo_url.clone(),
                });
                let _ = write_message(&mut send, &cancel).await;
                panic!("timeout waiting for RepoCommit");
            }
        };

        if let DriftMessage::RepoCommit(rc) = msg {
            let result = verify_repo_commit(&node_pubkey_for_coord, &node_id_for_coord, &rc.commit, &repo_url, &rc.signature);
            assert!(result.is_ok(), "Signature should verify with node's iroh public key");
        } else {
            panic!("Expected RepoCommit, got {:?}", msg);
        }
    });

    let accept_task = tokio::spawn(async move {
        let incoming = node_endpoint.accept().await.expect("no incoming connection");
        let conn = incoming.await.unwrap();
        let node_info = DriftMessage::NodeInfo(NodeInfo {
            node_id: node_id.clone(),
            gpu_name: "test".to_string(),
            gpu_vram_mb: 0,
            gpu_compute_capability: "0.0".to_string(),
            available: true,
        });
        handle_connection(&node_endpoint, conn, node_info, &node_id).await.ok();
    });

    coord_task.await.unwrap();
    accept_task.await.unwrap();
}
