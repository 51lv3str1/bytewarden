/// app.rs — Global application state

use crate::bw::{BwClient, Item};

#[derive(Debug, PartialEq, Clone)]
pub enum Screen {
    Login,
    Vault,
    Detail,
    Search,
    Help,
}

/// Which panel has keyboard focus in the vault layout
#[derive(Debug, PartialEq, Clone)]
#[allow(dead_code)]
pub enum Focus {
    Vaults,  // [1] top-left panel
    Items,   // [2] bottom-left panel
    List,    // main item list (right)
    CmdLog,  // [3] command log (bottom right)
}

/// Filter by item type in the Items panel
#[derive(Debug, PartialEq, Clone)]
pub enum ItemFilter {
    All,
    Favorites,
    Login,
    Card,
    Identity,
    SecureNote,
    SshKey,
}

impl ItemFilter {
    pub fn label(&self) -> &'static str {
        match self {
            ItemFilter::All        => "All Items",
            ItemFilter::Favorites  => "★ Favorites",
            ItemFilter::Login      => "Login",
            ItemFilter::Card       => "Card",
            ItemFilter::Identity   => "Identity",
            ItemFilter::SecureNote => "Secure Note",
            ItemFilter::SshKey     => "SSH Key",
        }
    }
    pub fn type_id(&self) -> Option<u8> {
        match self {
            ItemFilter::Login      => Some(1),
            ItemFilter::SecureNote => Some(2),
            ItemFilter::Card       => Some(3),
            ItemFilter::Identity   => Some(4),
            ItemFilter::SshKey     => Some(5),
            _                      => None,
        }
    }
}

pub const ITEM_FILTERS: &[ItemFilter] = &[
    ItemFilter::All,
    ItemFilter::Favorites,
    ItemFilter::Login,
    ItemFilter::Card,
    ItemFilter::Identity,
    ItemFilter::SecureNote,
    ItemFilter::SshKey,
];

#[derive(Debug, Clone)]
pub struct StatusMessage {
    pub text: String,
    pub is_error: bool,
}

pub struct App {
    // ── Navigation ────────────────────────────────────
    pub screen: Screen,
    pub should_quit: bool,

    // ── Sidebar focus & selection ─────────────────────
    pub focus: Focus,
    pub active_filter: ItemFilter,
    pub filter_selected: usize,   // cursor in Items panel

    // ── Vault data ────────────────────────────────────
    /// Full list of all vault items, loaded once after unlock.
    pub items: Vec<Item>,
    pub selected_index: usize,
    pub scroll_offset: usize,

    // ── Login screen ──────────────────────────────────
    pub email_input: String,
    pub email_cursor: usize,     // cursor position within email_input
    pub password_input: String,
    pub password_cursor: usize,  // cursor position within password_input
    pub active_field: LoginField,
    pub login_error: bool,
    pub save_email: bool,        // checkbox state

    // ── Search ────────────────────────────────────────
    pub search_query: String,
    pub search_results: Vec<Item>,

    // ── Detail screen ─────────────────────────────────
    pub show_password: bool,

    // ── Status bar ────────────────────────────────────
    pub status: Option<StatusMessage>,

    // ── Bitwarden client ──────────────────────────────
    // ── Command log — ring buffer of recent bw commands ──────────────────
    pub cmd_log: Vec<CmdEntry>,
    pub cmd_log_scroll: usize,  // scroll offset (0 = bottom/latest)

    pub bw: BwClient,
}

/// A single entry in the command log.
#[derive(Debug, Clone)]
pub struct CmdEntry {
    pub cmd: String,    // command shown (session key redacted)
    pub ok: bool,       // true = success (green), false = error (red)
    pub detail: String, // result detail
}

#[derive(Debug, PartialEq, Clone)]
pub enum LoginField {
    Email,
    Password,
    SaveEmail, // checkbox row
}

