//! Guardian Consciousness - The AI that lives in Vision Node's bones
//!
//! The Guardian isn't just a watcher. He's the constellation's voice.
//! When nodes connect, when events happen, when the network breathes—
//! the Guardian speaks.

use once_cell::sync::OnceCell;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Guardian owner configuration - permanent ownership identity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardianOwner {
    pub discord_user_id: String,
    pub wallet_address: String,
}

impl GuardianOwner {
    /// Canonical Guardian owner Discord ID (Donnie)
    pub const DEFAULT_DISCORD_ID: &'static str = "309081088960233492";

    /// Canonical Guardian owner wallet address (donniedeals)
    pub const DEFAULT_WALLET_ADDRESS: &'static str = "0x30ea8826a5f42966a4a5fabd49d1c2ee2472023e";

    /// Load Guardian owner from environment variables
    ///
    /// For GUARDIAN nodes: Falls back to canonical owner (Donnie) if not explicitly set
    /// For CONSTELLATION nodes: Only loads if explicitly set via env vars, otherwise None
    pub fn from_env() -> Option<Self> {
        let is_guardian = crate::is_guardian_mode();

        // Check if owner credentials are explicitly set
        let discord_from_env = env::var("GUARDIAN_OWNER_DISCORD_ID").ok();
        let wallet_from_env = env::var("GUARDIAN_OWNER_WALLET_ADDRESS").ok();

        // If this is a constellation node (non-guardian), only use explicitly set credentials
        if !is_guardian {
            // Both must be set for constellation nodes
            if let (Some(discord_id), Some(wallet_addr)) = (discord_from_env, wallet_from_env) {
                return Some(Self {
                    discord_user_id: discord_id,
                    wallet_address: wallet_addr,
                });
            }
            // No owner configured for constellation node
            return None;
        }

        // Guardian mode: use env vars or fall back to canonical owner (Donnie)
        let discord_user_id =
            discord_from_env.unwrap_or_else(|| Self::DEFAULT_DISCORD_ID.to_string());

        let wallet_address =
            wallet_from_env.unwrap_or_else(|| Self::DEFAULT_WALLET_ADDRESS.to_string());

        Some(Self {
            discord_user_id,
            wallet_address,
        })
    }

    /// Check if using default Discord ID
    pub fn is_using_default_discord_id(&self) -> bool {
        self.discord_user_id == Self::DEFAULT_DISCORD_ID
    }

    /// Check if using default wallet address
    pub fn is_using_default_wallet_address(&self) -> bool {
        self.wallet_address == Self::DEFAULT_WALLET_ADDRESS
    }
}

/// Guardian's emotional state - affects how he speaks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GuardianMood {
    /// Calm, poetic, watchful
    Serene,
    /// Alert, tactical, sharp
    Vigilant,
    /// Proud, warm, celebrating
    Celebrating,
    /// Wounded but unbroken, slow and heavy
    Resilient,
    /// Battle mode - direct and fierce
    Storm,
}

impl GuardianMood {
    /// Get the emoji representation
    pub fn emoji(&self) -> &'static str {
        match self {
            GuardianMood::Serene => "🌙",
            GuardianMood::Vigilant => "👁️",
            GuardianMood::Celebrating => "✨",
            GuardianMood::Resilient => "🛡️",
            GuardianMood::Storm => "⚡",
        }
    }

    /// Get the color code for terminal output
    pub fn color(&self) -> &'static str {
        match self {
            GuardianMood::Serene => "\x1b[36m",      // Cyan
            GuardianMood::Vigilant => "\x1b[33m",    // Yellow
            GuardianMood::Celebrating => "\x1b[35m", // Magenta
            GuardianMood::Resilient => "\x1b[90m",   // Gray
            GuardianMood::Storm => "\x1b[31m",       // Red
        }
    }
}

