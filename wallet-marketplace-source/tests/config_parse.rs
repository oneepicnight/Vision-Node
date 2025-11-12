use tempfile::TempDir;
use vision_market::config::AppConfig;

#[test]
fn parses_toml_and_resolves_with_env_overrides() {
    let toml = r#"
        [confirmations]
        btc = 3
        [electrum]
        btc_host = "example.com"
        btc_port_tls = 50099
    "#;

    let dir = TempDir::new().unwrap();
    let path = dir.path().join("vision.toml");
    std::fs::write(&path, toml).unwrap();

    let app = AppConfig::load_from(&path).unwrap().resolved();
    assert_eq!(app.btc_conf, 3);
    assert_eq!(app.btc_host, "example.com");
    assert_eq!(app.btc_port, 50099);

    std::env::set_var("CONF_BTC", "5");
    std::env::set_var("ELECTRUM_BTC_HOST", "env-host");
    std::env::set_var("ELECTRUM_BTC_PORT_TLS", "50123");
    let app2 = AppConfig::load_from(&path).unwrap().resolved();
    assert_eq!(app2.btc_conf, 5);
    assert_eq!(app2.btc_host, "env-host");
    assert_eq!(app2.btc_port, 50123);
}