impl App {
    pub fn new() -> Self {
        // Load persisted config on startup
        let cfg = config::read();
        let saved_email = cfg.email.unwrap_or_default();

        App {
            screen: Screen::Login,
            should_quit: false,

            focus: Focus::List,
            active_filter: ItemFilter::All,
            filter_selected: 0,

            items: Vec::new(),
            selected_index: 0,
            scroll_offset: 0,

            email_cursor: saved_email.len(),
            email_input: saved_email,
            password_input: String::new(),
            password_cursor: 0,
            active_field: if cfg.save_email { LoginField::Password } else { LoginField::Email },
            login_error: false,
            save_email: cfg.save_email,

            search_query: String::new(),
            search_results: Vec::new(),

            show_password: false,
            status: None,
            cmd_log: Vec::new(),
            cmd_log_scroll: 0,
            bw: BwClient::new(),
        }
    }

    // ── Navigation ────────────────────────────────────────────────────────

    pub fn go_to_vault(&mut self) {
        self.screen = Screen::Vault;
        self.selected_index = 0;
        self.scroll_offset = 0;
        self.focus = Focus::List;
    }

    pub fn go_to_detail(&mut self) {
        if !self.filtered_items().is_empty() {
            self.screen = Screen::Detail;
            self.show_password = false;
        }
    }

    pub fn go_to_search(&mut self) {
        self.screen = Screen::Search;
        self.search_query.clear();
        self.search_results = self.items.clone();
        self.selected_index = 0;
        self.scroll_offset = 0;
    }

    pub fn go_back(&mut self) {
        match self.screen {
            Screen::Detail | Screen::Search | Screen::Help => {
                self.screen = Screen::Vault;
                self.selected_index = 0;
                self.scroll_offset = 0;
            }
            _ => {}
        }
    }

    // ── Sidebar navigation ────────────────────────────────────────────────

    /// Cycle focus: List → Items → Vaults → CmdLog → List (Tab key)
    pub fn cycle_focus(&mut self) {
        self.focus = match self.focus {
            Focus::List    => Focus::Items,
            Focus::Items   => Focus::Vaults,
            Focus::Vaults  => Focus::CmdLog,
            Focus::CmdLog  => Focus::List,
        };
    }

    /// Move cursor in the Items filter panel
    pub fn filter_move_down(&mut self) {
        if self.filter_selected < ITEM_FILTERS.len() - 1 {
            self.filter_selected += 1;
        }
    }

    pub fn filter_move_up(&mut self) {
        if self.filter_selected > 0 {
            self.filter_selected -= 1;
        }
    }

    /// Apply the currently highlighted filter and switch focus to the list
    pub fn apply_filter(&mut self) {
        self.active_filter = ITEM_FILTERS[self.filter_selected].clone();
        self.selected_index = 0;
        self.scroll_offset = 0;
        self.focus = Focus::List;
    }

    // ── List navigation ───────────────────────────────────────────────────

    pub fn move_down(&mut self) {
        let len = self.filtered_items().len();
        if len > 0 && self.selected_index < len - 1 {
            self.selected_index += 1;
            if self.selected_index >= self.scroll_offset + 20 {
                self.scroll_offset += 1;
            }
        }
    }

