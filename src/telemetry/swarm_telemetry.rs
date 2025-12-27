#![allow(dead_code)]
// src/telemetry/swarm_telemetry.rs
use maxminddb::geoip2;
use maxminddb::Reader;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use sysinfo::System;
use tokio::time::sleep;
use tracing::{info, warn};

use crate::p2p::peer_store::PeerStore;

pub struct SwarmTelemetryConfig {
    pub swarm_viz_enabled: bool,
    pub geoip_db_path: Option<String>,
}

/// ASCII startup banner with version and mode info
pub fn print_startup_banner(version: &str, net_name: &str, mode: &str) {
    info!("");
    info!("   __      ___      _                 _   _           _      ");
    info!("   \\ \\    / (_)    (_)               | \\ | |         | |     ");
    info!("    \\ \\  / / _ ___ _  ___  _ __      |  \\| | ___   __| | ___ ");
    info!("     \\ \\/ / | / __| |/ _ \\| '_ \\     | . ` |/ _ \\ / _` |/ _ \\");
    info!("      \\  /  | \\__ \\ | (_) | | | |    | |\\  | (_) | (_| |  __/");
    info!("       \\/   |_|___/_|\\___/|_| |_|    |_| \\_|\\___/ \\__,_|\\___|");
    info!("");
    info!("🌌 Vision Node starting up...");
    info!("   Version : {}", version);
    info!("   Network : {}", net_name);
    info!("   Mode    : {}", mode);
    info!("");
}

/// Log CPU topology and thread information
pub fn log_cpu_topology() {
    let sys = System::new_all();

    let cpus = sys.cpus();
    let physical_cores = sys.physical_core_count().unwrap_or(cpus.len());
    let logical_cores = cpus.len();
    let model = cpus
        .first()
        .map(|c| c.brand().to_string())
        .unwrap_or_else(|| "Unknown CPU".to_string());

    info!(
        "[HW] CPU: {} | physical cores: {} | logical threads: {}",
        model, physical_cores, logical_cores
    );
}

/// Simple peer discovery 'animation' in logs with spinner frames
pub async fn run_peer_discovery_animation(
    get_peer_counts: impl Fn() -> (usize, usize) + Send + Sync + 'static,
) {
    let get_peer_counts = Arc::new(get_peer_counts);
    let frames = ["⠋", "⠙", "⠚", "⠞", "⠖", "⠦", "⠴", "⠲", "⠳", "⠓"];

    tokio::spawn({
        let get_peer_counts = get_peer_counts.clone();
        async move {
            let mut i = 0usize;
            loop {
                let (connected, known) = get_peer_counts();
                let frame = frames[i % frames.len()];
                info!(
                    "[SWARM] {} discovering peers... connected={} known={}",
                    frame, connected, known
                );
                i += 1;
                sleep(Duration::from_secs(5)).await;
            }
        }
    });
}

/// Snapshot style 'swarm visualization' showing peer stats
pub async fn run_swarm_visualizer(
    get_peer_store: impl Fn() -> Option<PeerStore> + Send + Sync + 'static,
) {
    let get_peer_store = Arc::new(get_peer_store);

    tokio::spawn(async move {
        loop {
            if let Some(store) = get_peer_store() {
                let peers = store.get_all();
                let total = peers.len();
                let connected = peers
                    .iter()
                    .filter(|p| p.connection_status == "connected")
                    .count();
                let trusted = peers.iter().filter(|p| p.trusted).count();
                let seeds = peers.iter().filter(|p| p.is_seed).count();

                info!(
                    "[SWARM VIZ] peers={} connected={} trusted={} seeds={}",
                    total, connected, trusted, seeds
                );
            }
            sleep(Duration::from_secs(30)).await;
        }
    });
}

/// Uptime badge based on node lifetime + peer connections
pub fn uptime_badge(start_time: Instant, connected_peers: usize) -> &'static str {
    let uptime = start_time.elapsed();
    if connected_peers >= 10 && uptime.as_secs() > 60 * 60 {
        "🌌 IMMORTAL"
    } else if connected_peers >= 5 && uptime.as_secs() > 15 * 60 {
        "🔥 STEADY"
    } else if uptime.as_secs() > 5 * 60 {
        "⚡ WARMING UP"
    } else {
        "✨ NEW STAR"
    }
}

// ============================================================================
// GeoIP Integration for Constellation Heatmap
// ============================================================================

/// Global GeoIP database reader (loaded once at startup)
static GEOIP_READER: Lazy<Option<Reader<Vec<u8>>>> = Lazy::new(|| {
    if let Ok(path) = std::env::var("VISION_GEOIP_DB") {
        match Reader::open_readfile(&path) {
            Ok(reader) => {
                info!("[GEOIP] Loaded GeoIP DB from: {}", path);
                Some(reader)
            }
            Err(e) => {
                warn!("[GEOIP] Failed to load GeoIP DB from {}: {}", path, e);
                None
            }
        }
    } else {
        info!("[GEOIP] VISION_GEOIP_DB not set; constellation heatmap disabled");
        None
    }
});

/// Lookup country code for an IP address
fn lookup_country_code(ip: &str) -> Option<String> {
    let reader = GEOIP_READER.as_ref()?;
    let parsed: IpAddr = ip.parse().ok()?;
    let city: geoip2::City = reader.lookup(parsed).ok()?;
    city.country.and_then(|c| c.iso_code).map(|s| s.to_string())
}

/// Run constellation geographic heatmap visualization
/// Logs distribution of peers across countries every 2 minutes
pub async fn run_constellation_heatmap(
    get_peer_store: impl Fn() -> Option<PeerStore> + Send + Sync + 'static,
) {
    let get_peer_store = Arc::new(get_peer_store);

    tokio::spawn(async move {
        loop {
            // Only run if GeoIP is available
            if GEOIP_READER.is_some() {
                if let Some(store) = get_peer_store() {
                    let peers = store.get_all();
                    let mut counts: HashMap<String, usize> = HashMap::new();

                    // Count peers by country
                    for p in peers {
                        if let Some(addr) = p.ip_address {
                            // Extract IP without port
                            let ip = addr.split(':').next().unwrap_or(&addr).to_string();
                            if let Some(code) = lookup_country_code(&ip) {
                                *counts.entry(code).or_insert(0) += 1;
                            }
                        }
                    }

                    // Log heatmap if we have data
                    if !counts.is_empty() {
                        let mut entries: Vec<_> = counts.into_iter().collect();
                        entries.sort_by(|a, b| b.1.cmp(&a.1));

                        info!("[CONSTELLATION] Geo heatmap (top regions):");
                        for (code, count) in entries.into_iter().take(10) {
                            let bar = "█".repeat(count.min(20));
                            info!("  {:>3} | {:>3} {}", code, count, bar);
                        }
                    } else {
                        info!("[CONSTELLATION] No peer geo data available yet");
                    }
                }
            }

            sleep(Duration::from_secs(120)).await;
        }
    });
}

/// Check if GeoIP is enabled (for conditional startup)
pub fn is_geoip_enabled() -> bool {
    GEOIP_READER.is_some()
}
