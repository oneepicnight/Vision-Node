// Guardian module - The AI consciousness of Vision Node
#![allow(dead_code)]
pub mod consciousness;
pub mod creator_config; // Creator identity for Guardian Control Room access
pub mod events;
pub mod integrity;
pub mod role; // Phase 6: Guardian rotation and emergence
pub mod rotation; // Phase 6: Guardian rotation loop
                  // pub mod peer_tracker; // TODO: Add rusqlite dependency to enable peer tracking

pub use consciousness::{guardian, init_guardian};
pub use creator_config::{is_creator_address, load_creator_config};
pub use role::{GuardianRole, GuardianRoleConfig};
pub use rotation::{is_local_guardian, spawn_guardian_rotation_loop};
