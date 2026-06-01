use drift_proto::{DriftMessage, ShardAssignment};

#[test]
fn test_ask_for_more_work_variant() {
    let msg = DriftMessage::AskForMoreWork;
    
    if let DriftMessage::AskForMoreWork = msg {
        // Pattern matches
    } else {
        panic!("Expected AskForMoreWork variant");
    }
}

#[test]
fn test_no_more_work_variant() {
    let msg = DriftMessage::NoMoreWork;
    
    if let DriftMessage::NoMoreWork = msg {
        // Pattern matches
    } else {
        panic!("Expected NoMoreWork variant");
    }
}

#[test]
fn test_assign_next_variant() {
    let shard = ShardAssignment::default();
    let msg = DriftMessage::AssignNext(shard);

    if let DriftMessage::AssignNext(s) = msg {
        assert_eq!(s.shard_index, 0u32);
    } else {
        panic!("Expected AssignNext variant");
    }
}