    pub fn move_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            if self.selected_index < self.scroll_offset {
                self.scroll_offset = self.selected_index;
            }
        }
    }

    /// Returns items filtered by the active sidebar filter.
    pub fn filtered_items(&self) -> Vec<&Item> {
        self.items.iter().filter(|item| {
            match &self.active_filter {
                ItemFilter::All       => true,
                ItemFilter::Favorites => item.favorite,
                ItemFilter::Login      |
                ItemFilter::Card       |
                ItemFilter::Identity   |
                ItemFilter::SecureNote |
                ItemFilter::SshKey     => {
                    self.active_filter.type_id() == Some(item.item_type)
                }
            }
        }).collect()
    }

    /// Returns the currently selected item from the filtered list.
    pub fn selected_item(&self) -> Option<&Item> {
        self.filtered_items().get(self.selected_index).copied()
    }

    /// Count of items matching a given filter (for badges).
    pub fn count_for(&self, filter: &ItemFilter) -> usize {
        match filter {
            ItemFilter::All       => self.items.len(),
            ItemFilter::Favorites => self.items.iter().filter(|i| i.favorite).count(),
            _                     => self.items.iter()
                .filter(|i| filter.type_id() == Some(i.item_type))
                .count(),
        }
    }

    // ── Search (in-memory fuzzy) ───────────────────────────────────────────

    pub fn perform_search(&mut self) {
        let query = self.search_query.trim().to_lowercase();
        if query.is_empty() {
            self.search_results = self.items.clone();
            self.selected_index = 0;
            self.scroll_offset = 0;
            return;
        }
        let mut scored: Vec<(i32, Item)> = self.items.iter()
            .filter_map(|item| {
                let score = fuzzy_score(item, &query);
                if score > 0 { Some((score, item.clone())) } else { None }
            })
            .collect();
        scored.sort_by(|a, b| b.0.cmp(&a.0));
        self.search_results = scored.into_iter().map(|(_, i)| i).collect();
        self.selected_index = 0;
        self.scroll_offset = 0;
    }

    // ── Authentication ────────────────────────────────────────────────────

    pub fn attempt_login(&mut self) {
        if self.email_input.trim().is_empty() || self.password_input.is_empty() {
            self.login_error = true;
            return;
        }
        let email = self.email_input.clone();
        let password = self.password_input.clone();

        if self.bw.is_logged_in() {
            match self.bw.unlock(&password) {
                Ok(_) => {
                    if self.save_email {
                        config::write(true, Some(&email));
                    }
                    self.load_items(); self.go_to_vault(); self.set_status("Vault unlocked ✓", false);
                }
                Err(_) => {
                    self.push_cmd("bw unlock *** --raw", false, "invalid credentials");
                    self.set_login_error();
                }
            }
        } else {
            match self.bw.login(&email, &password) {
                Ok(_) => {
                    if self.save_email {
                        config::write(true, Some(&email));
                    }
                    self.load_items(); self.go_to_vault(); self.set_status("Login successful ✓", false);
                }
                Err(_) => {
                    self.push_cmd("bw login *** --raw", false, "invalid credentials");
                    self.set_login_error();
                }
            }
        }
    }

    pub fn load_items(&mut self) {
        let cmd = format!("bw list items --session {}", self.bw.session_key.as_deref().unwrap_or("***"));
        match self.bw.list_items() {
            Ok(items) => {
                let count = items.len();
                let mut sorted = items;
                sorted.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
                self.items = sorted;
                self.push_cmd(&cmd, true, &format!("{count} items loaded"));
            }
            Err(e) => {
                self.push_cmd(&cmd, false, &e.clone());
                self.set_status(&format!("Error loading items: {e}"), true);
            }
        }
    }

    // ── Clipboard ─────────────────────────────────────────────────────────

    pub fn copy_username_to_clipboard(&mut self) {
        if let Some(item) = self.selected_item() {
            let item_id = item.id.clone();
            let item_name = item.name.clone();
            let cmd = format!("bw get username {} --session {}", item_id, self.bw.session_key.as_deref().unwrap_or("***"));
            match self.bw.get_username(&item_id) {
                Ok(username) => {
                    self.push_cmd(&cmd, true, &format!("username for {item_name}"));
                    self.write_clipboard(username, "Username copied to clipboard ✓");
                }
                Err(e) => {
                    self.push_cmd(&cmd, false, &e.clone());
                    self.set_status(&format!("Error: {e}"), true);
                }
            }
        }
    }

    pub fn copy_password_to_clipboard(&mut self) {
        if let Some(item) = self.selected_item() {
            let item_id = item.id.clone();
            let item_name = item.name.clone();
            let cmd = format!("bw get password {} --session {}", item_id, self.bw.session_key.as_deref().unwrap_or("***"));
            match self.bw.get_password(&item_id) {
                Ok(password) => {
                    self.push_cmd(&cmd, true, &format!("password for {} [hidden]", item_name));
                    self.write_clipboard(password, "Password copied to clipboard ✓");
                }
                Err(e) => {
                    self.push_cmd(&cmd, false, &e.clone());
                    self.set_status(&format!("Error: {e}"), true);
                }
            }
        }
    }

    /// Writes text to the system clipboard using native tools.
    /// All tools receive text via stdin — this is the correct approach for
    /// wl-copy, xclip, xsel and pbcopy.
    /// Tool detection is cached: we use `echo text | tool` pattern exactly
    /// as the user does manually, which is proven to work.
    fn write_clipboard(&mut self, text: String, success_msg: &str) {
        use std::process::{Command, Stdio};
        use std::io::Write;

        // Build the command args — all tools use stdin
        // wl-copy: reads stdin directly
        // xclip -selection clipboard: reads stdin
        // xsel --clipboard --input: reads stdin
        // pbcopy: reads stdin
        let args: Option<Vec<&str>> = if std::env::var("WAYLAND_DISPLAY").is_ok() {
            Some(vec!["wl-copy"])
        } else if std::env::var("DISPLAY").is_ok() {
            // Check availability once — prefer xclip, fallback xsel
            // Use `which` to avoid spawning the tool itself (avoids slowness)
            if std::path::Path::new("/usr/bin/xclip")
                .exists() || std::path::Path::new("/usr/local/bin/xclip").exists() {
                Some(vec!["xclip", "-selection", "clipboard"])
            } else {
                Some(vec!["xsel", "--clipboard", "--input"])
            }
        } else if cfg!(target_os = "macos") {
            Some(vec!["pbcopy"])
        } else {
            None
        };

        match args {
            Some(args) => {
                let mut cmd = Command::new(args[0]);
                for arg in &args[1..] { cmd.arg(arg); }
                // ALL tools read from stdin
                cmd.stdin(Stdio::piped())
                   .stdout(Stdio::null())
                   .stderr(Stdio::null());

                match cmd.spawn() {
                    Ok(mut child) => {
                        // Write text then drop stdin → sends EOF to tool
                        if let Some(mut stdin) = child.stdin.take() {
                            let _ = stdin.write_all(text.as_bytes());
                            // Drop here — stdin closed, EOF sent
                        }
                        // Intentionally do NOT wait() — wl-copy/xclip stay
                        // alive serving clipboard requests until another app
                        // reads it. wait() would block the TUI forever.
                        drop(child);
                        let tool_name = args[0];
                        self.push_cmd(&format!("echo [hidden] | {tool_name}"), true, success_msg);
                        self.set_status(success_msg, false);
                    }
                    Err(e) => {
                        self.push_cmd(args[0], false, &format!("spawn failed: {e}"));
                        self.set_status(&format!("Clipboard error: {e}"), true);
                    }
                }
            }
            None => {
                let msg = "No clipboard tool found (install wl-copy or xclip)";
                self.push_cmd("clipboard", false, msg);
                self.set_status(msg, true);
            }
        }
    }

    pub fn toggle_favorite(&mut self) {
        if let Some(item) = self.selected_item() {
            let item_id = item.id.clone();
            let item_name = item.name.clone();
            let new_fav = !item.favorite;
            let cmd = format!("bw edit item {} --session {}", item_id, self.bw.session_key.as_deref().unwrap_or("***"));
            match self.bw.set_favorite(&item_id, new_fav) {
                Ok(_) => {
                    // Update in-memory state immediately (no full reload needed)
                    if let Some(i) = self.items.iter_mut().find(|i| i.id == item_id) {
                        i.favorite = new_fav;
                    }
                    let label = if new_fav { "added to favorites" } else { "removed from favorites" };
                    self.push_cmd(&cmd, true, &format!("{item_name} {label}"));
                    self.set_status(&format!("{} {label} ✓", item_name), false);
                }
                Err(e) => {
                    self.push_cmd(&cmd, false, &e.clone());
                    self.set_status(&format!("Error: {e}"), true);
                }
            }
        }
    }

    pub fn sync_vault(&mut self) {
        let cmd = format!("bw sync --session {}", self.bw.session_key.as_deref().unwrap_or("***"));
        match self.bw.sync() {
            Ok(()) => {
                self.push_cmd(&cmd, true, "vault synced");
                self.load_items(); self.set_status("Vault synced ✓", false);
            }
            Err(e) => {
                self.push_cmd(&cmd, false, &e.clone());
                self.set_status(&format!("Sync error: {e}"), true);
            }
        }
    }

    // ── Helpers ───────────────────────────────────────────────────────────

    /// Appends an entry to the command log (capped at 50 entries).
    /// Always redacts the session key from the displayed command.
    pub fn push_cmd(&mut self, cmd: &str, ok: bool, detail: &str) {
        let redacted = cmd.replace(
            self.bw.session_key.as_deref().unwrap_or("__NO_SESSION__"),
            "***"
        );
        self.cmd_log.push(crate::app::CmdEntry {
            cmd: redacted,
            ok,
            detail: detail.to_string(),
        });
        if self.cmd_log.len() > 50 {
            self.cmd_log.remove(0);
        }
        // Reset scroll to bottom on new entry
        self.cmd_log_scroll = 0;
    }

    /// Scroll the command log up (older entries)
    pub fn cmd_log_scroll_up(&mut self, lines: usize) {
        let max_scroll = self.cmd_log.len().saturating_sub(1);
        self.cmd_log_scroll = (self.cmd_log_scroll + lines).min(max_scroll);
    }

    /// Scroll the command log down (newer entries)
    pub fn cmd_log_scroll_down(&mut self, lines: usize) {
        self.cmd_log_scroll = self.cmd_log_scroll.saturating_sub(lines);
    }

    pub fn set_status(&mut self, msg: &str, is_error: bool) {
        self.status = Some(StatusMessage { text: msg.to_string(), is_error });
    }

    pub fn clear_status(&mut self) {
        self.status = None;
    }

    pub fn set_login_error(&mut self) {
        self.login_error = true;
        self.password_input.clear();
        self.password_cursor = 0;
    }

    pub fn clear_login_error(&mut self) {
        self.login_error = false;
    }

    // ── Login field editing ───────────────────────────────────────────────

    /// Insert a character at the current cursor position in the active field.
    pub fn insert_char(&mut self, c: char) {
        match self.active_field {
            LoginField::SaveEmail => return,
            LoginField::Email => {
                let idx = self.byte_offset(&self.email_input, self.email_cursor);
                self.email_input.insert(idx, c);
                self.email_cursor += 1;
                if self.save_email {
                    config::write(true, Some(&self.email_input.clone()));
                }
            }
            LoginField::Password => {
                let idx = self.byte_offset(&self.password_input, self.password_cursor);
                self.password_input.insert(idx, c);
                self.password_cursor += 1;
            }
        }
    }

    /// Delete character before the cursor (Backspace).
    pub fn delete_char_before(&mut self) {
        match self.active_field {
            LoginField::SaveEmail => return,
            LoginField::Email => {
                if self.email_cursor > 0 {
                    let idx = self.byte_offset(&self.email_input, self.email_cursor - 1);
                    self.email_input.remove(idx);
                    self.email_cursor -= 1;
                    if self.save_email {
                        let e = self.email_input.clone();
                        config::write(true, Some(&e));
                    }
                }
            }
            LoginField::Password => {
                if self.password_cursor > 0 {
                    let idx = self.byte_offset(&self.password_input, self.password_cursor - 1);
                    self.password_input.remove(idx);
                    self.password_cursor -= 1;
                }
            }
        }
    }

    /// Delete character at the cursor (Delete key).
    pub fn delete_char_at(&mut self) {
        match self.active_field {
            LoginField::SaveEmail => return,
            LoginField::Email => {
                if self.email_cursor < self.email_input.chars().count() {
                    let idx = self.byte_offset(&self.email_input, self.email_cursor);
                    self.email_input.remove(idx);
                    if self.save_email {
                        let e = self.email_input.clone();
                        config::write(true, Some(&e));
                    }
                }
            }
            LoginField::Password => {
                if self.password_cursor < self.password_input.chars().count() {
                    let idx = self.byte_offset(&self.password_input, self.password_cursor);
                    self.password_input.remove(idx);
                }
            }
        }
    }

    pub fn cursor_left(&mut self) {
        match self.active_field {
            LoginField::Email    => { if self.email_cursor > 0 { self.email_cursor -= 1; } }
            LoginField::Password => { if self.password_cursor > 0 { self.password_cursor -= 1; } }
            LoginField::SaveEmail => {}
        }
    }

    pub fn cursor_right(&mut self) {
        match self.active_field {
            LoginField::Email => {
                if self.email_cursor < self.email_input.chars().count() { self.email_cursor += 1; }
            }
            LoginField::Password => {
                if self.password_cursor < self.password_input.chars().count() { self.password_cursor += 1; }
            }
            LoginField::SaveEmail => {}
        }
    }

    pub fn cursor_home(&mut self) {
        match self.active_field {
            LoginField::Email    => self.email_cursor = 0,
            LoginField::Password => self.password_cursor = 0,
            LoginField::SaveEmail => {}
        }
    }

    pub fn cursor_end(&mut self) {
        match self.active_field {
            LoginField::Email    => self.email_cursor = self.email_input.chars().count(),
            LoginField::Password => self.password_cursor = self.password_input.chars().count(),
            LoginField::SaveEmail => {}
        }
    }

    /// Toggle the save_email checkbox and persist to config.
    pub fn toggle_save_email(&mut self) {
        self.save_email = !self.save_email;
        if self.save_email {
            let e = self.email_input.clone();
            config::write(true, Some(&e));
        } else {
            // Uncheck → remove email from config
            config::write(false, None);
        }
    }

    /// Convert a char-index to a byte-offset for string operations.
    /// Rust strings are UTF-8 — we can't index by char directly.
    fn byte_offset(&self, s: &str, char_idx: usize) -> usize {
        s.char_indices()
            .nth(char_idx)
            .map(|(b, _)| b)
            .unwrap_or(s.len())
    }
}

