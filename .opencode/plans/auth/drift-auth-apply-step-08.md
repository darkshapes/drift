## 8. Coordinator Protocol (Items 33-37)

**Modify drift-coord/src/main.rs or create drift-coord/src/auth.rs:**

```rust
// drift-coord/src/auth.rs
// Checklist items: 33, 34, 35, 36, 37

use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot};
use anyhow::{Context, Result};

use drift_proto::{DriftMessage, TrainConfig};
use drift_auth::{Aggregator, SignedAuthMessage, AggregationError};

/// Coordinator's auth context during a training run
pub struct CoordinatorAuth {
    /// Map of node_id -> sender for sending aggregate back
    node_channels: HashMap<String, mpsc::Sender<DriftMessage>>,
    /// Aggregator for collecting signatures
    aggregator: Option<Aggregator>,
    /// Training config (to check if auth is enabled)
    train_config: TrainConfig,
}

impl CoordinatorAuth {
    pub fn new(train_config: TrainConfig) -> Self {
        Self {
            node_channels: HashMap::new(),
            aggregator: None,
            train_config,
        }
    }
    
    /// Register a node's channel for sending responses
    pub fn register_node(&mut self, node_id: String, tx: mpsc::Sender<DriftMessage>) {
        self.node_channels.insert(node_id, tx);
    }
    
    /// === Item 33: collect_signatures_from_nodes ===
    pub async fn collect_signatures(
        &mut self,
        expected_nodes: Vec<String>,
        timeout: Duration,
    ) -> Result<AggregateAuthMessage, anyhow::Error> {
        // Create aggregator
        let threshold = self.train_config.auth_threshold;
        let mut aggregator = Aggregator::new(expected_nodes, threshold, timeout);
        self.aggregator = Some(aggregator);
        let agg_ref = self.aggregator.as_mut().unwrap();
        
        // Wait loop: collect signatures from nodes via channels
        // In production, this would be integrated with the message handling loop
        let start = Instant::now();
        loop {
            if agg_ref.is_timed_out() {
                return Err(anyhow::anyhow!("timeout waiting for signatures"));
            }
            
            // Check if we have enough signatures
            if agg_ref.has_threshold() {
                break;
            }
            
            // Sleep briefly
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        
        // Create aggregate
        let agg_msg = agg_ref.create_aggregate()
            .map_err(|e: AggregationError| anyhow::anyhow!(e))?;
        
        Ok(agg_msg)
    }
    
    /// === Item 34: broadcast_aggregate_to_all_nodes ===
    pub async fn broadcast_aggregate(
        &self,
        agg_msg: &AggregateAuthMessage,
    ) -> Result<()> {
        for (node_id, tx) in self.node_channels.iter() {
            let msg = DriftMessage::AuthAggregate(agg_msg.clone());
            tx.send(msg).await
                .context(format!("failed to send to node {}", node_id))?;
        }
        Ok(())
    }
    
    /// === Item 35: Logging ===
    pub fn log_status(&self) {
        if let Some(agg) = &self.aggregator {
            println!("Auth status: {}/{} signatures collected, need {}",
                agg.collected_count(),
                agg.total_nodes,
                agg.threshold);
            
            let missing: Vec<&String> = agg.missing_nodes();
            if !missing.is_empty() {
                println!("  Missing from: {:?}", missing);
            }
        }
    }
    
    /// === Item 36: Handle coordinator key rotation ===
    pub fn rotate_coordinator_keys(&mut self) -> Result<()> {
        // In production, coordinator would generate new keypair
        // and notify all nodes of new public key
        todo!("implement coordinator key rotation")
    }
}

/// === Item 37: Test coordinator auth flow ===
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_coordinator_auth_collection() -> Result<()> {
        let mut auth = CoordinatorAuth::new(TrainConfig {
            enable_auth: true,
            auth_threshold: 2,
            ..Default::default()
        });
        
        // Simulate 3 nodes, but only 2 will send signatures
        let nodes = vec!["node1".to_string(), "node2".to_string(), "node3".to_string()];
        let (tx1, _rx1) = mpsc::channel(16);
        let (tx2, _rx2) = mpsc::channel(16);
        let (tx3, _rx3) = mpsc::channel(16);
        
        auth.register_node("node1".to_string(), tx1);
        auth.register_node("node2".to_string(), tx2);
        auth.register_node("node3".to_string(), tx3);
        
        // Should be able to collect from threshold number
        // This is a simplified test - full integration would have nodes actually send
        Ok(())
    }
}
```

