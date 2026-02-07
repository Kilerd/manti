use crate::db::DatabaseService;
use crate::models::usage::CreateUsage;
use rust_decimal::Decimal;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, warn};
use uuid::Uuid;

/// Usage record with metadata for balance deduction
struct UsageWithMeta {
    usage: CreateUsage,
    user_id: Uuid,
    cost: Decimal,
}

/// Async usage recorder that buffers writes for non-blocking operation
pub struct UsageRecorder {
    sender: mpsc::UnboundedSender<UsageWithMeta>,
}

impl UsageRecorder {
    /// Create a new usage recorder with a background write task
    pub fn new(db: Arc<DatabaseService>) -> Self {
        let (sender, mut receiver) = mpsc::unbounded_channel::<UsageWithMeta>();

        // Spawn background task for writing usage records
        tokio::spawn(async move {
            while let Some(meta) = receiver.recv().await {
                let user_id = meta.user_id;
                let cost = meta.cost;

                match db.record_usage(meta.usage).await {
                    Ok(_) => {
                        // Deduct balance after successful usage recording
                        if let Err(e) = db.deduct_balance(user_id, cost).await {
                            error!("Failed to deduct balance for user {}: {}", user_id, e);
                        }
                    }
                    Err(e) => {
                        error!("Failed to record usage: {}", e);
                        // In production, might want to:
                        // 1. Retry with backoff
                        // 2. Write to a dead letter queue
                        // 3. Alert monitoring system
                    }
                }
            }
            warn!("Usage recorder channel closed");
        });

        Self { sender }
    }

    /// Record usage asynchronously (non-blocking)
    /// Returns immediately without waiting for DB write
    /// Also deducts the cost from user's balance after recording
    pub fn record(&self, usage: CreateUsage) {
        let user_id = usage.user_id;
        let cost = usage.cost;
        let meta = UsageWithMeta {
            usage,
            user_id,
            cost,
        };
        if let Err(e) = self.sender.send(meta) {
            error!("Failed to queue usage record: {}", e);
        }
    }

    /// Check if the recorder is still accepting records
    pub fn is_closed(&self) -> bool {
        self.sender.is_closed()
    }
}

impl Clone for UsageRecorder {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
        }
    }
}
