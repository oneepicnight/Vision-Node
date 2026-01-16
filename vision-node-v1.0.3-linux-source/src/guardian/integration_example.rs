// Example integration points for Guardian event webhooks
// Add these to src/main.rs at appropriate locations

/*
=============================================================================
INTEGRATION POINT 1: First Block Detection
=============================================================================

Add this after accepting a new block in the mining logic:

*/

// After validating and accepting a new block
async fn on_new_block_accepted(block: &Block) {
    let chain = CHAIN.lock();
    let height = chain.blocks.len();
    
    // Check if this is the FIRST BLOCK on Constellation
    if height == 1 {
        let miner = block.header.miner.clone();
        let hash = block.header.pow_hash.clone();
        drop(chain);
        
        info!("[CONSTELLATION] 🌟 FIRST BLOCK mined by {}", miner);
        
        // Look up Discord ID for this miner (if tracked)
        // TODO: Implement miner tracking system
        let discord_id = lookup_discord_id_for_miner(&miner).await;
        
        // Send event to Vision Bot
        crate::guardian::notify_first_block(
            discord_id,
            1,
            hash,
            miner,
        ).await;
    }
}

/*
=============================================================================
INTEGRATION POINT 2: Node Status Monitoring
=============================================================================

Add this as a background task in tokio::spawn:

*/

async fn monitor_tracked_nodes() {
    let mut interval = tokio::time::interval(Duration::from_secs(30));
    
    loop {
        interval.tick().await;
        
        // Get list of tracked miners (Discord users who ran /constellation-test)
        let tracked = get_tracked_miners().await;
        
        for tracker in tracked {
            // Check if their node is online
            let is_online = check_node_health(&tracker.peer_address).await;
            
            // Detect status change
            if is_online != tracker.was_online {
                info!(
                    "[GUARDIAN] Node {} changed status: {} -> {}",
                    tracker.miner_address,
                    if tracker.was_online { "online" } else { "offline" },
                    if is_online { "online" } else { "offline" }
                );
                
                // Send event to Vision Bot
                crate::guardian::notify_node_status(
                    Some(tracker.discord_user_id.clone()),
                    if is_online { "online" } else { "offline" }.to_string(),
                    tracker.miner_address.clone(),
                ).await;
                
                // Update tracker state
                update_tracker_online_status(tracker.id, is_online).await;
            }
        }
    }
}

/*
=============================================================================
HELPER FUNCTIONS
=============================================================================
*/

// TODO: Implement miner tracking database
// Option 1: Use Guardian's SQLite peer_tracker
// Option 2: Add to CHAIN state
// Option 3: External service/API

#[derive(Clone)]
struct TrackedMiner {
    id: u64,
    discord_user_id: String,
    miner_address: String,
    peer_address: String,
    was_online: bool,
}

async fn get_tracked_miners() -> Vec<TrackedMiner> {
    // TODO: Query from database
    // For now, return empty list
    vec![]
}

async fn lookup_discord_id_for_miner(miner: &str) -> Option<String> {
    // TODO: Query from tracking database
    // Look up which Discord user owns this miner address
    None
}

async fn check_node_health(peer_addr: &str) -> bool {
    // Check if node is responding
    let url = format!("http://{}/health", peer_addr);
    
    match reqwest::Client::new()
        .get(&url)
        .timeout(Duration::from_secs(5))
        .send()
        .await
    {
        Ok(response) => response.status().is_success(),
        Err(_) => false,
    }
}

async fn update_tracker_online_status(tracker_id: u64, is_online: bool) {
    // TODO: Update database
    // UPDATE tracked_miners SET was_online = ? WHERE id = ?
}

/*
=============================================================================
STARTUP INTEGRATION
=============================================================================

Add to main() function after Guardian initialization:

*/

// Start node status monitoring (if in guardian mode)
if is_guardian_mode() {
    tokio::spawn(async {
        info!("[GUARDIAN] Starting node status monitoring for tracked miners");
        monitor_tracked_nodes().await;
    });
}

/*
=============================================================================
DATABASE SCHEMA (if using SQLite)
=============================================================================

CREATE TABLE tracked_miners (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    discord_user_id TEXT NOT NULL,
    discord_username TEXT,
    miner_address TEXT NOT NULL UNIQUE,
    peer_address TEXT,
    was_online INTEGER DEFAULT 0,
    first_seen_at INTEGER NOT NULL,
    last_status_check_at INTEGER,
    UNIQUE(discord_user_id, miner_address)
);

CREATE INDEX idx_miner_address ON tracked_miners(miner_address);
CREATE INDEX idx_peer_address ON tracked_miners(peer_address);
CREATE INDEX idx_discord_user_id ON tracked_miners(discord_user_id);

*/

/*
=============================================================================
VISION BOT ENDPOINT (Discord bot side - TypeScript/Node.js)
=============================================================================

app.post('/guardian', async (req: Request, res: Response) => {
  const event = req.body;
  
  try {
    if (event.event === 'first_block') {
      const { discord_user_id, height, hash, miner_address } = event;
      
      // Find the user in Discord
      const user = await client.users.fetch(discord_user_id);
      const guild = await client.guilds.fetch(GUILD_ID);
      const member = await guild.members.fetch(discord_user_id);
      
      // Post FIRST CONTACT ceremony message
      const channel = await client.channels.fetch(CONSTELLATION_CHANNEL_ID);
      await channel.send({
        embeds: [{
          title: "🌟 FIRST CONTACT 🌟",
          description: `**${user.username}** has mined the first block on the Constellation!`,
          color: 0x00D9FF,
          fields: [
            { name: "Height", value: `${height}`, inline: true },
            { name: "Miner", value: miner_address, inline: false },
            { name: "Block Hash", value: hash, inline: false },
          ],
          timestamp: new Date(),
        }]
      });
      
      // Give role "First Star of the Constellation"
      const role = guild.roles.cache.find(r => r.name === "First Star of the Constellation");
      if (role) {
        await member.roles.add(role);
        await channel.send(`✨ ${user} has been crowned **First Star of the Constellation**!`);
      }
    }
    
    if (event.event === 'node_status') {
      const { discord_user_id, status, miner_address } = event;
      
      const user = await client.users.fetch(discord_user_id);
      const channel = await client.channels.fetch(CONSTELLATION_CHANNEL_ID);
      
      // Post Guardian salute
      if (status === 'online') {
        await channel.send(
          `🟢 **Guardian Salute:** ${user.username}'s node is ONLINE and ready.`
        );
      } else {
        await channel.send(
          `🔴 **Guardian Alert:** ${user.username}'s node has gone OFFLINE.`
        );
      }
    }
    
    res.status(200).send('OK');
  } catch (error) {
    console.error('Error handling guardian event:', error);
    res.status(500).send('Error');
  }
});

*/
