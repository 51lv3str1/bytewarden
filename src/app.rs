/// app.rs — Global application state
///
/// TypeScript/Redux analogy:
///   type Screen = "Login" | "Vault" | "Detail" | "Search" | "Help"
///   interface AppState { screen: Screen; items: Item[]; ... }
///
/// `enum Screen` uses Rust's algebraic enum — like a sealed class in Kotlin.
/// `struct App` is our single store — the whole app shares it via `&mut app`.

use crate::bw::{BwClient, Item};

/// All possible screens in the app.
/// `#[derive(PartialEq)]` enables `==` comparison (like `.equals()` in Java).
#[derive(Debug, PartialEq, Clone)]
pub enum Screen {
    Login,
    Vault,
    Detail,
    Search,
    Help,
}

/// User-facing feedback message shown in the status bar.
#[derive(Debug, Clone)]
pub struct StatusMessage {
    pub text: String,
    pub is_error: bool,
}

/// Central application state — equivalent to a Redux store or React Context.
pub struct App {
    // ── Navigation ────────────────────────────────────
    pub screen: Screen,
    pub should_quit: bool,

    // ── Vault data ────────────────────────────────────
    /// Full list of all vault items, loaded once after unlock.
    pub items: Vec<Item>,
    pub selected_index: usize,
    pub scroll_offset: usize,

    // ── Login screen ──────────────────────────────────
    pub email_input: String,
    pub password_input: String,
    pub active_field: LoginField,
    /// Shows a sanitized "invalid credentials" error inside the form.
    /// Never indicates which field is wrong (security best practice).
    pub login_error: bool,

    // ── Search ────────────────────────────────────────
    /// Current search query string typed by the user.
    pub search_query: String,
    /// Filtered results — computed in-memory from `items`, no bw calls.
    pub search_results: Vec<Item>,

    // ── Detail screen ─────────────────────────────────
    /// Whether to render the password in plaintext or masked.
    pub show_password: bool,

    // ── Status bar ────────────────────────────────────
    pub status: Option<StatusMessage>,

    // ── Bitwarden client ──────────────────────────────
    pub bw: BwClient,
}

/// Which login field currently has keyboard focus.
#[derive(Debug, PartialEq, Clone)]
pub enum LoginField {
    Email,
    Password,
}

impl App {
    pub fn new() -> Self {
        App {
            screen: Screen::Login,
            should_quit: false,

            items: Vec::new(),
            selected_index: 0,
            scroll_offset: 0,

            email_input: String::new(),
            password_input: String::new(),
            active_field: LoginField::Email,
            login_error: false,

            search_query: String::new(),
            search_results: Vec::new(),

            show_password: false,

            status: None,

            bw: BwClient::new(),
        }
    }

    // ── Navigation ────────────────────────────────────────────────────────

    pub fn go_to_vault(&mut self) {
        self.screen = Screen::Vault;
        self.selected_index = 0;
        self.scroll_offset = 0;
    }

    pub fn go_to_detail(&mut self) {
        if !self.current_list().is_empty() {
            self.screen = Screen::Detail;
            self.show_password = false;
        }
    }

