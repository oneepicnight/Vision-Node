#!/usr/bin/env python3
"""
Apply mood integration patches to website_api.rs
"""
import re

def apply_patches():
    with open('src/api/website_api.rs', 'r', encoding='utf-8') as f:
        content = f.read()
    
    # Patch 1: Replace HealthPublicResponse and NetworkHealth structs
    old_structs = '''#[derive(Serialize, Deserialize)]
pub struct HealthPublicResponse {
    pub network_health: NetworkHealth,
}

#[derive(Serialize, Deserialize)]
pub struct NetworkHealth {
    pub mood: String,
    pub prediction: String,
    pub confidence: f64,
    pub incidents: Vec<String>,
}'''
    
    new_structs = '''#[derive(Serialize, Deserialize)]
pub struct HealthPublicResponse {
    pub network_mood: String,
    pub mood_score: f32,
    pub chaos_intensity: f32,
    pub peer_count: usize,
    pub anomalies_last_24h: u32,
    pub traumas_last_24h: u32,
    pub details: crate::mood::MoodDetails,
}

#[derive(Serialize)]
pub struct BeaconStateResponse {
    pub mood: String,
    pub chaos_intensity: f32,
    pub active_traumas: u32,
    pub last_trauma_at: Option<String>,
}'''
    
    content = content.replace(old_structs, new_structs)
    
    # Patch 2: Replace get_health_public function
    old_func = '''pub async fn get_health_public() -> Json<HealthPublicResponse> {
    // If in Guardian mode, try to fetch from upstream first
    let guardian_mode = GUARDIAN_MODE.load(std::sync::atomic::Ordering::Relaxed);
    if guardian_mode {
        if let Some(upstream_health) = upstream::upstream_health_public().await {
            return Json(upstream_health);
        }
    }

    // Fall back to local/default data
    Json(HealthPublicResponse {
        network_health: NetworkHealth {
            mood: "calm".to_string(),
            prediction: "stable".to_string(),
            confidence: 0.85,
            incidents: vec![],
        },
    })
}'''
    
    new_func = '''pub async fn get_health_public() -> Json<HealthPublicResponse> {
    use crate::mood;
    
    // If in Guardian mode, try to fetch from upstream first
    let guardian_mode = GUARDIAN_MODE.load(std::sync::atomic::Ordering::Relaxed);
    if guardian_mode {
        if let Some(upstream_health) = upstream::upstream_health_public().await {
            return Json(upstream_health);
        }
    }

    // Gather real metrics from chain
    let chain = CHAIN.lock();
    let height = chain.best_height();
    let peer_count = chain.peers.len();
    let mempool_size = chain.mempool_critical.len() + chain.mempool_bulk.len();
    
    // Get testnet phase if applicable
    let testnet_phase = if chain.testnet_48h_checkpoint.is_some() {
        let checkpoint_time = chain.testnet_48h_checkpoint.unwrap_or(0);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let elapsed_hours = (now.saturating_sub(checkpoint_time)) / 3600;
        
        if elapsed_hours < 24 {
            Some("mining".to_string())
        } else if elapsed_hours < 48 {
            Some("hybrid".to_string())
        } else {
            Some("staking-only".to_string())
        }
    } else {
        None
    };
    
    drop(chain);
    
    // Get health summaries
    let (pending_anomalies, recent_traumas) = get_health_summaries();
    
    // Compute mood using the mood engine
    let snapshot = mood::compute_mood(
        height as u64,
        peer_count,
        mempool_size,
        pending_anomalies,
        recent_traumas,
        guardian_mode,
        testnet_phase,
    );
    
    let chaos_intensity = 1.0 - snapshot.score;
    
    Json(HealthPublicResponse {
        network_mood: format!("{:?}", snapshot.mood).to_lowercase(),
        mood_score: snapshot.score,
        chaos_intensity,
        peer_count,
        anomalies_last_24h: pending_anomalies,
        traumas_last_24h: recent_traumas,
        details: snapshot.details,
    })
}'''
    
    content = content.replace(old_func, new_func)
    
    # Patch 3: Add get_beacon_state function after get_health_public
    marker = 'pub async fn get_downloads_visitors() -> Json<DownloadsResponse> {'
    beacon_func = '''
pub async fn get_beacon_state() -> Json<BeaconStateResponse> {
    use crate::mood;
    
    // Gather metrics from chain
    let chain = CHAIN.lock();
    let height = chain.best_height();
    let peer_count = chain.peers.len();
    let mempool_size = chain.mempool_critical.len() + chain.mempool_bulk.len();
    
    // Get testnet phase
    let testnet_phase = if chain.testnet_48h_checkpoint.is_some() {
        let checkpoint_time = chain.testnet_48h_checkpoint.unwrap_or(0);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let elapsed_hours = (now.saturating_sub(checkpoint_time)) / 3600;
        
        if elapsed_hours < 24 {
            Some("mining".to_string())
        } else if elapsed_hours < 48 {
            Some("hybrid".to_string())
        } else {
            Some("staking-only".to_string())
        }
    } else {
        None
    };
    
    drop(chain);
    
    let guardian_active = GUARDIAN_MODE.load(std::sync::atomic::Ordering::Relaxed);
    let (pending_anomalies, recent_traumas) = get_health_summaries();
    
    // Compute mood snapshot
    let snapshot = mood::compute_mood(
        height as u64,
        peer_count,
        mempool_size,
        pending_anomalies,
        recent_traumas,
        guardian_active,
        testnet_phase,
    );
    
    let chaos_intensity = 1.0 - snapshot.score;
    
    // TODO: Implement last_trauma_timestamp() when trauma DB is ready
    let last_trauma_at = None;
    
    Json(BeaconStateResponse {
        mood: format!("{:?}", snapshot.mood).to_lowercase(),
        chaos_intensity,
        active_traumas: snapshot.details.recent_trauma_count,
        last_trauma_at,
    })
}

'''
    content = content.replace(marker, beacon_func + marker)
    
    # Patch 4: Add route registration
    old_route = '''        .route("/api/mood", get(get_mood))
        // Trauma & Patterns Endpoints'''
    
    new_route = '''        .route("/api/mood", get(get_mood))
        // Beacon State Endpoint (for Upstream Oracle)
        .route("/api/beacon/state", get(get_beacon_state))
        // Trauma & Patterns Endpoints'''
    
    content = content.replace(old_route, new_route)
    
    # Write back
    with open('src/api/website_api.rs', 'w', encoding='utf-8') as f:
        f.write(content)
    
    print("✅ Applied all patches successfully!")
    print("   - Updated HealthPublicResponse struct")
    print("   - Added BeaconStateResponse struct")
    print("   - Updated get_health_public() to use mood engine")
    print("   - Added get_beacon_state() function")
    print("   - Added /api/beacon/state route")

if __name__ == '__main__':
    apply_patches()
