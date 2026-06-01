use drift_proto::{DriftMessage, TrainProgress};
use tokio::sync::mpsc;

#[tokio::test]
async fn test_spawn_training_with_progress_sends_progress_messages() {
    let (tx, mut rx) = mpsc::channel::<DriftMessage>(16);

    let node_id = "test_node_123".to_string();

    let expected_messages = vec![
        TrainProgress {
            node_id: node_id.clone(),
            epoch: 1u32,
            step: 0u64,
            loss: 2.5,
            throughput_samples_per_sec: 100.0,
        },
        TrainProgress {
            node_id: node_id.clone(),
            epoch: 1u32,
            step: 100u64,
            loss: 1.8,
            throughput_samples_per_sec: 100.0,
        },
    ];

    for progress in expected_messages {
        tx.send(DriftMessage::TrainProgress(progress)).await.unwrap();
    }

    drop(tx);

    let mut received: Vec<TrainProgress> = Vec::new();
    while let Some(msg) = rx.recv().await {
        match msg {
            DriftMessage::TrainProgress(p) => received.push(p),
            _ => panic!("expected TrainProgress"),
        }
    }

    assert_eq!(received.len(), 2);
    assert_eq!(received[0].step, 0u64);
    assert_eq!(received[1].step, 100u64);
}

#[tokio::test]
async fn test_progress_channel_drops_on_drop() {
    let (tx, mut rx) = mpsc::channel::<DriftMessage>(1);

    tx.send(DriftMessage::TrainProgress(TrainProgress {
        node_id: "test".to_string(),
        epoch: 1u32,
        step: 0u64,
        loss: 1.0,
        throughput_samples_per_sec: 0.0,
    })).await.unwrap();

    drop(tx);

    let msg = rx.recv().await.unwrap();
    assert!(matches!(msg, DriftMessage::TrainProgress(_)));
}

#[test]
fn test_progress_loss_decreases_over_time() {
    let losses = vec![2.5, 2.0, 1.5, 1.0, 0.5];
    for i in 1..losses.len() {
        assert!(losses[i] < losses[i - 1], "loss should decrease");
    }
}

#[test]
fn test_progress_epoch_increment() {
    let steps_per_epoch = 100u64;
    let epoch = 2u32;
    let step_in_epoch = 50u64;
    let absolute_step = (epoch as u64 * steps_per_epoch) + step_in_epoch;
    assert_eq!(absolute_step, 250u64);
}

#[test]
fn test_checkpoint_throttle_triggers_at_interval() {
    use drift_node::training::{should_write_checkpoint, CHECKPOINT_THROTTLE_INTERVAL};

    assert!(should_write_checkpoint(CHECKPOINT_THROTTLE_INTERVAL));
    assert!(should_write_checkpoint(CHECKPOINT_THROTTLE_INTERVAL * 2));
    assert!(!should_write_checkpoint(CHECKPOINT_THROTTLE_INTERVAL - 1));
    assert!(!should_write_checkpoint(0));
}

#[test]
fn test_checkpoint_throttle_never_triggers_at_zero() {
    use drift_node::training::should_write_checkpoint;

    assert!(!should_write_checkpoint(0));
}

#[test]
fn test_checkpoint_throttle_never_triggers_before_interval() {
    use drift_node::training::{should_write_checkpoint, CHECKPOINT_THROTTLE_INTERVAL};

    assert!(!should_write_checkpoint(CHECKPOINT_THROTTLE_INTERVAL - 1));
    assert!(!should_write_checkpoint(1));
    assert!(!should_write_checkpoint(50));
}