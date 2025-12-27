use axum::{extract::State, http::StatusCode, Json};
use ed25519_dalek::{Signer, SigningKey};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Registered Constellation node information
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegisteredNode {
    pub node_id: String,
    pub ip: String,
    pub port: u16,
    pub registered_at: u64,
    pub last_heartbeat: u64,
}

/// Global registry of Constellation nodes
pub type NodeRegistry = Arc<RwLock<HashMap<String, RegisteredNode>>>;

#[derive(Clone)]
pub struct BeaconState {
    pub nodes: NodeRegistry,
}

impl BeaconState {
    pub fn new() -> Self {
        Self {
            nodes: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

/// POST /api/beacon/register - Register a Constellation node
#[derive(Deserialize)]
pub struct RegisterRequest {
    pub node_id: String,
    pub ip: String,
    pub port: u16,
}

#[derive(Serialize)]
pub struct RegisterResponse {
    pub ok: bool,
    pub message: String,
    pub node_id: String,
    pub passport: Option<crate::passport::NodePassport>,
}

pub async fn register_node(
    State(state): State<BeaconState>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<RegisterResponse>, (StatusCode, String)> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let node = RegisteredNode {
        node_id: req.node_id.clone(),
        ip: req.ip.clone(),
        port: req.port,
        registered_at: now,
        last_heartbeat: now,
    };

    let mut nodes = state.nodes.write();
    nodes.insert(req.node_id.clone(), node);

    // Issue signed NodePassport (7-day expiry)
    let passport = match issue_passport(&req.node_id, now) {
        Ok(p) => {
            tracing::info!(
                "[BEACON_REGISTER] Issued passport to node_tag={} network={} expires_at={}",
                p.node_tag,
                p.network,
                p.expires_at
            );
            Some(p)
        }
        Err(e) => {
            tracing::warn!(
                "[BEACON_REGISTER] Failed to issue passport for {}: {}",
                req.node_id,
                e
            );
            None
        }
    };

    tracing::info!(
        "[BEACON] 📥 Constellation node registered: {} ({}:{})",
        req.node_id,
        req.ip,
        req.port
    );

    Ok(Json(RegisterResponse {
        ok: true,
        message: "Node registered successfully".to_string(),
        node_id: req.node_id,
        passport,
    }))
}

/// GET /api/beacon/nodes - List all registered Constellation nodes
#[derive(Serialize)]
pub struct NodesResponse {
    pub ok: bool,
    pub count: usize,
    pub nodes: Vec<RegisteredNode>,
}

pub async fn list_nodes(State(state): State<BeaconState>) -> Json<NodesResponse> {
    let nodes = state.nodes.read();
    let node_list: Vec<RegisteredNode> = nodes.values().cloned().collect();
    let count = node_list.len();

    Json(NodesResponse {
        ok: true,
        count,
        nodes: node_list,
    })
}

/// POST /api/beacon/heartbeat - Accept heartbeat from Constellation node
#[derive(Deserialize)]
pub struct HeartbeatRequest {
    pub node_id: String,
}

#[derive(Serialize)]
pub struct HeartbeatResponse {
    pub ok: bool,
    pub message: String,
}

pub async fn heartbeat(
    State(state): State<BeaconState>,
    Json(req): Json<HeartbeatRequest>,
) -> Result<Json<HeartbeatResponse>, (StatusCode, String)> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let mut nodes = state.nodes.write();

    if let Some(node) = nodes.get_mut(&req.node_id) {
        node.last_heartbeat = now;

        tracing::debug!("[BEACON] 💓 Heartbeat from node: {}", req.node_id);

        Ok(Json(HeartbeatResponse {
            ok: true,
            message: "Heartbeat acknowledged".to_string(),
        }))
    } else {
        Err((
            StatusCode::NOT_FOUND,
            format!("Node {} not registered", req.node_id),
        ))
    }
}

/// Issue a signed NodePassport
///
/// Creates a passport with 7-day expiry, signs it with guardian keypair,
/// and returns it for the node to store.
fn issue_passport(node_tag: &str, now: u64) -> Result<crate::passport::NodePassport, String> {
    const SEVEN_DAYS: u64 = 7 * 24 * 60 * 60;
    const NODE_VERSION: u32 = 111; // v1.1.1

    // Load guardian keypair from keys.json
    let keys_json = std::fs::read_to_string("keys.json")
        .map_err(|e| format!("Failed to read keys.json: {}", e))?;

    let keys: serde_json::Value = serde_json::from_str(&keys_json)
        .map_err(|e| format!("Failed to parse keys.json: {}", e))?;

    let sk_hex = keys["secret_key"]
        .as_str()
        .ok_or_else(|| "secret_key not found in keys.json".to_string())?;
    let pk_hex = keys["public_key"]
        .as_str()
        .ok_or_else(|| "public_key not found in keys.json".to_string())?;

    let sk_bytes = hex::decode(sk_hex).map_err(|e| format!("Invalid secret_key hex: {}", e))?;
    let pk_bytes = hex::decode(pk_hex).map_err(|e| format!("Invalid public_key hex: {}", e))?;

    // Create ed25519 keypair (64 bytes: [32 secret][32 public])
    let mut keypair_bytes = sk_bytes.to_vec();
    keypair_bytes.extend_from_slice(&pk_bytes);
    let keypair_array: [u8; 64] = keypair_bytes
        .try_into()
        .map_err(|_| "Invalid keypair bytes length".to_string())?;

    let keypair = SigningKey::from_keypair_bytes(&keypair_array)
        .map_err(|e| format!("Failed to create keypair: {}", e))?;

    // Create passport without signature
    let mut passport = crate::passport::NodePassport {
        node_tag: node_tag.to_string(),
        role: "dreamer".to_string(),
        network: "testnet".to_string(), // TODO: get from config
        issued_at: now,
        expires_at: now + SEVEN_DAYS,
        max_peers: 32,
        min_version: NODE_VERSION,
        guardian_pubkey: pk_hex.to_string(),
        signature: Vec::new(), // Will be filled after signing
    };

    // Serialize passport (without signature) for signing
    let mut signing_passport = passport.clone();
    signing_passport.signature = Vec::new();

    let passport_json = serde_json::to_vec(&signing_passport)
        .map_err(|e| format!("Failed to serialize passport: {}", e))?;

    // Sign the passport
    let signature = keypair.sign(&passport_json);
    passport.signature = signature.to_bytes().to_vec();

    Ok(passport)
}

/// GET /api/beacon/health - Beacon health check
#[derive(Serialize)]
pub struct HealthResponse {
    pub ok: bool,
    pub mode: String,
    pub registered_nodes: usize,
}

pub async fn health(State(state): State<BeaconState>) -> Json<HealthResponse> {
    let nodes = state.nodes.read();

    Json(HealthResponse {
        ok: true,
        mode: "guardian".to_string(),
        registered_nodes: nodes.len(),
    })
}
