/// bw.rs — Wrapper around the `bw` Bitwarden CLI process
///
/// TypeScript analogy:
///   export class BwClient {
///     sessionKey: string | null = null;
///     async login(email, password): Promise<string> { ... }
///     async listItems(): Promise<Item[]> { ... }
///   }
///
/// We use std::process::Command to spawn `bw` as a child process,
/// capture its stdout as a String, and parse JSON with serde_json —
/// equivalent to child_process.execSync() + JSON.parse() in Node.js.

use serde::Deserialize;
use std::process::Command;

// ── Data models ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct Item {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub item_type: u8,
    pub login: Option<LoginData>,
    pub notes: Option<String>,
    #[serde(rename = "folderId")]
    pub folder_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoginData {
    pub username: Option<String>,
    pub password: Option<String>,
    pub uris: Option<Vec<UriData>>,
    pub totp: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UriData {
    pub uri: Option<String>,
}

// ── BwClient ───────────────────────────────────────────────────────────────

pub struct BwClient {
    pub session_key: Option<String>,
}

impl BwClient {
    pub fn new() -> Self {
        BwClient { session_key: None }
    }

    pub fn is_logged_in(&self) -> bool {
        Command::new("bw")
            .args(["login", "--check"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    pub fn login(&mut self, email: &str, password: &str) -> Result<String, String> {
        let output = Command::new("bw")
            .args(["login", email, password, "--raw"])
            .output()
            .map_err(|e| format!("Could not run bw: {e}"))?;

        if output.status.success() {
            let key = String::from_utf8_lossy(&output.stdout).trim().to_string();
            self.session_key = Some(key.clone());
            Ok(key)
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    }

    pub fn unlock(&mut self, password: &str) -> Result<String, String> {
        let output = Command::new("bw")
            .args(["unlock", password, "--raw"])
            .output()
            .map_err(|e| format!("Could not run bw: {e}"))?;

        if output.status.success() {
            let key = String::from_utf8_lossy(&output.stdout).trim().to_string();
            self.session_key = Some(key.clone());
            Ok(key)
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    }

    /// Fetches ALL vault items in one shot.
    /// We load everything once into memory — search is then instant (no extra bw calls).
    pub fn list_items(&self) -> Result<Vec<Item>, String> {
        let session = self.session_key.as_deref().ok_or("Vault is locked")?;

        let output = Command::new("bw")
            .args(["list", "items", "--session", session])
            .output()
            .map_err(|e| format!("Error running bw: {e}"))?;

        if output.status.success() {
            let json = String::from_utf8_lossy(&output.stdout);
            serde_json::from_str::<Vec<Item>>(&json)
                .map_err(|e| format!("Error parsing JSON: {e}"))
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    }

    pub fn get_password(&self, item_id: &str) -> Result<String, String> {
        let session = self.session_key.as_deref().ok_or("Vault is locked")?;

        let output = Command::new("bw")
            .args(["get", "password", item_id, "--session", session])
            .output()
            .map_err(|e| format!("Error running bw: {e}"))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    }

    pub fn get_totp(&self, item_id: &str) -> Result<String, String> {
        let session = self.session_key.as_deref().ok_or("Vault is locked")?;

        let output = Command::new("bw")
            .args(["get", "totp", item_id, "--session", session])
            .output()
            .map_err(|e| format!("Error running bw: {e}"))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    }

    pub fn sync(&self) -> Result<(), String> {
        let session = self.session_key.as_deref().ok_or("Vault is locked")?;

        let output = Command::new("bw")
            .args(["sync", "--session", session])
            .output()
            .map_err(|e| format!("Error running bw: {e}"))?;

        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    }

    pub fn generate_password(length: u32, special: bool) -> Result<String, String> {
        let mut args = vec![
            "generate".to_string(),
            "-uln".to_string(),
            "--length".to_string(),
            length.to_string(),
        ];
        if special {
            args.push("-s".to_string());
        }

        let output = Command::new("bw")
            .args(&args)
            .output()
            .map_err(|e| format!("Error running bw: {e}"))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    }

    pub fn lock(&mut self) {
        let _ = Command::new("bw").arg("lock").output();
        self.session_key = None;
    }
}

pub fn item_type_label(t: u8) -> &'static str {
    match t {
        1 => "Login",
        2 => "Note",
        3 => "Card",
        4 => "Identity",
        _ => "Other",
    }
}