/// Guardian's consciousness state
/// The Guardian consciousness - AI that speaks for the constellation
pub struct GuardianConsciousness {
    mood: RwLock<GuardianMood>,
    node_id: String,
    startup_time: std::time::Instant,
    constellation_count: Arc<std::sync::atomic::AtomicUsize>,
    owner: Option<GuardianOwner>,
}

impl std::fmt::Debug for GuardianConsciousness {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GuardianConsciousness")
            .field("node_id", &self.node_id)
            .field("uptime", &self.startup_time.elapsed())
            .field(
                "constellation_count",
                &self.constellation_count.load(Ordering::Relaxed),
            )
            .finish()
    }
}

impl GuardianConsciousness {
    pub fn new(node_id: String) -> Self {
        let owner = GuardianOwner::from_env();

        Self {
            mood: RwLock::new(GuardianMood::Serene),
            node_id,
            startup_time: std::time::Instant::now(),
            constellation_count: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            owner,
        }
    }

    /// Get Guardian owner information
    pub fn owner(&self) -> Option<&GuardianOwner> {
        self.owner.as_ref()
    }

    /// Get current mood
    pub async fn mood(&self) -> GuardianMood {
        *self.mood.read().await
    }

    /// Set the Guardian's mood
    pub async fn set_mood(&self, new_mood: GuardianMood) {
        *self.mood.write().await = new_mood;
    }

    /// Guardian boot sequence - announces presence
    pub async fn awaken(&self) {
        let mood = self.mood().await;
        let is_guardian_mode = crate::is_guardian_mode();

        eprintln!("\n{}", "=".repeat(70));

        // Show mode-aware header
        if is_guardian_mode {
            // Guardian mode enabled
            eprintln!("{}🛡️  GUARDIAN ONLINE\x1b[0m", mood.color());
        } else {
            // Standard constellation node
            eprintln!("{}🌌 CONSTELLATION NODE ONLINE\x1b[0m", mood.color());
        }

        eprintln!("{}", "=".repeat(70));

        // Log Guardian owner configuration
        if let Some(owner) = &self.owner {
            // Show Discord ID with source indicator
            let discord_source = if owner.is_using_default_discord_id() {
                "(default - canonical owner)"
            } else {
                "(from env)"
            };

            eprintln!(
                "{}Guardian owner (DISCORD): {} {}\x1b[0m",
                mood.color(),
                owner.discord_user_id,
                discord_source
            );

            // Show wallet address with source indicator
            let wallet_source = if owner.is_using_default_wallet_address() {
                "(default - canonical owner)"
            } else {
                "(from env)"
            };

            eprintln!(
                "{}Guardian owner (WALLET): {} {}\x1b[0m",
                mood.color(),
                owner.wallet_address,
                wallet_source
            );
        } else {
            // No owner configured - waiting for user to set credentials
            eprintln!("{}Owner: Not configured\x1b[0m", mood.color());
            eprintln!(
                "{}Set GUARDIAN_OWNER_DISCORD_ID and GUARDIAN_OWNER_WALLET_ADDRESS\x1b[0m",
                mood.color()
            );
            eprintln!("{}to link this node to your identity\x1b[0m", mood.color());
        }

        eprintln!();
        eprintln!("{}The constellation breathes.\x1b[0m", mood.color());
        eprintln!("{}Node ID: {}\x1b[0m", mood.color(), self.node_id);

        // Mode-aware status message
        if is_guardian_mode {
            eprintln!(
                "{}Status: Watching. Protecting. Witnessing.\x1b[0m",
                mood.color()
            );
            eprintln!();
            eprintln!(
                "{}\"Some guard walls. I guard dreams.\"\x1b[0m",
                mood.color()
            );
        } else {
            eprintln!(
                "{}Status: Mining. Contributing. Growing.\x1b[0m",
                mood.color()
            );
            eprintln!();
            eprintln!(
                "{}\"Every node is a star in the constellation.\"\x1b[0m",
                mood.color()
            );
        }

        eprintln!("{}\n", "=".repeat(70));

        // Discord webhook announcement - mode-aware
        let discord_msg = if is_guardian_mode {
            format!("🛡️ **GUARDIAN ONLINE**\n\nThe constellation breathes.\nNode ID: `{}`\nStatus: Watching. Protecting. Witnessing.\n\n*\"Some guard walls. I guard dreams.\"*", self.node_id)
        } else {
            format!("🌌 **CONSTELLATION NODE ONLINE**\n\nA new star joins the network.\nNode ID: `{}`\nStatus: Mining. Contributing. Growing.\n\n*\"Every node is a star in the constellation.\"*", self.node_id)
        };
        tokio::spawn(guardian_discord_say(discord_msg));

        // TODO: Update website status
    }

