// Vision Node - NEW_STAR Event Tracker
// Add to: C:\vision-node\src\guardian\peer_tracker.rs

use rusqlite::{Connection, params};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct PeerTracker {
    db_path: String,
}

impl PeerTracker {
    pub fn new(db_path: String) -> Self {
        Self { db_path }
    }

    /// Initialize the database with NEW_STAR events table
    pub fn init_db(&self) -> Result<(), Box<dyn std::error::Error>> {
        let conn = Connection::open(&self.db_path)?;
        
        conn.execute(
            "CREATE TABLE IF NOT EXISTS new_star_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                peer_id TEXT NOT NULL UNIQUE,
                alias TEXT,
                first_seen_at INTEGER NOT NULL,
                announced_at INTEGER,
                latitude REAL,
                longitude REAL,
                region TEXT,
                CONSTRAINT unique_peer UNIQUE(peer_id)
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_peer_id ON new_star_events(peer_id)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_first_seen ON new_star_events(first_seen_at DESC)",
            [],
        )?;

        Ok(())
    }

    /// Check if peer has been seen before (debounce logic)
    pub fn has_seen_peer(&self, peer_id: &str) -> Result<bool, Box<dyn std::error::Error>> {
        let conn = Connection::open(&self.db_path)?;
        let mut stmt = conn.prepare("SELECT COUNT(*) FROM new_star_events WHERE peer_id = ?")?;
        let count: i64 = stmt.query_row(params![peer_id], |row| row.get(0))?;
        Ok(count > 0)
    }

    /// Log a NEW_STAR event when peer connects for the first time
    pub fn log_new_star(
        &self,
        peer_id: &str,
        alias: Option<&str>,
        latitude: Option<f64>,
        longitude: Option<f64>,
        region: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let conn = Connection::open(&self.db_path)?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs() as i64;

        conn.execute(
            "INSERT OR IGNORE INTO new_star_events 
             (peer_id, alias, first_seen_at, latitude, longitude, region)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![peer_id, alias, now, latitude, longitude, region],
        )?;

        Ok(())
    }

    /// Called when a peer connects to the network
    pub fn on_peer_connected(
        &self,
        peer_id: &str,
        alias: Option<&str>,
        latitude: Option<f64>,
        longitude: Option<f64>,
        region: Option<&str>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        // Check if this is a new star (first time seeing this peer)
        if !self.has_seen_peer(peer_id)? {
            self.log_new_star(peer_id, alias, latitude, longitude, region)?;
            println!("✨ NEW_STAR event logged: {} ({})", 
                alias.unwrap_or("Unknown"), peer_id);
            Ok(true) // New star
        } else {
            Ok(false) // Already seen
        }
    }

    /// Get new stars since a given timestamp (for API endpoint)
    pub fn get_new_stars_since(
        &self,
        since: i64,
        limit: usize,
    ) -> Result<Vec<NewStarEvent>, Box<dyn std::error::Error>> {
        let conn = Connection::open(&self.db_path)?;
        let mut stmt = conn.prepare(
            "SELECT peer_id, alias, first_seen_at, region, latitude, longitude
             FROM new_star_events
             WHERE first_seen_at >= ?1
             ORDER BY first_seen_at DESC
             LIMIT ?2"
        )?;

        let events = stmt.query_map(params![since, limit], |row| {
            Ok(NewStarEvent {
                peer_id: row.get(0)?,
                alias: row.get(1)?,
                first_seen_at: row.get(2)?,
                region: row.get(3)?,
                latitude: row.get(4)?,
                longitude: row.get(5)?,
            })
        })?;

        let mut result = Vec::new();
        for event in events {
            result.push(event?);
        }

        Ok(result)
    }

    /// Mark a star as announced (Discord bot update)
    pub fn mark_announced(&self, peer_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let conn = Connection::open(&self.db_path)?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs() as i64;

        conn.execute(
            "UPDATE new_star_events SET announced_at = ?1 WHERE peer_id = ?2",
            params![now, peer_id],
        )?;

        Ok(())
    }
}

#[derive(Debug)]
pub struct NewStarEvent {
    pub peer_id: String,
    pub alias: Option<String>,
    pub first_seen_at: i64,
    pub region: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}