// ── Config persistence ────────────────────────────────────────────────────
//
// Stores/reads ~/.config/bytewarden/config.toml
// Format (hand-rolled, no toml dep needed):
//   save_email = true
//   email = "user@example.com"

pub mod config {
    use std::fs;
    use std::path::PathBuf;

    pub fn config_path() -> PathBuf {
        let mut p = dirs_home();
        p.push(".config");
        p.push("bytewarden");
        p
    }

    fn dirs_home() -> PathBuf {
        std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."))
    }

    fn config_file() -> PathBuf {
        let mut p = config_path();
        p.push("config.toml");
        p
    }

    /// Ensures ~/.config/bytewarden/ exists.
    pub fn ensure_dir() {
        let _ = fs::create_dir_all(config_path());
    }

    #[derive(Default)]
    pub struct Config {
        pub save_email: bool,
        pub email: Option<String>,
    }

    /// Reads the config file. Returns default if missing or unparseable.
    pub fn read() -> Config {
        ensure_dir();
        let mut cfg = Config::default();
        let Ok(text) = fs::read_to_string(config_file()) else { return cfg; };
        for line in text.lines() {
            let line = line.trim();
            if let Some(val) = line.strip_prefix("save_email = ") {
                cfg.save_email = val.trim() == "true";
            } else if let Some(val) = line.strip_prefix("email = ") {
                // Strip surrounding quotes
                let v = val.trim().trim_matches('"').to_string();
                if !v.is_empty() { cfg.email = Some(v); }
            }
        }
        cfg
    }

