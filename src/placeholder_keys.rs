use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default)]
pub struct PlaceholderStatus {
    pub is_placeholder: bool,
    pub failed_files: Vec<PlaceholderFailure>,
}

#[derive(Debug, Clone)]
pub struct PlaceholderFailure {
    pub path: PathBuf,
    pub reason: String,
}

impl std::fmt::Display for PlaceholderFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.path.display(), self.reason)
    }
}

pub fn check_placeholder_keys() -> PlaceholderStatus {
    let files = ["keys.json", "keys-recipient.json"];
    let mut failed = Vec::new();

    for name in files.iter() {
        let path = Path::new(name).to_path_buf();
        match fs::read_to_string(&path) {
            Ok(contents) => {
                if is_placeholder_json(&contents) {
                    failed.push(PlaceholderFailure {
                        path,
                        reason: "placeholder or empty key fields".to_string(),
                    });
                }
            }
            Err(_) => {
                failed.push(PlaceholderFailure {
                    path,
                    reason: "file missing".to_string(),
                });
            }
        }
    }

    PlaceholderStatus {
        is_placeholder: !failed.is_empty(),
        failed_files: failed,
    }
}

fn is_placeholder_json(contents: &str) -> bool {
    let trimmed = contents.trim();
    if trimmed.is_empty() {
        return true;
    }

    let markers = ["REDACTED", "CHANGE_ME", "PLACEHOLDER", "REPLACE_WITH", ""].as_slice();
    for marker in markers {
        if trimmed.contains(marker) {
            return true;
        }
    }

    if let Ok(json) = serde_json::from_str::<serde_json::Value>(trimmed) {
        let keys = ["public_key", "secret_key"];
        for k in keys.iter() {
            if let Some(v) = json.get(*k).and_then(|v| v.as_str()) {
                if v.trim().is_empty() || v.len() < 64 {
                    return true;
                }
                let upper = v.to_ascii_uppercase();
                if upper.contains("REPLACE") || upper.contains("PLACEHOLDER") || upper.contains("CHANGE") || upper.contains("REDACTED") {
                    return true;
                }
            } else {
                return true;
            }
        }
        false
    } else {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_temp(contents: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let fname = format!("placeholder-test-{}.json", nonce);
        path.push(fname);
        let mut f = fs::File::create(&path).expect("create temp file");
        f.write_all(contents.as_bytes()).expect("write temp file");
        path
    }

    #[test]
    fn detects_placeholder_strings() {
        let path = write_temp("{\"public_key\":\"REPLACE_WITH_PUBLIC_KEY_HEX\",\"secret_key\":\"REPLACE_WITH_SECRET_KEY_HEX\"}");
        let data = fs::read_to_string(&path).unwrap();
        assert!(is_placeholder_json(&data));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn passes_valid_keys() {
        let path = write_temp("{\"public_key\":\"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\",\"secret_key\":\"abcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcd\"}");
        let data = fs::read_to_string(&path).unwrap();
        assert!(!is_placeholder_json(&data));
        let _ = fs::remove_file(path);
    }
}