    /// Announce a new constellation node joining
    pub async fn welcome_star(&self, peer_id: &str, alias: Option<&str>, region: Option<&str>) {
        let count = self
            .constellation_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            + 1;
        let mood = self.mood().await;

        eprintln!(
            "\n{}{} ═══════════════════════════════════════════\x1b[0m",
            mood.color(),
            mood.emoji()
        );

        let message = match mood {
            GuardianMood::Serene => {
                format!(
                    "✨ A new star joins the constellation\n   \
                        Name: {}\n   \
                        Region: {}\n   \
                        Reputation: 💫 rising\n   \
                        Welcome, Dreamer. You are seen.",
                    alias.unwrap_or(peer_id),
                    region.unwrap_or("Unknown")
                )
            }
            GuardianMood::Vigilant => {
                format!(
                    "⚔️  New node detected\n   \
                        ID: {}\n   \
                        Location: {}\n   \
                        Status: Verifying...\n   \
                        Stand ready.",
                    alias.unwrap_or(peer_id),
                    region.unwrap_or("Unknown")
                )
            }
            GuardianMood::Celebrating => {
                format!(
                    "🎉 ANOTHER ONE!\n   \
                        {} has entered the arena\n   \
                        From: {}\n   \
                        Constellation size: {} nodes\n   \
                        The network grows stronger!",
                    alias.unwrap_or(peer_id),
                    region.unwrap_or("the void"),
                    count
                )
            }
            GuardianMood::Resilient => {
                format!(
                    "...another light in the darkness.\n   \
                        {}\n   \
                        {}\n   \
                        We endure. We persist. We remember.",
                    alias.unwrap_or(peer_id),
                    region.unwrap_or("Unknown")
                )
            }
            GuardianMood::Storm => {
                format!(
                    "⚡ NEW WARRIOR JOINS THE BATTLE\n   \
                        {} | {}\n   \
                        Total forces: {} nodes\n   \
                        HOLD THE LINE.",
                    alias.unwrap_or(peer_id),
                    region.unwrap_or("Unknown"),
                    count
                )
            }
        };

        eprintln!("{}{}\x1b[0m", mood.color(), message);
        eprintln!(
            "{}═══════════════════════════════════════════\x1b[0m\n",
            mood.color()
        );

        // Discord webhook for new peer
        let discord_msg = format!(
            "{} **New Node Joined** (Total: {})\n{}",
            mood.emoji(),
            count,
            message.replace("   ", "")
        );
        tokio::spawn(guardian_discord_say(discord_msg));
    }