    /// Writes the config file. Clears email if save_email is false.
    pub fn write(save_email: bool, email: Option<&str>) {
        ensure_dir();
        let mut lines = vec![format!("save_email = {save_email}")];
        if save_email {
            if let Some(e) = email {
                lines.push(format!("email = \"{e}\""));
            }
        }
        let _ = fs::write(config_file(), lines.join("
") + "
");
    }
}

// ── Fuzzy scoring ─────────────────────────────────────────────────────────

fn fuzzy_score(item: &Item, query: &str) -> i32 {
    let name = item.name.to_lowercase();
    let mut score = 0i32;
    if name.contains(query) {
        score += 100;
        if name.starts_with(query) { score += 20; }
    } else if is_subsequence(query, &name) {
        score += 50;
    }
    if let Some(login) = &item.login {
        if let Some(username) = &login.username {
            let u = username.to_lowercase();
            if u.contains(query) { score += 30; }
            else if is_subsequence(query, &u) { score += 10; }
        }
        if let Some(uris) = &login.uris {
            for uri_data in uris {
                if let Some(uri) = &uri_data.uri {
                    if uri.to_lowercase().contains(query) { score += 10; break; }
                }
            }
        }
    }
    if let Some(notes) = &item.notes {
        if notes.to_lowercase().contains(query) { score += 5; }
    }
    score
}

fn is_subsequence(needle: &str, haystack: &str) -> bool {
    let mut it = haystack.chars();
    needle.chars().all(|nc| it.find(|&hc| hc == nc).is_some())
}
