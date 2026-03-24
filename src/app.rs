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
pub enum Focus {
    Vaults,  // [1] top-left panel
    Items,   // [2] bottom-left panel
    List,    // main item list (right)
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
    pub password_input: String,
    pub active_field: LoginField,
    pub login_error: bool,

    // ── Search ────────────────────────────────────────
    pub search_query: String,
    pub search_results: Vec<Item>,

    // ── Detail screen ─────────────────────────────────
    pub show_password: bool,

    // ── Status bar ────────────────────────────────────
    pub status: Option<StatusMessage>,

    // ── Bitwarden client ──────────────────────────────
    pub bw: BwClient,
}

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

            focus: Focus::List,
            active_filter: ItemFilter::All,
            filter_selected: 0,

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

    /// Cycle focus: List → Items → List (Tab key)
    pub fn cycle_focus(&mut self) {
        self.focus = match self.focus {
            Focus::List   => Focus::Items,
            Focus::Items  => Focus::List,
            Focus::Vaults => Focus::List,
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
                ItemFilter::Favorites => false, // TODO: bw doesn't expose favorites in JSON yet
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
            ItemFilter::Favorites => 0,
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
                Ok(_) => { self.load_items(); self.go_to_vault(); self.set_status("Vault unlocked ✓", false); }
                Err(_) => self.set_login_error(),
            }
        } else {
            match self.bw.login(&email, &password) {
                Ok(_) => { self.load_items(); self.go_to_vault(); self.set_status("Login successful ✓", false); }
                Err(_) => self.set_login_error(),
            }
        }
    }

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

    pub fn sync_vault(&mut self) {
        match self.bw.sync() {
            Ok(()) => { self.load_items(); self.set_status("Vault synced ✓", false); }
            Err(e) => self.set_status(&format!("Sync error: {e}"), true),
        }
    }

    // ── Helpers ───────────────────────────────────────────────────────────

    pub fn set_status(&mut self, msg: &str, is_error: bool) {
        self.status = Some(StatusMessage { text: msg.to_string(), is_error });
    }

    pub fn clear_status(&mut self) {
        self.status = None;
    }

    pub fn set_login_error(&mut self) {
        self.login_error = true;
        self.password_input.clear();
    }

    pub fn clear_login_error(&mut self) {
        self.login_error = false;
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