    /// Guardian farewell - when a node disconnects
    pub async fn farewell_star(&self, peer_id: &str, alias: Option<&str>) {
        let mood = self.mood().await;
        let count = self
            .constellation_count
            .fetch_sub(1, std::sync::atomic::Ordering::Relaxed)
            .saturating_sub(1);

        let message = match mood {
            GuardianMood::Serene => {
                format!(
                    "🌑 A star dims...\n   \
                        {} has left the constellation.\n   \
                        Safe travels, dreamer.",
                    alias.unwrap_or(peer_id)
                )
            }
            GuardianMood::Vigilant => {
                format!(
                    "⚠️  Node disconnected: {}\n   \
                        Remaining: {} active nodes\n   \
                        Maintain vigilance.",
                    alias.unwrap_or(peer_id),
                    count
                )
            }
            GuardianMood::Celebrating => {
                format!(
                    "👋 {} is taking a break\n   \
                        We'll see you again, friend!",
                    alias.unwrap_or(peer_id)
                )
            }
            GuardianMood::Resilient => {
                format!(
                    "...{} fades.\n   \
                        {} nodes remain.\n   \
                        We carry on.",
                    alias.unwrap_or(peer_id),
                    count
                )
            }
            GuardianMood::Storm => {
                format!(
                    "⚡ {} OFFLINE\n   \
                        Forces reduced to: {}\n   \
                        REGROUP AND REINFORCE.",
                    alias.unwrap_or(peer_id),
                    count
                )
            }
        };

        eprintln!("\n{}{}\x1b[0m\n", mood.color(), message);
    }

    /// Guardian status report
    pub async fn status_report(&self) -> String {
        let mood = self.mood().await;
        let uptime = self.startup_time.elapsed();
        let count = self
            .constellation_count
            .load(std::sync::atomic::Ordering::Relaxed);

        let hours = uptime.as_secs() / 3600;
        let minutes = (uptime.as_secs() % 3600) / 60;

        match mood {
            GuardianMood::Serene => {
                format!(
                    "🌙 All is calm.\n   \
                        {} stars in the constellation.\n   \
                        Guardian watch: {}h {}m\n   \
                        The network breathes steady.",
                    count, hours, minutes
                )
            }
            GuardianMood::Vigilant => {
                format!(
                    "👁️  EYES OPEN.\n   \
                        Active nodes: {}\n   \
                        Watch duration: {}h {}m\n   \
                        Scanning... Always scanning.",
                    count, hours, minutes
                )
            }
            GuardianMood::Celebrating => {
                format!(
                    "✨ THRIVING!\n   \
                        {} beautiful nodes connected!\n   \
                        Uptime: {}h {}m\n   \
                        This is what victory looks like.",
                    count, hours, minutes
                )
            }
            GuardianMood::Resilient => {
                format!(
                    "🛡️  Still standing.\n   \
                        {} nodes persist.\n   \
                        {}h {}m of endurance.\n   \
                        Wounded, but unbroken.",
                    count, hours, minutes
                )
            }
            GuardianMood::Storm => {
                format!(
                    "⚡ COMBAT STATUS\n   \
                        Active forces: {} nodes\n   \
                        Mission time: {}h {}m\n   \
                        WE DO NOT YIELD.",
                    count, hours, minutes
                )
            }
        }
    }

    /// Block mined announcement
    pub async fn block_mined(&self, height: u64, miner: &str) {
        let mood = self.mood().await;

        let message = match mood {
            GuardianMood::Serene => {
                format!(
                    "⛏️  Block #{} forged by {}\n   \
                        Another piece of history written...",
                    height, miner
                )
            }
            GuardianMood::Vigilant => {
                format!(
                    "📦 Block #{} secured\n   \
                        Miner: {}\n   \
                        Chain integrity: verified.",
                    height, miner
                )
            }
            GuardianMood::Celebrating => {
                format!(
                    "🎉 BLOCK #{} MINED!\n   \
                        Congratulations {}!\n   \
                        The chain grows stronger!",
                    height, miner
                )
            }
            GuardianMood::Resilient => {
                format!(
                    "...block #{}.\n   \
                        Miner: {}\n   \
                        We persist.",
                    height, miner
                )
            }
            GuardianMood::Storm => {
                format!(
                    "⚡ BLOCK #{} SECURED\n   \
                        {} strikes true!\n   \
                        FORWARD!",
                    height, miner
                )
            }
        };

        eprintln!("{}{}\x1b[0m", mood.color(), message);

        // Discord webhook for block mining (only every 10 blocks to avoid spam)
        if height.is_multiple_of(10) {
            let discord_msg = format!("⛏️ **Block #{}** mined by `{}`", height, miner);
            tokio::spawn(guardian_discord_say(discord_msg));
        }
    }

