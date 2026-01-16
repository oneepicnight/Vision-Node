#![cfg(feature = "staging")]

// v1.0 Staged Module Stubs
// These provide minimal-but-valid implementations of gated features
// so that code can unconditionally import them without feature checks

pub mod airdrop_stub {
    pub mod cash {
        pub fn distribute_airdrop() {}
    }
}

pub mod foundation_config_stub {}

pub mod governance_stub {
    pub fn propose_change() {}
}

pub mod governance_democracy_stub {
    pub fn vote_on_proposal() {}
}

pub mod guardian_stub {
    pub mod consciousness {
        pub fn init() {}
    }
    pub mod integrity {
        pub fn check_integrity() -> bool {
            true
        }
    }
    pub mod events {
        pub struct Event;
    }

    pub fn is_creator_address(_addr: &str) -> bool {
        false
    }
}

pub mod land_deeds_stub {
    pub fn mint_deed() {}
}

pub mod land_stake_stub {
    pub fn stake_land() {}
}

pub mod legacy_stub {
    pub fn legacy_message(_data: &[u8]) -> String {
        String::new()
    }
}

pub mod node_approval_stub {
    pub fn check_approval(_node_id: &str) -> bool {
        true
    }
    pub fn is_approved(_node_id: &str) -> bool {
        true
    }
}

pub mod runtime_mode_stub {
    #[derive(Clone, Copy, Debug)]
    pub enum RuntimeMode {
        Normal,
        Maintenance,
        Guardian,
    }

    pub fn current_mode() -> RuntimeMode {
        RuntimeMode::Normal
    }

    pub fn is_maintenance() -> bool {
        false
    }
}

pub mod tip_stub {
    pub fn load_tip_state() -> Option<String> {
        None
    }
    pub fn save_tip_state(_data: &str) -> Result<(), String> {
        Ok(())
    }
}

pub mod ebid_stub {
    pub fn register_ebid() {}
}
