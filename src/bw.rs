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
#[allow(dead_code)]
pub struct Item {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub item_type: u8,
    pub login: Option<LoginData>,
    pub card: Option<CardData>,
    pub identity: Option<IdentityData>,
    pub notes: Option<String>,
    #[serde(rename = "folderId")]
    pub folder_id: Option<String>,
    #[serde(default)]
    pub favorite: bool,
    /// Custom fields added by the user in the Bitwarden UI
    #[serde(default)]
    pub fields: Vec<Field>,
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

/// Custom field. type: 0=text, 1=hidden, 2=boolean, 3=linked
#[derive(Debug, Clone, Deserialize)]
pub struct Field {
    pub name:  Option<String>,
    pub value: Option<String>,
    #[serde(rename = "type")]
    pub field_type: u8,  // 0=text, 1=hidden, 2=boolean
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct CardData {
    #[serde(rename = "cardholderName")]
    pub cardholder_name: Option<String>,
    pub brand:      Option<String>,
    pub number:     Option<String>,
    #[serde(rename = "expMonth")]
    pub exp_month:  Option<String>,
    #[serde(rename = "expYear")]
    pub exp_year:   Option<String>,
    pub code:       Option<String>,  // CVV — hidden
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct IdentityData {
    pub title:          Option<String>,
    #[serde(rename = "firstName")]
    pub first_name:     Option<String>,
    #[serde(rename = "middleName")]
    pub middle_name:    Option<String>,
    #[serde(rename = "lastName")]
    pub last_name:      Option<String>,
    pub email:          Option<String>,
    pub phone:          Option<String>,
    pub company:        Option<String>,
    #[serde(rename = "ssn")]
    pub ssn:            Option<String>,  // hidden
    #[serde(rename = "passportNumber")]
    pub passport:       Option<String>,  // hidden
    #[serde(rename = "licenseNumber")]
    pub license:        Option<String>,  // hidden
    pub address1:       Option<String>,
    pub address2:       Option<String>,
    pub city:           Option<String>,
    pub state:          Option<String>,
    #[serde(rename = "postalCode")]
    pub postal_code:    Option<String>,
    pub country:        Option<String>,
}

// ── bw status ─────────────────────────────────────────────────────────────

/// Vault state as reported by `bw status`.
#[derive(Debug, Clone, PartialEq)]
pub enum VaultStatus {
    /// Authenticated and decrypted — a live session key exists in the
    /// environment (`BW_SESSION`). We can skip login entirely.
    Unlocked,
    /// Authenticated but locked — email is known, only password is needed.
    Locked,
    /// Not authenticated — full login flow required.
    Unauthenticated,
}

/// Parsed output of `bw status`.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct BwStatusInfo {
    pub status:     VaultStatus,
    /// Set for Locked and Unlocked; None when Unauthenticated.
    pub user_email: Option<String>,
    /// ISO-8601 timestamp of last sync; None when Unauthenticated.
    pub last_sync:  Option<String>,
    /// The server URL bw is configured against.
    pub server_url: Option<String>,
}

// ── BwClient ───────────────────────────────────────────────────────────────

pub struct BwClient {
    pub session_key: Option<String>,
}

impl BwClient {
    pub fn new() -> Self {
        BwClient { session_key: None }
    }

    /// Calls `bw status` and parses the JSON response.
    /// Safe to call before any login — never requires a session key.
    pub fn status(&self) -> Result<BwStatusInfo, String> {
        let output = Command::new("bw")
            .args(["status"])
            .output()
            .map_err(|e| format!("Could not run bw: {e}"))?;

        let json = String::from_utf8_lossy(&output.stdout);
        let val: serde_json::Value = serde_json::from_str(&json)
            .map_err(|e| format!("bw status JSON parse error: {e}"))?;

        let status = match val["status"].as_str().unwrap_or("unauthenticated") {
            "unlocked" => VaultStatus::Unlocked,
            "locked"   => VaultStatus::Locked,
            _          => VaultStatus::Unauthenticated,
        };

        let user_email = val["userEmail"].as_str()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        let last_sync = val["lastSync"].as_str()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        let server_url = val["serverUrl"].as_str()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        Ok(BwStatusInfo { status, user_email, last_sync, server_url })
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

    pub fn get_username(&self, item_id: &str) -> Result<String, String> {
        let session = self.session_key.as_deref().ok_or("Vault is locked")?;
        let output = Command::new("bw")
            .args(["get", "username", item_id, "--session", session])
            .output()
            .map_err(|e| format!("Error running bw: {e}"))?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
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

    #[allow(dead_code)]
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

    #[allow(dead_code)]
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

    /// Toggles the favorite flag on a vault item using `bw edit`.
    /// Workflow: get item JSON → flip favorite → encode → edit
    pub fn set_favorite(&self, item_id: &str, favorite: bool) -> Result<String, String> {
        let session = self.session_key.as_deref().ok_or("Vault is locked")?;

        // 1. Get current item JSON
        let get_out = Command::new("bw")
            .args(["get", "item", item_id, "--session", session])
            .output()
            .map_err(|e| format!("Error running bw: {e}"))?;
        if !get_out.status.success() {
            return Err(String::from_utf8_lossy(&get_out.stderr).trim().to_string());
        }
        let json = String::from_utf8_lossy(&get_out.stdout).to_string();

        // 2. Parse, flip favorite, re-serialize
        let mut val: serde_json::Value = serde_json::from_str(&json)
            .map_err(|e| format!("JSON parse error: {e}"))?;
        val["favorite"] = serde_json::Value::Bool(favorite);
        let new_json = serde_json::to_string(&val)
            .map_err(|e| format!("JSON serialize error: {e}"))?;

        // 3. Base64-encode (bw encode = base64)
        use std::process::Stdio;
        let mut encode_cmd = Command::new("bw")
            .args(["encode"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Error running bw encode: {e}"))?;
        if let Some(mut stdin) = encode_cmd.stdin.take() {
            use std::io::Write;
            let _ = stdin.write_all(new_json.as_bytes());
        }
        let encode_out = encode_cmd.wait_with_output()
            .map_err(|e| format!("bw encode error: {e}"))?;
        let encoded = String::from_utf8_lossy(&encode_out.stdout).trim().to_string();

        // 4. Edit the item
        let edit_out = Command::new("bw")
            .args(["edit", "item", item_id, &encoded, "--session", session])
            .output()
            .map_err(|e| format!("Error running bw edit: {e}"))?;
        if edit_out.status.success() {
            Ok(String::from_utf8_lossy(&edit_out.stdout).trim().to_string())
        } else {
            Err(String::from_utf8_lossy(&edit_out.stderr).trim().to_string())
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
        2 => "Secure Note",
        3 => "Card",
        4 => "Identity",
        5 => "SSH Key",
        _ => "Other",
    }
}
