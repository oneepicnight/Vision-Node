use tracing::{Level, Metadata, Subscriber};
use tracing_subscriber::layer::{Context, Layer};

/// Layer that captures log events and broadcasts them to WebSocket clients
pub struct LogCaptureLayer;

impl LogCaptureLayer {
    pub fn new() -> Self {
        Self
    }

    /// Check if a log should be broadcasted (exclude verbose DEBUG logs)
    fn should_broadcast(metadata: &Metadata) -> bool {
        // Only broadcast INFO, WARN, ERROR
        matches!(*metadata.level(), Level::INFO | Level::WARN | Level::ERROR)
    }

    /// Format log entry as JSON for WebSocket broadcast
    fn format_log_entry(
        level: &Level,
        target: &str,
        message: String,
        fields: Vec<(String, String)>,
    ) -> String {
        // Extract key fields
        let mut chain_id = None;
        let mut pow_fp = None;
        let mut block_hash = None;
        let mut height = None;
        let mut miner = None;
        let mut peer = None;
        let mut extra_fields = Vec::new();

        for (key, value) in fields {
            match key.as_str() {
                "chain_id" => chain_id = Some(value),
                "pow_fp" => pow_fp = Some(value),
                "block_hash" | "hash" => block_hash = Some(value),
                "height" => height = Some(value),
                "miner" => miner = Some(value),
                "peer" | "peer_addr" => peer = Some(value),
                _ => extra_fields.push((key, value)),
            }
        }

        // Determine log category from message prefix
        let category = if message.starts_with("[PAYOUT]") {
            "payout"
        } else if message.starts_with("[CANON]") {
            "canon"
        } else if message.starts_with("[ORPHAN]") {
            "orphan"
        } else if message.starts_with("[REJECT]") {
            "reject"
        } else if message.starts_with("[ACCEPT]") {
            "accept"
        } else if message.starts_with("[P2P]") {
            "p2p"
        } else if message.starts_with("[COMPAT]") {
            "compat"
        } else if message.starts_with("[SYNC") {
            "sync"
        } else if message.starts_with("[MINER-ERROR]") {
            "miner_error"
        } else if message.starts_with("[JOB-CHECK]") {
            "job_check"
        } else if message.starts_with("[STRIKE]") {
            "strike"
        } else {
            "general"
        };

        let mut json = serde_json::json!({
            "type": "log",
            "timestamp": chrono::Utc::now().timestamp(),
            "level": level.to_string().to_lowercase(),
            "target": target,
            "message": message,
            "category": category,
        });

        // Add optional fields if present
        let obj = json.as_object_mut().unwrap();
        if let Some(cid) = chain_id {
            obj.insert("chain_id".to_string(), serde_json::Value::String(cid));
        }
        if let Some(fp) = pow_fp {
            obj.insert("pow_fp".to_string(), serde_json::Value::String(fp));
        }
        if let Some(hash) = block_hash {
            obj.insert("block_hash".to_string(), serde_json::Value::String(hash));
        }
        if let Some(h) = height {
            obj.insert("height".to_string(), serde_json::Value::String(h));
        }
        if let Some(m) = miner {
            obj.insert("miner".to_string(), serde_json::Value::String(m));
        }
        if let Some(p) = peer {
            obj.insert("peer".to_string(), serde_json::Value::String(p));
        }
        if !extra_fields.is_empty() {
            let mut fields_obj = serde_json::Map::new();
            for (k, v) in extra_fields {
                fields_obj.insert(k, serde_json::Value::String(v));
            }
            obj.insert("fields".to_string(), serde_json::Value::Object(fields_obj));
        }

        serde_json::to_string(&json).unwrap_or_else(|_| "{}".to_string())
    }
}

impl<S> Layer<S> for LogCaptureLayer
where
    S: Subscriber,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: Context<'_, S>,
    ) {
        let metadata = event.metadata();

        // Filter out verbose logs
        if !Self::should_broadcast(metadata) {
            return;
        }

        // Capture log fields
        struct FieldVisitor {
            message: Option<String>,
            fields: Vec<(String, String)>,
        }

        impl tracing::field::Visit for FieldVisitor {
            fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
                let value_str = format!("{:?}", value);
                if field.name() == "message" {
                    self.message = Some(value_str.trim_matches('"').to_string());
                } else {
                    self.fields.push((field.name().to_string(), value_str.trim_matches('"').to_string()));
                }
            }

            fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
                if field.name() == "message" {
                    self.message = Some(value.to_string());
                } else {
                    self.fields.push((field.name().to_string(), value.to_string()));
                }
            }
        }

        let mut visitor = FieldVisitor {
            message: None,
            fields: Vec::new(),
        };

        event.record(&mut visitor);

        if let Some(message) = visitor.message {
            let json_log = Self::format_log_entry(
                metadata.level(),
                metadata.target(),
                message,
                visitor.fields,
            );

            // Broadcast to WebSocket clients (non-blocking)
            let _ = crate::WS_LOGS_TX.send(json_log);
        }
    }
}