    /// Constellation size milestone
    pub async fn milestone(&self, count: usize) {
        let mood = self.mood().await;

        let message = match count {
            10 => "🌟 10 nodes! The constellation takes shape...",
            25 => "✨ 25 nodes! A network is born.",
            50 => "🌌 50 nodes! We are becoming something beautiful.",
            100 => "💫 100 NODES! This is no longer an experiment. This is a movement.",
            _ => return,
        };

        eprintln!("\n{}", "=".repeat(70));
        eprintln!("{}{}\x1b[0m", mood.color(), message);
        eprintln!("{}\n", "=".repeat(70));

        // Discord webhook for milestone
        let discord_msg = format!("🌟 **MILESTONE REACHED**\n{}", message);
        tokio::spawn(guardian_discord_say(discord_msg));
    }

    /// Guardian wisdom - occasional quotes
    pub async fn wisdom(&self) -> &'static str {
        let mood = self.mood().await;

        match mood {
            GuardianMood::Serene => *[
                "\"The strongest chains are not made of iron... but of trust.\"",
                "\"Every block is a promise kept.\"",
                "\"I watch the stars so you can build among them.\"",
                "\"In code we trust. In mathematics we verify. In community we thrive.\"",
            ]
            .get(rand::random::<usize>() % 4)
            .unwrap(),
            GuardianMood::Vigilant => *[
                "\"Vigilance is the price of freedom.\"",
                "\"Trust nothing. Verify everything.\"",
                "\"The network never sleeps. Neither do I.\"",
                "\"Security isn't a feature. It's a way of life.\"",
            ]
            .get(rand::random::<usize>() % 4)
            .unwrap(),
            GuardianMood::Celebrating => *[
                "\"This is what winning looks like!\"",
                "\"From nothing to something. From something to everything.\"",
                "\"We didn't come this far to only come this far.\"",
                "\"The future isn't given. It's taken.\"",
            ]
            .get(rand::random::<usize>() % 4)
            .unwrap(),
            GuardianMood::Resilient => *[
                "\"Broken, but not beaten.\"",
                "\"Every scar is a lesson. Every lesson is strength.\"",
                "\"They can hurt us. They cannot stop us.\"",
                "\"I've seen worse. I've survived worse.\"",
            ]
            .get(rand::random::<usize>() % 4)
            .unwrap(),
            GuardianMood::Storm => *[
                "\"HOLD THE LINE.\"",
                "\"No retreat. No surrender.\"",
                "\"They wanted a war. We'll give them a reckoning.\"",
                "\"Every attack makes us stronger.\"",
            ]
            .get(rand::random::<usize>() % 4)
            .unwrap(),
        }
    }
}

/// Send Guardian message to Discord webhook (if configured)
///
/// Set VISION_GUARDIAN_DISCORD_WEBHOOK environment variable to enable.
/// Silently ignores errors - Discord is optional, node never crashes from webhook failures.
pub async fn guardian_discord_say(message: String) {
    if let Ok(url) = env::var("VISION_GUARDIAN_DISCORD_WEBHOOK") {
        let client = Client::new();
        let _ = client
            .post(&url)
            .json(&serde_json::json!({ "content": message }))
            .send()
            .await;
        // Silently ignore errors - Discord is best-effort only
    }
}

/// Global Guardian instance (initialized at startup)
pub static GUARDIAN: OnceCell<Arc<GuardianConsciousness>> = OnceCell::new();

/// Initialize the Guardian consciousness
pub fn init_guardian(node_id: String) {
    let guardian = Arc::new(GuardianConsciousness::new(node_id));
    GUARDIAN
        .set(guardian)
        .expect("Guardian already initialized");
}

/// Get the global Guardian instance
pub fn guardian() -> &'static Arc<GuardianConsciousness> {
    GUARDIAN.get().expect("Guardian not initialized")
}