    pub fn go_to_search(&mut self) {
        self.screen = Screen::Search;
        self.search_query.clear();
        self.search_results = self.items.clone(); // show all items initially
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

    // ── List navigation ───────────────────────────────────────────────────

    pub fn move_down(&mut self) {
        let len = self.current_list().len();
        if !len == 0 && self.selected_index < len - 1 {
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

    /// Returns whichever list is currently active (full vault or search results).
    pub fn current_list(&self) -> &Vec<Item> {
        if self.screen == Screen::Search {
            &self.search_results
        } else {
            &self.items
        }
    }

    /// Returns the currently highlighted item, if any.
    /// `Option<&Item>` is like `Item | undefined` in TypeScript.
    pub fn selected_item(&self) -> Option<&Item> {
        self.current_list().get(self.selected_index)
    }

    // ── Authentication ────────────────────────────────────────────────────

    pub fn attempt_login(&mut self) {
        // Guard: both fields must be filled before calling bw
        if self.email_input.trim().is_empty() || self.password_input.is_empty() {
            self.login_error = true;
            return;
        }

        let email = self.email_input.clone();
        let password = self.password_input.clone();

        if self.bw.is_logged_in() {
            match self.bw.unlock(&password) {
                Ok(_) => {
                    self.load_items();
                    self.go_to_vault();
                    self.set_status("Vault unlocked ✓", false);
                }
                Err(_) => self.set_login_error(),
            }
        } else {
            match self.bw.login(&email, &password) {
                Ok(_) => {
                    self.load_items();
                    self.go_to_vault();
                    self.set_status("Login successful ✓", false);
                }
                Err(_) => self.set_login_error(),
            }
        }
    }

    /// Loads all vault items into memory (one `bw list items` call).
    /// All subsequent searches filter this in-memory Vec — no extra bw calls.
    pub fn load_items(&mut self) {
        match self.bw.list_items() {
            Ok(items) => {
                let mut sorted = items;
                sorted.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
                self.items = sorted;
            }
            Err(e) => self.set_status(&format!("Error loading items: {e}"), true),
        }
    }

    // ── Fuzzy search (in-memory, instant) ────────────────────────────────
    //
    // Algorithm: for each item, compute a score against the query.
    // A score > 0 means the item matches. Higher score = better match.
    // This runs entirely on `self.items` — no subprocess, no I/O.
    //
    // Scoring rules (analogous to fzf's logic):
    //   +100  exact substring match in the name (case-insensitive)
    //   +50   all query chars appear in order in the name (subsequence match)
    //   +30   match found in username
    //   +10   match found in URL
    //
    // TypeScript equivalent:
    //   function fuzzyScore(item: Item, query: string): number { ... }

    pub fn perform_search(&mut self) {
        let query = self.search_query.trim().to_lowercase();

        if query.is_empty() {
            // Empty query = show everything
            self.search_results = self.items.clone();
            self.selected_index = 0;
            self.scroll_offset = 0;
            return;
        }

        let mut scored: Vec<(i32, Item)> = self
            .items
            .iter()
            .filter_map(|item| {
                let score = fuzzy_score(item, &query);
                if score > 0 { Some((score, item.clone())) } else { None }
            })
            .collect();

        // Sort descending by score (best match first)
        scored.sort_by(|a, b| b.0.cmp(&a.0));

        self.search_results = scored.into_iter().map(|(_, item)| item).collect();
        self.selected_index = 0;
        self.scroll_offset = 0;
    }

    // ── Clipboard ─────────────────────────────────────────────────────────

    pub fn copy_password_to_clipboard(&mut self) {
        if let Some(item) = self.selected_item() {
            let item_id = item.id.clone();
            match self.bw.get_password(&item_id) {
                Ok(password) => {
                    if let Ok(mut clipboard) = arboard::Clipboard::new() {
                        if clipboard.set_text(&password).is_ok() {
                            self.set_status("Password copied to clipboard ✓", false);
                        }
                    }
                }
                Err(e) => self.set_status(&format!("Error: {e}"), true),
            }
        }
    }

    // ── Sync ──────────────────────────────────────────────────────────────

    pub fn sync_vault(&mut self) {
        match self.bw.sync() {
            Ok(()) => {
                self.load_items();
                self.set_status("Vault synced ✓", false);
            }
            Err(e) => self.set_status(&format!("Sync error: {e}"), true),
        }
    }

    // ── Helpers ───────────────────────────────────────────────────────────

    pub fn set_status(&mut self, msg: &str, is_error: bool) {
        self.status = Some(StatusMessage {
            text: msg.to_string(),
            is_error,
        });
    }

    /// Shows a generic "invalid credentials" error — never reveals which field failed.
    pub fn set_login_error(&mut self) {
        self.login_error = true;
        self.password_input.clear(); // clear password so user retypes it
    }

    pub fn clear_login_error(&mut self) {
        self.login_error = false;
    }

    pub fn clear_status(&mut self) {
        self.status = None;
    }
}

// ── Fuzzy scoring function ────────────────────────────────────────────────
//
// Pure function: takes an Item and a lowercase query, returns a score.
// Score 0 = no match (item will be filtered out).

fn fuzzy_score(item: &Item, query: &str) -> i32 {
    let name = item.name.to_lowercase();
    let mut score = 0i32;

    // Rule 1: exact substring match in name → highest priority
    if name.contains(query) {
        score += 100;
        // Bonus: match at the start of the name
        if name.starts_with(query) {
            score += 20;
        }
    }

    // Rule 2: subsequence match in name
    // "gthb" matches "GitHub" because g→i→t→h→u→b contains g,t,h,b in order
    if score == 0 && is_subsequence(query, &name) {
        score += 50;
    }

    // Rule 3: match in username
    if let Some(login) = &item.login {
        if let Some(username) = &login.username {
            let uname = username.to_lowercase();
            if uname.contains(query) {
                score += 30;
            } else if is_subsequence(query, &uname) {
                score += 10;
            }
        }

        // Rule 4: match in URL
        if let Some(uris) = &login.uris {
            for uri_data in uris {
                if let Some(uri) = &uri_data.uri {
                    if uri.to_lowercase().contains(query) {
                        score += 10;
                        break;
                    }
                }
            }
        }
    }

    // Rule 5: match in notes
    if let Some(notes) = &item.notes {
        if notes.to_lowercase().contains(query) {
            score += 5;
        }
    }

    score
}

/// Returns true if every character of `needle` appears in `haystack` in order.
/// "gth" is a subsequence of "github" → true
/// This is O(n*m) but vaults are small (~hundreds of items) — negligible.
fn is_subsequence(needle: &str, haystack: &str) -> bool {
    let mut haystack_chars = haystack.chars();
    for nc in needle.chars() {
        // `find` advances the iterator — classic subsequence pointer technique
        if haystack_chars.find(|&hc| hc == nc).is_none() {
            return false;
        }
    }
    true
}