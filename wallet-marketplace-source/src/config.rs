use anyhow::{Context, Result};
use once_cell::sync::OnceCell;
use serde::Deserialize;
use std::{fs, path::Path};

#[derive(Debug, Clone, Deserialize)]
pub struct Confirmations {
    pub btc: Option<u64>,
    pub bch: Option<u64>,
    pub doge: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ElectrumCfg {
    pub btc_host: Option<String>,
    pub btc_port_tls: Option<u16>,
    pub bch_host: Option<String>,
    pub bch_port_tls: Option<u16>,
    pub doge_host: Option<String>,
    pub doge_port_tls: Option<u16>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct AppConfig {
    pub confirmations: Option<Confirmations>,
    pub electrum: Option<ElectrumCfg>,
    pub test: Option<TestCfg>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct TestCfg {
    pub electrum_plaintext: Option<bool>,
    pub confirm_callback_url: Option<String>,
    pub mock_chain: Option<bool>,
    pub bech32_permissive: Option<bool>,
}

impl AppConfig {
    pub fn load_from(path: impl AsRef<Path>) -> Result<Self> {
        if path.as_ref().exists() {
            let s = fs::read_to_string(&path)
                .with_context(|| format!("reading {}", path.as_ref().display()))?;
            let cfg: AppConfig = toml::from_str(&s)
                .with_context(|| format!("parsing {}", path.as_ref().display()))?;
            Ok(cfg)
        } else {
            Ok(Default::default())
        }
    }

    fn resolve_u64(env_key: &str, toml_opt: Option<u64>, default_: u64) -> u64 {
        std::env::var(env_key)
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| toml_opt.unwrap_or(default_))
    }

    fn resolve_host_port(
        env_host: &str,
        env_port: &str,
        toml_host: Option<String>,
        toml_port: Option<u16>,
        def_host: &str,
        def_port: u16,
    ) -> (String, u16) {
        let host = std::env::var(env_host)
            .ok()
            .unwrap_or_else(|| toml_host.unwrap_or(def_host.to_string()));
        let port = std::env::var(env_port)
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| toml_port.unwrap_or(def_port));
        (host, port)
    }

    pub fn resolved(&self) -> ResolvedConfig {
        let btc_conf = Self::resolve_u64(
            "CONF_BTC",
            self.confirmations.as_ref().and_then(|c| c.btc),
            2,
        );
        let bch_conf = Self::resolve_u64(
            "CONF_BCH",
            self.confirmations.as_ref().and_then(|c| c.bch),
            2,
        );
        let doge_conf = Self::resolve_u64(
            "CONF_DOGE",
            self.confirmations.as_ref().and_then(|c| c.doge),
            40,
        );

        let (btc_host, btc_port) = {
            let e = self.electrum.as_ref();
            Self::resolve_host_port(
                "ELECTRUM_BTC_HOST",
                "ELECTRUM_BTC_PORT_TLS",
                e.and_then(|x| x.btc_host.clone()),
                e.and_then(|x| x.btc_port_tls),
                "electrum.blockstream.info",
                50002,
            )
        };
        let (bch_host, bch_port) = {
            let e = self.electrum.as_ref();
            Self::resolve_host_port(
                "ELECTRUM_BCH_HOST",
                "ELECTRUM_BCH_PORT_TLS",
                e.and_then(|x| x.bch_host.clone()),
                e.and_then(|x| x.bch_port_tls),
                "bch.imaginary.cash",
                50002,
            )
        };
        let (doge_host, doge_port) = {
            let e = self.electrum.as_ref();
            Self::resolve_host_port(
                "ELECTRUM_DOGE_HOST",
                "ELECTRUM_DOGE_PORT_TLS",
                e.and_then(|x| x.doge_host.clone()),
                e.and_then(|x| x.doge_port_tls),
                "electrum.dogecoin.org",
                50002,
            )
        };

        let plaintext = std::env::var("ELECTRUM_PLAINTEXT")
            .ok()
            .map(|s| s == "1" || s.eq_ignore_ascii_case("true"))
            .or_else(|| self.test.as_ref().and_then(|t| t.electrum_plaintext))
            .unwrap_or(false);

        let confirm_cb = std::env::var("MARKET_CONFIRM_URL")
            .ok()
            .or_else(|| {
                self.test
                    .as_ref()
                    .and_then(|t| t.confirm_callback_url.clone())
            })
            .unwrap_or_else(|| "http://127.0.0.1:8080/_market/land/confirm".into());

        let mock = std::env::var("MOCK_CHAIN")
            .ok()
            .map(|s| s == "1" || s.eq_ignore_ascii_case("true"))
            .unwrap_or_else(|| {
                self.test
                    .as_ref()
                    .and_then(|t| t.mock_chain)
                    .unwrap_or(false)
            });

        let bech32_perm = std::env::var("BECH32_PERMISSIVE")
            .ok()
            .map(|s| s == "1" || s.eq_ignore_ascii_case("true"))
            .unwrap_or_else(|| {
                self.test
                    .as_ref()
                    .and_then(|t| t.bech32_permissive)
                    .unwrap_or(false)
            });

        ResolvedConfig {
            btc_conf,
            bch_conf,
            doge_conf,
            btc_host,
            btc_port,
            bch_host,
            bch_port,
            doge_host,
            doge_port,
            electrum_plaintext: plaintext,
            confirm_callback_url: confirm_cb,
            mock_chain: mock,
            bech32_permissive: bech32_perm,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ResolvedConfig {
    pub btc_conf: u64,
    pub bch_conf: u64,
    pub doge_conf: u64,
    pub btc_host: String,
    pub btc_port: u16,
    pub bch_host: String,
    pub bch_port: u16,
    pub doge_host: String,
    pub doge_port: u16,
    pub electrum_plaintext: bool,
    pub confirm_callback_url: String,
    pub mock_chain: bool,
    pub bech32_permissive: bool,
}

static APP_CFG: OnceCell<ResolvedConfig> = OnceCell::new();

/// Install the resolved config for global access. Returns Err if already set.
pub fn set_app_cfg(cfg: ResolvedConfig) -> Result<(), anyhow::Error> {
    APP_CFG
        .set(cfg)
        .map_err(|_| anyhow::anyhow!("app config already set"))
}

/// Get the globally-installed resolved config. Panics if not initialized.
pub fn get_app_cfg() -> &'static ResolvedConfig {
    APP_CFG.get().expect("app config initialized")
}
