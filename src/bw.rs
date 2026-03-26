/// bw.rs — Wrapper around the `bw` Bitwarden CLI process

use serde::Deserialize;
use std::process::Command;

// ── Data models ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct Item {
    pub id:        String,
    pub name:      String,
    #[serde(rename = "type")]
    pub item_type: u8,
    pub login:     Option<LoginData>,
    pub card:      Option<CardData>,
    pub identity:  Option<IdentityData>,
    pub notes:     Option<String>,
    #[serde(rename = "folderId")]
    #[allow(dead_code)]
    pub folder_id: Option<String>,
    #[serde(default)]
    pub favorite:  bool,
    #[serde(default)]
    pub fields:    Vec<Field>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoginData {
    pub username: Option<String>,
    pub password: Option<String>,
    pub uris:     Option<Vec<UriData>>,
    pub totp:     Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UriData {
    pub uri: Option<String>,
}

/// Custom field — type: 0=text, 1=hidden, 2=boolean, 3=linked
#[derive(Debug, Clone, Deserialize)]
pub struct Field {
    pub name:  Option<String>,
    pub value: Option<String>,
    #[serde(rename = "type")]
    pub field_type: u8,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CardData {
    #[serde(rename = "cardholderName")]
    pub cardholder_name: Option<String>,
    pub brand:     Option<String>,
    pub number:    Option<String>,
    #[serde(rename = "expMonth")]
    pub exp_month: Option<String>,
    #[serde(rename = "expYear")]
    pub exp_year:  Option<String>,
    pub code:      Option<String>, // CVV
}

#[derive(Debug, Clone, Deserialize)]
pub struct IdentityData {
    pub title:      Option<String>,
    #[serde(rename = "firstName")]  pub first_name:  Option<String>,
    #[serde(rename = "middleName")] pub middle_name: Option<String>,
    #[serde(rename = "lastName")]   pub last_name:   Option<String>,
    pub email:      Option<String>,
    pub phone:      Option<String>,
    pub company:    Option<String>,
    #[serde(rename = "ssn")]            pub ssn:      Option<String>,
    #[serde(rename = "passportNumber")] pub passport: Option<String>,
    #[serde(rename = "licenseNumber")]  pub license:  Option<String>,
    pub address1:    Option<String>,
    pub address2:    Option<String>,
    pub city:        Option<String>,
    pub state:       Option<String>,
    #[serde(rename = "postalCode")]
    pub postal_code: Option<String>,
    pub country:     Option<String>,
}

// ── VaultStatus / BwStatusInfo ────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum VaultStatus { Unlocked, Locked, Unauthenticated }

#[derive(Debug, Clone)]
pub struct BwStatusInfo {
    pub status:     VaultStatus,
    pub user_email: Option<String>,
    #[allow(dead_code)] pub last_sync:  Option<String>,  // P7: show in status pane
    #[allow(dead_code)] pub server_url: Option<String>,  // P7: show in status pane
}

// ── BwClient ──────────────────────────────────────────────────────────────

pub struct BwClient {
    pub session_key: Option<String>,
}

impl BwClient {
    pub fn new() -> Self { BwClient { session_key: None } }

    // ── Auth ──────────────────────────────────────────────────────────────

    pub fn status(&self) -> Result<BwStatusInfo, String> {
        // Use a timeout — bw status should be fast (local only), but Node.js
        // startup can be slow. If it takes >4 s we fall through to normal login.
        let out = bw_run_timeout(&["status", "--nointeraction"], 4)?;
        let val: serde_json::Value = serde_json::from_str(&stdout_str(&out))
            .map_err(|e| format!("bw status JSON parse error: {e}"))?;

        let status = match val["status"].as_str().unwrap_or("unauthenticated") {
            "unlocked" => VaultStatus::Unlocked,
            "locked"   => VaultStatus::Locked,
            _          => VaultStatus::Unauthenticated,
        };
        Ok(BwStatusInfo {
            status,
            user_email: opt_str(&val, "userEmail"),
            last_sync:  opt_str(&val, "lastSync"),
            server_url: opt_str(&val, "serverUrl"),
        })
    }

    pub fn login(&mut self, email: &str, password: &str) -> Result<String, String> {
        let out = bw_run(&["login", email, password, "--raw"])?;
        if out.status.success() {
            let key = stdout_str(&out);
            self.session_key = Some(key.clone());
            Ok(key)
        } else {
            Err(stderr_str(&out))
        }
    }

    pub fn unlock(&mut self, password: &str) -> Result<String, String> {
        let out = bw_run(&["unlock", password, "--raw"])?;
        if out.status.success() {
            let key = stdout_str(&out);
            self.session_key = Some(key.clone());
            Ok(key)
        } else {
            Err(stderr_str(&out))
        }
    }

    pub fn lock(&mut self) {
        let _ = bw_run(&["lock"]);
        self.session_key = None;
    }

    // ── Vault operations ──────────────────────────────────────────────────

    pub fn list_items(&self) -> Result<Vec<Item>, String> {
        let session = self.session()?;
        let out     = bw_run(&["list", "items", "--session", session])?;
        if out.status.success() {
            serde_json::from_str::<Vec<Item>>(&stdout_str(&out))
                .map_err(|e| format!("Error parsing items JSON: {e}"))
        } else {
            Err(stderr_str(&out))
        }
    }

    /// Fetches only trashed items (`bw list items --trash`).
    pub fn list_trash(&self) -> Result<Vec<Item>, String> {
        let session = self.session()?;
        let out     = bw_run(&["list", "items", "--trash", "--session", session])?;
        if out.status.success() {
            serde_json::from_str::<Vec<Item>>(&stdout_str(&out))
                .map_err(|e| format!("Error parsing trash JSON: {e}"))
        } else {
            Err(stderr_str(&out))
        }
    }

    /// Restores a trashed item back to the vault.
    pub fn restore_item(&self, item_id: &str) -> Result<(), String> {
        let session = self.session()?;
        let out     = bw_run(&["restore", "item", item_id, "--session", session])?;
        if out.status.success() { Ok(()) } else { Err(stderr_str(&out)) }
    }

    pub fn sync(&self) -> Result<(), String> {
        let session = self.session()?;
        let out     = bw_run(&["sync", "--session", session])?;
        if out.status.success() { Ok(()) } else { Err(stderr_str(&out)) }
    }

    // ── Single-field getters ──────────────────────────────────────────────

    pub fn get_username(&self, item_id: &str) -> Result<String, String> {
        self.bw_get_field("username", item_id)
    }
    pub fn get_password(&self, item_id: &str) -> Result<String, String> {
        self.bw_get_field("password", item_id)
    }
    pub fn get_totp(&self, item_id: &str) -> Result<String, String> {
        self.bw_get_field("totp", item_id)
    }

    /// `bw get <field> <item_id> --session` — shared by username/password/totp.
    fn bw_get_field(&self, field: &str, item_id: &str) -> Result<String, String> {
        let session = self.session()?;
        let out     = bw_run(&["get", field, item_id, "--session", session])?;
        if out.status.success() { Ok(stdout_str(&out)) } else { Err(stderr_str(&out)) }
    }

    // ── Item CRUD ─────────────────────────────────────────────────────────

    /// Returns the raw JSON string for an item — base for edit patching.
    pub fn get_item_json(&self, item_id: &str) -> Result<String, String> {
        let session = self.session()?;
        let out     = bw_run(&["get", "item", item_id, "--session", session])?;
        if out.status.success() {
            Ok(String::from_utf8_lossy(&out.stdout).to_string())
        } else {
            Err(stderr_str(&out))
        }
    }

    /// Creates a new vault item from a JSON string (base64-encoded internally).
    pub fn create_item(&self, item_json: &str) -> Result<Item, String> {
        let session = self.session()?;
        let encoded = base64_encode(item_json);
        let out     = bw_run(&["create", "item", &encoded, "--session", session])?;
        if out.status.success() {
            serde_json::from_str::<Item>(&stdout_str(&out))
                .map_err(|e| format!("Error parsing created item: {e}"))
        } else {
            Err(stderr_str(&out))
        }
    }

    /// Replaces a vault item with a mutated JSON object (base64-encoded internally).
    pub fn edit_item(&self, item_id: &str, item_json: &str) -> Result<Item, String> {
        let session = self.session()?;
        let encoded = base64_encode(item_json);
        let out     = bw_run(&["edit", "item", item_id, &encoded, "--session", session])?;
        if out.status.success() {
            serde_json::from_str::<Item>(&stdout_str(&out))
                .map_err(|e| format!("Error parsing edited item: {e}"))
        } else {
            Err(stderr_str(&out))
        }
    }

    /// Deletes a vault item. `permanent = false` moves it to trash.
    pub fn delete_item(&self, item_id: &str, permanent: bool) -> Result<(), String> {
        let session = self.session()?;
        let mut args = vec!["delete", "item", item_id, "--session", session];
        if permanent { args.push("--permanent"); }
        let out = bw_run(&args)?;
        if out.status.success() { Ok(()) } else { Err(stderr_str(&out)) }
    }

    /// Toggles the favorite flag: get item JSON → flip field → base64 → edit.
    pub fn set_favorite(&self, item_id: &str, favorite: bool) -> Result<String, String> {
        let json     = self.get_item_json(item_id)?;
        let mut val: serde_json::Value = serde_json::from_str(&json)
            .map_err(|e| format!("JSON parse error: {e}"))?;
        val["favorite"] = serde_json::Value::Bool(favorite);
        let new_json = serde_json::to_string(&val)
            .map_err(|e| format!("JSON serialize error: {e}"))?;
        let item = self.edit_item(item_id, &new_json)?;
        Ok(item.name)
    }

    // ── Helpers ───────────────────────────────────────────────────────────

    fn session(&self) -> Result<&str, String> {
        self.session_key.as_deref().ok_or_else(|| "Vault is locked".to_string())
    }
}

// ── Free functions ────────────────────────────────────────────────────────

pub fn item_type_label(t: u8) -> &'static str {
    match t {
        1 => "Login", 2 => "Secure Note", 3 => "Card",
        4 => "Identity", 5 => "SSH Key", _ => "Other",
    }
}

// ── Private helpers ───────────────────────────────────────────────────────

/// Run `bw <args>` and return the raw Output, mapping spawn errors to String.
fn bw_run(args: &[&str]) -> Result<std::process::Output, String> {
    Command::new("bw")
        .args(args)
        .output()
        .map_err(|e| format!("Could not run bw: {e}"))
}

/// Run `bw <args>` with a wall-clock timeout (seconds).
/// If the process doesn't finish in time it is killed and an error is returned.
fn bw_run_timeout(args: &[&str], secs: u64) -> Result<std::process::Output, String> {
    use std::time::{Duration, Instant};
    use std::io::Read;

    let mut child = Command::new("bw")
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Could not run bw: {e}"))?;

    let deadline = Instant::now() + Duration::from_secs(secs);
    loop {
        match child.try_wait().map_err(|e| format!("bw wait error: {e}"))? {
            Some(status) => {
                let mut stdout = Vec::new();
                let mut stderr = Vec::new();
                if let Some(mut o) = child.stdout.take() { let _ = o.read_to_end(&mut stdout); }
                if let Some(mut e) = child.stderr.take() { let _ = e.read_to_end(&mut stderr); }
                return Ok(std::process::Output { status, stdout, stderr });
            }
            None => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    return Err(format!("bw status timed out after {secs}s"));
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    }
}

/// Trimmed stdout as String.
fn stdout_str(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// Trimmed stderr as String.
fn stderr_str(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stderr).trim().to_string()
}

/// Extract a non-empty string field from a JSON Value, returning None if absent or empty.
fn opt_str(val: &serde_json::Value, key: &str) -> Option<String> {
    val[key].as_str().filter(|s| !s.is_empty()).map(|s| s.to_string())
}

/// Standard base64 encoding — equivalent to piping through `bw encode`.
fn base64_encode(input: &str) -> String {
    const C: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes = input.as_bytes();
    let mut out = String::with_capacity((bytes.len() + 2) / 3 * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let n  = (b0 << 16) | (b1 << 8) | b2;
        out.push(C[((n >> 18) & 63) as usize] as char);
        out.push(C[((n >> 12) & 63) as usize] as char);
        out.push(if chunk.len() > 1 { C[((n >>  6) & 63) as usize] as char } else { '=' });
        out.push(if chunk.len() > 2 { C[( n        & 63) as usize] as char } else { '=' });
    }
    out
}
