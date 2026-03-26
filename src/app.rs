/// app.rs — Global application state

use crate::bw::{BwClient, Item, VaultStatus, IdentityData, item_type_label};
use std::process::{Command, Stdio};
use std::io::Write;

// ── Enums ─────────────────────────────────────────────────────────────────

#[derive(Debug, PartialEq, Clone)]
pub enum Screen { Login, Vault, Detail, Help, Create, ConfirmDelete }

#[derive(Debug, PartialEq, Clone)]
pub enum Focus {
    Status, Search, Vaults, Items, List, CmdLog,
}

#[derive(Debug, PartialEq, Clone)]
pub enum ItemFilter {
    All, Favorites, Login, Card, Identity, SecureNote, SshKey,
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
    ItemFilter::All, ItemFilter::Favorites, ItemFilter::Login,
    ItemFilter::Card, ItemFilter::Identity, ItemFilter::SecureNote, ItemFilter::SshKey,
];

#[derive(Debug, PartialEq, Clone)]
pub enum LoginField { Email, Password, SaveEmail, AutoLock }

#[derive(Debug, Clone, PartialEq)]
pub enum ActionState { Idle, Running(String), Done(String), Error(String) }

#[derive(Debug, Clone, PartialEq)]
pub enum PendingAction {
    None,
    Login,
    CopyUsername,
    CopyPassword,
    SyncVault,
    ToggleFavorite,
    CopyRaw(String, String),  // (text, success_msg)
    CopyTotp(String),         // item_id
    SaveEdit,
    CreateItem,
    DeleteItem { permanent: bool },
}

// ── Supporting structs ────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct CmdEntry {
    pub cmd:    String,
    pub ok:     bool,
    pub detail: String,
}

/// Screen areas for mouse hit-testing — updated every frame by ui.rs.
#[derive(Debug, Clone, Default)]
pub struct MouseAreas {
    pub status: Option<ratatui::layout::Rect>,
    pub search: Option<ratatui::layout::Rect>,
    pub vaults: Option<ratatui::layout::Rect>,
    pub items:  Option<ratatui::layout::Rect>,
    pub list:   Option<ratatui::layout::Rect>,
    pub cmdlog: Option<ratatui::layout::Rect>,
    pub detail: Option<ratatui::layout::Rect>,
    pub login:  Option<ratatui::layout::Rect>,
}

impl MouseAreas {
    pub fn focus_for(&self, col: u16, row: u16) -> Option<Focus> {
        let hit = |r: Option<ratatui::layout::Rect>| r.map_or(false, |r| rect_contains(r, col, row));
        if hit(self.status) { return Some(Focus::Status); }
        if hit(self.search) { return Some(Focus::Search); }
        if hit(self.vaults) { return Some(Focus::Vaults); }
        if hit(self.items)  { return Some(Focus::Items);  }
        if hit(self.list)   { return Some(Focus::List);   }
        if hit(self.cmdlog) { return Some(Focus::CmdLog); }
        None
    }
    pub fn list_row(&self, row: u16) -> Option<usize> {
        let r = self.list?;
        if row < r.y + 1 || row >= r.y + r.height.saturating_sub(1) { return None; }
        Some((row - r.y - 1) as usize)
    }
    pub fn items_row(&self, row: u16) -> Option<usize> {
        let r = self.items?;
        if row < r.y + 1 || row >= r.y + r.height.saturating_sub(1) { return None; }
        Some((row - r.y - 1) as usize)
    }
}

pub fn rect_contains(r: ratatui::layout::Rect, col: u16, row: u16) -> bool {
    col >= r.x && col < r.x + r.width && row >= r.y && row < r.y + r.height
}

// ── EditField ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct EditField {
    pub label:     String,
    pub value:     String,
    pub hidden:    bool,
    pub revealed:  bool,
    pub cursor:    usize,
    pub read_only: bool,
}

impl EditField {
    pub fn new(label: &str, value: &str, hidden: bool) -> Self {
        EditField {
            label: label.to_string(), value: value.to_string(),
            hidden, revealed: false, cursor: value.chars().count(), read_only: false,
        }
    }
    pub fn read_only(label: &str, value: &str) -> Self {
        EditField { read_only: true, ..Self::new(label, value, false) }
    }
    pub fn insert(&mut self, c: char) {
        if self.read_only { return; }
        let byte = self.char_byte(self.cursor);
        self.value.insert(byte, c);
        self.cursor += 1;
    }
    pub fn delete_before(&mut self) {
        if self.read_only || self.cursor == 0 { return; }
        let byte = self.char_byte(self.cursor - 1);
        self.value.remove(byte);
        self.cursor -= 1;
    }
    pub fn delete_at(&mut self) {
        if self.read_only || self.cursor >= self.value.chars().count() { return; }
        let byte = self.char_byte(self.cursor);
        self.value.remove(byte);
    }
    pub fn cursor_left(&mut self)  { if self.cursor > 0 { self.cursor -= 1; } }
    pub fn cursor_right(&mut self) { if self.cursor < self.value.chars().count() { self.cursor += 1; } }
    pub fn cursor_home(&mut self)  { self.cursor = 0; }
    pub fn cursor_end(&mut self)   { self.cursor = self.value.chars().count(); }

    fn char_byte(&self, char_idx: usize) -> usize {
        self.value.char_indices().nth(char_idx).map(|(b, _)| b).unwrap_or(self.value.len())
    }
}

// ── CreateItemType ────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum CreateItemType { Login, SecureNote, Card, Identity }

impl CreateItemType {
    pub fn label(&self) -> &'static str {
        match self {
            CreateItemType::Login      => "Login",
            CreateItemType::SecureNote => "Secure Note",
            CreateItemType::Card       => "Card",
            CreateItemType::Identity   => "Identity",
        }
    }
}

pub const CREATE_ITEM_TYPES: &[CreateItemType] = &[
    CreateItemType::Login, CreateItemType::SecureNote,
    CreateItemType::Card,  CreateItemType::Identity,
];

// ── App struct ────────────────────────────────────────────────────────────

pub struct App {
    pub screen:       Screen,
    pub should_quit:  bool,

    pub focus:           Focus,
    pub active_filter:   ItemFilter,
    pub filter_selected: usize,

    pub items:          Vec<Item>,
    pub selected_index: usize,
    pub scroll_offset:  usize,

    pub email_input:     String,
    pub email_cursor:    usize,
    pub password_input:  String,
    pub password_cursor: usize,
    pub active_field:    LoginField,
    pub login_error:     bool,
    pub save_email:      bool,

    pub search_query: String,

    pub show_password: bool,
    pub detail_field:  usize,

    pub cmd_log:        Vec<CmdEntry>,
    pub cmd_log_scroll: usize,
    pub action_state:   ActionState,
    pub action_tick:    u8,
    pub pending_action: PendingAction,

    pub auto_lock:       bool,
    pub lock_after_secs: u64,
    pub last_activity:   std::time::Instant,

    pub mouse_areas: MouseAreas,
    pub last_click:  Option<(u16, u16)>,

    pub bw:    BwClient,
    pub theme: crate::theme::Theme,

    pub edit_fields:      Vec<EditField>,
    pub edit_field_idx:   usize,
    pub edit_item_id:     String,

    pub create_fields:       Vec<EditField>,
    pub create_field_idx:    usize,
    pub create_type:         CreateItemType,
    pub create_type_idx:     usize,
    pub create_choosing_type: bool,

    pub edit_mode: bool,
}

impl App {
    pub fn new() -> Self {
        let cfg         = config::read();
        let saved_email = cfg.email.unwrap_or_default();
        App {
            screen: Screen::Login, should_quit: false,
            focus: Focus::List, active_filter: ItemFilter::All, filter_selected: 0,
            items: Vec::new(), selected_index: 0, scroll_offset: 0,
            email_input: saved_email.clone(), email_cursor: saved_email.len(),
            password_input: String::new(), password_cursor: 0,
            active_field: if cfg.save_email { LoginField::Password } else { LoginField::Email },
            login_error: false, save_email: cfg.save_email,
            search_query: String::new(),
            show_password: false, detail_field: 0,
            cmd_log: Vec::new(), cmd_log_scroll: 0,
            action_state: ActionState::Idle, action_tick: 0,
            pending_action: PendingAction::None,
            auto_lock: cfg.auto_lock, lock_after_secs: cfg.lock_after_secs,
            last_activity: std::time::Instant::now(),
            mouse_areas: MouseAreas::default(), last_click: None,
            bw: BwClient::new(),
            theme: crate::theme::load(&config::config_path()),
            edit_fields: Vec::new(), edit_field_idx: 0, edit_item_id: String::new(),
            create_fields: Vec::new(), create_field_idx: 0,
            create_type: CreateItemType::Login, create_type_idx: 0,
            create_choosing_type: true,
            edit_mode: false,
        }
    }

    // ── Session resume ────────────────────────────────────────────────────

    pub fn resume_from_status(&mut self) {
        let info = match self.bw.status() {
            Ok(i)  => i,
            Err(e) => { self.push_cmd("bw status", false, &e); return; }
        };
        self.push_cmd("bw status", true, &format!("{:?}", info.status));

        match info.status {
            VaultStatus::Unlocked => {
                if let Ok(key) = std::env::var("BW_SESSION") {
                    if !key.trim().is_empty() {
                        self.bw.session_key = Some(key.trim().to_string());
                        if self.email_input.is_empty() {
                            if let Some(email) = info.user_email {
                                self.email_cursor = email.len();
                                self.email_input  = email;
                            }
                        }
                        self.push_cmd("bw status", true, "session resumed from BW_SESSION");
                        self.load_items();
                        self.go_to_vault();
                        return;
                    }
                }
                self.apply_locked_state(info.user_email);
            }
            VaultStatus::Locked         => self.apply_locked_state(info.user_email),
            VaultStatus::Unauthenticated => {}
        }
    }

    fn apply_locked_state(&mut self, user_email: Option<String>) {
        if let Some(email) = user_email {
            if !email.is_empty() && self.email_input.is_empty() {
                self.email_cursor = email.len();
                self.email_input  = email;
            }
        }
        self.active_field = LoginField::Password;
    }

    // ── Lock / auto-lock ──────────────────────────────────────────────────

    pub fn lock_vault(&mut self) {
        self.bw.lock();
        self.screen = Screen::Login;
        self.items.clear();
        self.password_input.clear();
        self.password_cursor = 0;
        self.active_field = LoginField::Password;
        self.push_cmd("bw lock", true, "vault locked");
        self.set_action(ActionState::Done("Locked ✓".into()));
    }

    pub fn check_auto_lock(&mut self) {
        if !self.auto_lock { return; }
        if self.screen != Screen::Vault && self.screen != Screen::Detail { return; }
        if self.last_activity.elapsed().as_secs() >= self.lock_after_secs {
            self.lock_vault();
        }
    }

    pub fn reset_activity(&mut self) { self.last_activity = std::time::Instant::now(); }

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
            self.detail_field = 0;
        }
    }

    pub fn go_back(&mut self) {
        match self.screen {
            Screen::Detail => {
                if self.edit_mode { self.edit_mode = false; }
                else              { self.screen = Screen::Vault; }
            }
            Screen::Help | Screen::Create | Screen::ConfirmDelete => {
                self.screen = Screen::Vault;
            }
            _ => {}
        }
    }

    // ── Focus ─────────────────────────────────────────────────────────────

    pub fn cycle_focus(&mut self) {
        self.focus = match self.focus {
            Focus::Status | Focus::CmdLog => Focus::Search,
            Focus::Search  => Focus::Vaults,
            Focus::Vaults  => Focus::Items,
            Focus::Items   => Focus::List,
            Focus::List    => Focus::CmdLog,
        };
    }

    pub fn focus_panel(&mut self, n: u8) {
        self.focus = match n {
            0 => Focus::Search,
            1 => Focus::Vaults,
            2 => Focus::Items,
            3 => Focus::List,
            4 => Focus::CmdLog,
            _ => return,
        };
    }

    // ── Filter ────────────────────────────────────────────────────────────

    pub fn filter_move_down(&mut self) {
        if self.filter_selected < ITEM_FILTERS.len() - 1 { self.filter_selected += 1; }
    }
    pub fn filter_move_up(&mut self) {
        if self.filter_selected > 0 { self.filter_selected -= 1; }
    }
    pub fn apply_filter(&mut self) {
        self.active_filter  = ITEM_FILTERS[self.filter_selected].clone();
        self.selected_index = 0;
        self.scroll_offset  = 0;
        self.focus          = Focus::List;
    }

    // ── List navigation ───────────────────────────────────────────────────

    pub fn move_down(&mut self) {
        let len = self.filtered_items().len();
        if len > 0 && self.selected_index < len - 1 {
            self.selected_index += 1;
            if self.selected_index >= self.scroll_offset + 20 { self.scroll_offset += 1; }
        }
    }
    pub fn move_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            if self.selected_index < self.scroll_offset { self.scroll_offset = self.selected_index; }
        }
    }
    pub fn move_down_page(&mut self) { for _ in 0..10 { self.move_down(); } }
    pub fn move_up_page(&mut self)   { for _ in 0..10 { self.move_up();   } }

    // ── Vault data ────────────────────────────────────────────────────────

    pub fn filtered_items(&self) -> Vec<&Item> {
        let base: Vec<&Item> = self.items.iter().filter(|item| match &self.active_filter {
            ItemFilter::All       => true,
            ItemFilter::Favorites => item.favorite,
            f                     => f.type_id() == Some(item.item_type),
        }).collect();

        if self.search_query.is_empty() { return base; }

        let query = self.search_query.to_lowercase();
        let mut scored: Vec<(i32, &Item)> = base.into_iter()
            .filter_map(|item| { let s = fuzzy_score(item, &query); if s > 0 { Some((s, item)) } else { None } })
            .collect();
        scored.sort_by(|a, b| b.0.cmp(&a.0));
        scored.into_iter().map(|(_, i)| i).collect()
    }

    pub fn selected_item(&self) -> Option<&Item> {
        self.filtered_items().get(self.selected_index).copied()
    }

    pub fn count_for(&self, filter: &ItemFilter) -> usize {
        match filter {
            ItemFilter::All       => self.items.len(),
            ItemFilter::Favorites => self.items.iter().filter(|i| i.favorite).count(),
            f                     => self.items.iter().filter(|i| f.type_id() == Some(i.item_type)).count(),
        }
    }

    pub fn perform_search(&mut self) { self.selected_index = 0; self.scroll_offset = 0; }

    pub fn clear_search(&mut self) {
        self.search_query.clear();
        self.focus = Focus::List;
        self.selected_index = 0;
        self.scroll_offset = 0;
    }

    // ── Authentication ────────────────────────────────────────────────────

    pub fn attempt_login(&mut self) {
        if self.email_input.trim().is_empty() || self.password_input.is_empty() {
            self.login_error = true;
            return;
        }
        self.set_action(ActionState::Running("Logging in…".into()));
        self.pending_action = PendingAction::Login;
    }

    pub fn do_login(&mut self) {
        let email    = self.email_input.clone();
        let password = self.password_input.clone();
        let already_auth = matches!(
            self.bw.status().map(|s| s.status),
            Ok(VaultStatus::Locked) | Ok(VaultStatus::Unlocked)
        );
        let result = if already_auth { self.bw.unlock(&password) } else { self.bw.login(&email, &password) };

        match result {
            Ok(_) => {
                if self.save_email { config::write(true, Some(&email)); }
                self.password_input.clear();
                self.password_cursor = 0;
                self.load_items();
                self.set_action(ActionState::Done("Loaded ✓".into()));
                self.go_to_vault();
            }
            Err(_) => {
                self.push_cmd("bw auth *** --raw", false, "invalid credentials");
                self.set_action(ActionState::Idle);
                self.set_login_error();
            }
        }
    }

    pub fn load_items(&mut self) {
        let cmd = format!("bw list items --session {}", self.session_key_display());
        self.set_action(ActionState::Running("Loading vault…".into()));
        match self.bw.list_items() {
            Ok(items) => {
                let count = items.len();
                self.items = items;
                self.sort_items();
                self.push_cmd(&cmd, true, &format!("{count} items loaded"));
            }
            Err(e) => self.cmd_err(&cmd, &e, "Load failed"),
        }
    }

    // ── Clipboard actions ─────────────────────────────────────────────────

    fn queue_copy(&mut self, action: PendingAction, msg: &str) {
        self.set_action(ActionState::Running(msg.to_string()));
        self.pending_action = action;
    }

    pub fn copy_username_to_clipboard(&mut self) {
        if self.selected_item().is_some() { self.queue_copy(PendingAction::CopyUsername, "Copying user…"); }
    }
    pub fn copy_password_to_clipboard(&mut self) {
        if self.selected_item().is_some() { self.queue_copy(PendingAction::CopyPassword, "Copying pass…"); }
    }
    pub fn toggle_favorite(&mut self) {
        if self.selected_item().is_some() { self.queue_copy(PendingAction::ToggleFavorite, "Updating…"); }
    }
    pub fn sync_vault(&mut self) { self.queue_copy(PendingAction::SyncVault, "Syncing…"); }

    pub fn copy_selected_field(&mut self) {
        let item = match self.selected_item() { Some(i) => i.clone(), None => return };
        let mut idx = 0usize;

        if self.detail_field == idx { return self.queue_copy(PendingAction::CopyRaw(item.name.clone(), "Name copied ✓".into()), "Copying…"); }
        idx += 1;
        if self.detail_field == idx { return; } // Type — not useful to copy
        idx += 1;

        if let Some(login) = item.login.as_ref() {
            if login.username.is_some() {
                if self.detail_field == idx { return self.copy_username_to_clipboard(); }
                idx += 1;
            }
            if self.detail_field == idx { return self.copy_password_to_clipboard(); }
            idx += 1;
            for uri in login.uris.iter().flat_map(|u| u.iter()).filter_map(|u| u.uri.as_ref()) {
                if self.detail_field == idx { return self.queue_copy(PendingAction::CopyRaw(uri.clone(), "URL copied ✓".into()), "Copying…"); }
                idx += 1;
            }
            if login.totp.is_some() {
                if self.detail_field == idx { return self.queue_copy(PendingAction::CopyTotp(item.id.clone()), "Copying TOTP…"); }
                idx += 1;
            }
        }

        if let Some(card) = item.card.as_ref() {
            for (val, lbl) in [
                (card.cardholder_name.as_deref(), "Cardholder"),
                (card.brand.as_deref(),           "Brand"),
                (card.number.as_deref(),          "Number"),
            ] {
                if let Some(v) = val { if !v.is_empty() {
                    if self.detail_field == idx { return self.queue_copy(PendingAction::CopyRaw(v.into(), format!("{lbl} copied ✓")), "Copying…"); }
                    idx += 1;
                }}
            }
            if card.exp_month.is_some() || card.exp_year.is_some() {
                if self.detail_field == idx {
                    let v = format!("{}/{}", card.exp_month.as_deref().unwrap_or("?"), card.exp_year.as_deref().unwrap_or("?"));
                    return self.queue_copy(PendingAction::CopyRaw(v, "Expiry copied ✓".into()), "Copying…");
                }
                idx += 1;
            }
            if let Some(v) = card.code.as_deref() { if !v.is_empty() {
                if self.detail_field == idx { return self.queue_copy(PendingAction::CopyRaw(v.into(), "CVV copied ✓".into()), "Copying…"); }
                idx += 1;
            }}
        }

        if let Some(id) = item.identity.as_ref() {
            let full = build_full_name(id.title.as_deref(), id.first_name.as_deref(), id.middle_name.as_deref(), id.last_name.as_deref());
            if !full.is_empty() {
                if self.detail_field == idx { return self.queue_copy(PendingAction::CopyRaw(full, "Name copied ✓".into()), "Copying…"); }
                idx += 1;
            }
            for (lbl, val) in identity_fields(id) {
                if let Some(v) = val { if !v.is_empty() {
                    if self.detail_field == idx { return self.queue_copy(PendingAction::CopyRaw(v.to_string(), format!("{lbl} copied ✓")), "Copying…"); }
                    idx += 1;
                }}
            }
        }

        for field in &item.fields {
            let value = field.value.as_deref().unwrap_or("");
            let label = field.name.as_deref().unwrap_or("Field");
            if self.detail_field == idx { return self.queue_copy(PendingAction::CopyRaw(value.into(), format!("{label} copied ✓")), "Copying…"); }
            idx += 1;
        }

        if let Some(notes) = &item.notes { if !notes.is_empty() {
            if self.detail_field == idx { self.queue_copy(PendingAction::CopyRaw(notes.clone(), "Notes copied ✓".into()), "Copying…"); }
        }}
    }

    // ── Deferred copy executors ───────────────────────────────────────────

    /// Shared executor for single-value bw-fetched copies.
    fn do_bw_copy(&mut self, cmd: &str, result: Result<String, String>, label: &str, success_msg: &str) {
        match result {
            Ok(v)  => { self.set_action(ActionState::Done("Copied ✓".into())); self.push_cmd(cmd, true, label); self.write_clipboard(v, success_msg); }
            Err(e) => self.cmd_err(cmd, &e, "Failed"),
        }
    }

    pub fn do_copy_username(&mut self) {
        let Some(item) = self.selected_item() else { return };
        let (id, name) = (item.id.clone(), item.name.clone());
        let cmd    = format!("bw get username {} --session {}", id, self.session_key_display());
        let result = self.bw.get_username(&id);
        self.do_bw_copy(&cmd, result, &format!("username for {name}"), "Username copied ✓");
    }

    pub fn do_copy_password(&mut self) {
        let Some(item) = self.selected_item() else { return };
        let (id, name) = (item.id.clone(), item.name.clone());
        let cmd    = format!("bw get password {} --session {}", id, self.session_key_display());
        let result = self.bw.get_password(&id);
        self.do_bw_copy(&cmd, result, &format!("password for {name} [hidden]"), "Password copied ✓");
    }

    pub fn do_copy_totp(&mut self, item_id: String) {
        let cmd    = format!("bw get totp {} --session {}", item_id, self.session_key_display());
        let result = self.bw.get_totp(&item_id);
        self.do_bw_copy(&cmd, result, "totp [hidden]", "TOTP copied ✓");
    }

    pub fn do_copy_raw(&mut self, text: String, msg: String) {
        self.set_action(ActionState::Done("Copied ✓".into()));
        self.push_cmd("clipboard", true, &msg);
        self.write_clipboard(text, &msg);
    }

    pub fn do_toggle_favorite(&mut self) {
        let Some(item) = self.selected_item() else { return };
        let (id, name, new_fav) = (item.id.clone(), item.name.clone(), !item.favorite);
        let cmd = format!("bw edit item {} --session {}", id, self.session_key_display());
        match self.bw.set_favorite(&id, new_fav) {
            Ok(_) => {
                if let Some(i) = self.items.iter_mut().find(|i| i.id == id) { i.favorite = new_fav; }
                let label = if new_fav { "★ Favorited" } else { "Unfavorited" };
                self.set_action(ActionState::Done(label.into()));
                self.push_cmd(&cmd, true, &format!("{name} {label}"));
            }
            Err(e) => self.cmd_err(&cmd, &e, "Failed"),
        }
    }

    pub fn do_sync_vault(&mut self) {
        let cmd = format!("bw sync --session {}", self.session_key_display());
        match self.bw.sync() {
            Ok(())  => { self.set_action(ActionState::Done("Synced ✓".into())); self.push_cmd(&cmd, true, "vault synced"); self.load_items(); }
            Err(e)  => self.cmd_err(&cmd, &e, "Sync failed"),
        }
    }

    // ── Create ────────────────────────────────────────────────────────────

    pub fn open_create(&mut self) {
        self.create_type_idx      = 0;
        self.create_type          = CreateItemType::Login;
        self.create_choosing_type = true;
        self.create_fields        = Vec::new();
        self.create_field_idx     = 0;
        self.screen               = Screen::Create;
    }

    pub fn create_select_type(&mut self) {
        self.create_type      = CREATE_ITEM_TYPES[self.create_type_idx].clone();
        self.create_fields    = build_create_fields(&self.create_type);
        self.create_field_idx = 0;
        self.create_choosing_type = false;
    }

    /// Delegate a key to the currently focused create field.
    fn create_field_mut(&mut self) -> Option<&mut EditField> {
        self.create_fields.get_mut(self.create_field_idx)
    }
    pub fn create_insert_char(&mut self, c: char)   { if let Some(f) = self.create_field_mut() { f.insert(c); } }
    pub fn create_delete_before(&mut self)           { if let Some(f) = self.create_field_mut() { f.delete_before(); } }
    pub fn create_delete_at(&mut self)               { if let Some(f) = self.create_field_mut() { f.delete_at(); } }
    pub fn create_cursor_left(&mut self)             { if let Some(f) = self.create_field_mut() { f.cursor_left(); } }
    pub fn create_cursor_right(&mut self)            { if let Some(f) = self.create_field_mut() { f.cursor_right(); } }
    pub fn create_cursor_home(&mut self)             { if let Some(f) = self.create_field_mut() { f.cursor_home(); } }
    pub fn create_cursor_end(&mut self)              { if let Some(f) = self.create_field_mut() { f.cursor_end(); } }

    pub fn queue_create_item(&mut self) {
        let name = self.create_fields.first().map(|f| f.value.trim().to_string()).unwrap_or_default();
        if name.is_empty() { self.set_action(ActionState::Error("Name is required".into())); return; }
        self.set_action(ActionState::Running("Creating…".into()));
        self.pending_action = PendingAction::CreateItem;
    }

    pub fn do_create_item(&mut self) {
        let json = build_item_json_from_fields(&self.create_type, &self.create_fields);
        let cmd  = format!("bw create item --session {}", self.session_key_display());
        match self.bw.create_item(&json) {
            Ok(item) => {
                let name = item.name.clone();
                self.items.push(item);
                self.sort_items();
                if let Some(idx) = self.items.iter().position(|i| i.name == name) {
                    self.selected_index = idx;
                    self.scroll_offset  = idx.saturating_sub(5);
                }
                self.push_cmd(&cmd, true, &format!("created: {name}"));
                self.set_action(ActionState::Done("Created ✓".into()));
                self.screen = Screen::Vault;
            }
            Err(e) => self.cmd_err(&cmd, &e, "Create failed"),
        }
    }

    // ── Edit ──────────────────────────────────────────────────────────────

    pub fn enter_edit_mode(&mut self) {
        let Some(item) = self.selected_item() else { return };
        let id     = item.id.clone();
        let fields = build_edit_fields(item);
        let detail = self.detail_field;
        self.edit_item_id   = id;
        self.edit_fields    = fields;
        self.edit_field_idx = detail.min(self.edit_fields.len().saturating_sub(1));
        self.edit_mode = true;
    }

    /// Delegate a key to the currently focused edit field.
    fn edit_field_mut(&mut self) -> Option<&mut EditField> {
        self.edit_fields.get_mut(self.edit_field_idx)
    }
    pub fn edit_insert_char(&mut self, c: char)   { if let Some(f) = self.edit_field_mut() { f.insert(c); } }
    pub fn edit_delete_before(&mut self)           { if let Some(f) = self.edit_field_mut() { f.delete_before(); } }
    pub fn edit_delete_at(&mut self)               { if let Some(f) = self.edit_field_mut() { f.delete_at(); } }
    pub fn edit_cursor_left(&mut self)             { if let Some(f) = self.edit_field_mut() { f.cursor_left(); } }
    pub fn edit_cursor_right(&mut self)            { if let Some(f) = self.edit_field_mut() { f.cursor_right(); } }
    pub fn edit_cursor_home(&mut self)             { if let Some(f) = self.edit_field_mut() { f.cursor_home(); } }
    pub fn edit_cursor_end(&mut self)              { if let Some(f) = self.edit_field_mut() { f.cursor_end(); } }
    pub fn edit_toggle_reveal(&mut self) {
        if let Some(f) = self.edit_field_mut() { if f.hidden { f.revealed = !f.revealed; } }
    }

    pub fn queue_save_edit(&mut self) {
        self.set_action(ActionState::Running("Saving…".into()));
        self.pending_action = PendingAction::SaveEdit;
    }

    pub fn do_save_edit(&mut self) {
        let item_id = self.edit_item_id.clone();
        let cmd     = format!("bw edit item {item_id} --session {}", self.session_key_display());
        let base_json = match self.bw.get_item_json(&item_id) {
            Ok(j)  => j,
            Err(e) => { self.cmd_err(&cmd, &e, "Fetch failed"); return; }
        };
        let patched = patch_item_json(&base_json, &self.edit_fields);
        match self.bw.edit_item(&item_id, &patched) {
            Ok(updated) => {
                let name = updated.name.clone();
                if let Some(i) = self.items.iter_mut().find(|i| i.id == item_id) { *i = updated; }
                self.sort_items();
                self.push_cmd(&cmd, true, &format!("saved: {name}"));
                self.set_action(ActionState::Done("Saved ✓".into()));
                self.edit_mode = false;
            }
            Err(e) => self.cmd_err(&cmd, &e, "Save failed"),
        }
    }

    // ── Delete ────────────────────────────────────────────────────────────

    pub fn open_confirm_delete(&mut self) {
        if self.selected_item().is_some() { self.screen = Screen::ConfirmDelete; }
    }

    pub fn queue_delete_item(&mut self, permanent: bool) {
        self.set_action(ActionState::Running(if permanent { "Deleting…" } else { "Trashing…" }.into()));
        self.pending_action = PendingAction::DeleteItem { permanent };
        self.screen = Screen::Vault;
    }

    pub fn do_delete_item(&mut self, permanent: bool) {
        let Some(item) = self.selected_item() else { return };
        let (id, name) = (item.id.clone(), item.name.clone());
        let perm_str   = if permanent { " --permanent" } else { "" };
        let cmd        = format!("bw delete item {id}{perm_str} --session {}", self.session_key_display());
        match self.bw.delete_item(&id, permanent) {
            Ok(()) => {
                self.items.retain(|i| i.id != id);
                if self.selected_index >= self.items.len() && !self.items.is_empty() {
                    self.selected_index = self.items.len() - 1;
                }
                let label = if permanent { "deleted permanently" } else { "moved to trash" };
                self.push_cmd(&cmd, true, &format!("{name} {label}"));
                self.set_action(ActionState::Done(if permanent { "Deleted ✓".into() } else { "Trashed ✓".into() }));
            }
            Err(e) => self.cmd_err(&cmd, &e, "Delete failed"),
        }
    }

    // ── Detail field count ────────────────────────────────────────────────

    pub fn detail_field_count(&self) -> usize {
        let Some(item) = self.selected_item() else { return 0 };
        let mut n = 2; // Name + Type
        if let Some(l) = &item.login {
            if l.username.is_some() { n += 1; }
            n += 1; // password always shown
            n += l.uris.as_ref().map_or(0, |u| u.iter().filter(|x| x.uri.is_some()).count());
            if l.totp.is_some() { n += 1; }
        }
        if let Some(c) = &item.card {
            for v in [&c.cardholder_name, &c.brand, &c.number, &c.code] {
                if v.as_ref().map_or(false, |s| !s.is_empty()) { n += 1; }
            }
            if c.exp_month.is_some() || c.exp_year.is_some() { n += 1; }
        }
        if let Some(id) = &item.identity {
            let full = build_full_name(id.title.as_deref(), id.first_name.as_deref(), id.middle_name.as_deref(), id.last_name.as_deref());
            if !full.is_empty() { n += 1; }
            n += identity_fields(id).into_iter().filter(|(_, v)| v.as_ref().map_or(false, |s| !s.is_empty())).count();
        }
        n += item.fields.iter().filter(|f| f.value.as_ref().map_or(false, |v| !v.is_empty())).count();
        if item.notes.as_ref().map_or(false, |s| !s.is_empty()) { n += 1; }
        n
    }

    // ── Login field editing ───────────────────────────────────────────────

    /// Returns (input, cursor) mutable refs for the active text field, or None for checkboxes.
    fn login_text_mut(&mut self) -> Option<(&mut String, &mut usize)> {
        match self.active_field {
            LoginField::Email    => Some((&mut self.email_input,    &mut self.email_cursor)),
            LoginField::Password => Some((&mut self.password_input, &mut self.password_cursor)),
            _                    => None,
        }
    }

    pub fn insert_char(&mut self, c: char) {
        let save = self.active_field == LoginField::Email && self.save_email;
        if let Some((input, cursor)) = self.login_text_mut() {
            let byte = input.char_indices().nth(*cursor).map(|(b,_)| b).unwrap_or(input.len());
            input.insert(byte, c);
            *cursor += 1;
        }
        if save { let e = self.email_input.clone(); config::write(true, Some(&e)); }
    }

    pub fn delete_char_before(&mut self) {
        let save = self.active_field == LoginField::Email && self.save_email;
        if let Some((input, cursor)) = self.login_text_mut() {
            if *cursor > 0 {
                let byte = input.char_indices().nth(*cursor - 1).map(|(b,_)| b).unwrap_or(0);
                input.remove(byte);
                *cursor -= 1;
            }
        }
        if save { let e = self.email_input.clone(); config::write(true, Some(&e)); }
    }

    pub fn delete_char_at(&mut self) {
        let save = self.active_field == LoginField::Email && self.save_email;
        if let Some((input, cursor)) = self.login_text_mut() {
            if *cursor < input.chars().count() {
                let byte = input.char_indices().nth(*cursor).map(|(b,_)| b).unwrap_or(0);
                input.remove(byte);
            }
        }
        if save { let e = self.email_input.clone(); config::write(true, Some(&e)); }
    }

    pub fn cursor_left(&mut self) {
        if let Some((_, cursor)) = self.login_text_mut() { if *cursor > 0 { *cursor -= 1; } }
    }
    pub fn cursor_right(&mut self) {
        // need to read len first without holding the mut ref
        let len = match self.active_field {
            LoginField::Email    => self.email_input.chars().count(),
            LoginField::Password => self.password_input.chars().count(),
            _                    => return,
        };
        if let Some((_, cursor)) = self.login_text_mut() { if *cursor < len { *cursor += 1; } }
    }
    pub fn cursor_home(&mut self) {
        if let Some((_, cursor)) = self.login_text_mut() { *cursor = 0; }
    }
    pub fn cursor_end(&mut self) {
        let len = match self.active_field {
            LoginField::Email    => self.email_input.chars().count(),
            LoginField::Password => self.password_input.chars().count(),
            _                    => return,
        };
        if let Some((_, cursor)) = self.login_text_mut() { *cursor = len; }
    }

    pub fn toggle_save_email(&mut self) {
        self.save_email = !self.save_email;
        if self.save_email { let e = self.email_input.clone(); config::write(true, Some(&e)); }
        else               { config::write(false, None); }
    }

    // ── Command log ───────────────────────────────────────────────────────

    pub fn push_cmd(&mut self, cmd: &str, ok: bool, detail: &str) {
        let redacted = cmd.replace(self.bw.session_key.as_deref().unwrap_or("__NO_SESSION__"), "***");
        self.cmd_log.push(CmdEntry { cmd: redacted, ok, detail: detail.to_string() });
        if self.cmd_log.len() > 50 { self.cmd_log.remove(0); }
        self.cmd_log_scroll = 0;
    }

    pub fn cmd_log_scroll_up(&mut self, n: usize) {
        let max = self.cmd_log.len().saturating_sub(1);
        self.cmd_log_scroll = (self.cmd_log_scroll + n).min(max);
    }
    pub fn cmd_log_scroll_down(&mut self, n: usize) {
        self.cmd_log_scroll = self.cmd_log_scroll.saturating_sub(n);
    }

    // ── Action state ──────────────────────────────────────────────────────

    pub fn set_action(&mut self, state: ActionState) { self.action_state = state; self.action_tick = 0; }
    pub fn tick_action(&mut self) { self.action_tick = self.action_tick.wrapping_add(1); }

    // ── Status / errors ───────────────────────────────────────────────────

    pub fn set_login_error(&mut self) {
        self.login_error = true;
        self.password_input.clear();
        self.password_cursor = 0;
    }
    pub fn clear_login_error(&mut self) { self.login_error = false; }

    // ── Clipboard ─────────────────────────────────────────────────────────

    fn write_clipboard(&mut self, text: String, success_msg: &str) {

        let args: Option<Vec<&str>> = if std::env::var("WAYLAND_DISPLAY").is_ok() {
            Some(vec!["wl-copy"])
        } else if std::env::var("DISPLAY").is_ok() {
            if std::path::Path::new("/usr/bin/xclip").exists()
            || std::path::Path::new("/usr/local/bin/xclip").exists() {
                Some(vec!["xclip", "-selection", "clipboard"])
            } else {
                Some(vec!["xsel", "--clipboard", "--input"])
            }
        } else if cfg!(target_os = "macos") {
            Some(vec!["pbcopy"])
        } else { None };

        let Some(args) = args else {
            let msg = "No clipboard tool found (install wl-copy or xclip)";
            self.push_cmd("clipboard", false, msg);
            self.set_action(ActionState::Error(msg.to_string()));
            return;
        };

        let mut cmd = Command::new(args[0]);
        for a in &args[1..] { cmd.arg(a); }
        cmd.stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::null());

        match cmd.spawn() {
            Ok(mut child) => {
                if let Some(mut stdin) = child.stdin.take() { let _ = stdin.write_all(text.as_bytes()); }
                drop(child);
                self.push_cmd(&format!("echo [hidden] | {}", args[0]), true, success_msg);
            }
            Err(e) => {
                self.push_cmd(args[0], false, &format!("spawn failed: {e}"));
                self.set_action(ActionState::Error(format!("Clipboard error: {e}")));
            }
        }
    }

    fn session_key_display(&self) -> &str {
        self.bw.session_key.as_deref().unwrap_or("***")
    }

    /// Sort self.items alphabetically by lowercase name — called after any mutation.
    fn sort_items(&mut self) {
        self.items.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    }

    /// Log a failed bw command and set an error action state.
    fn cmd_err(&mut self, cmd: &str, e: &str, label: &str) {
        self.push_cmd(cmd, false, e);
        self.set_action(ActionState::Error(format!("{label}: {e}")));
    }
}

// ── Edit / Create field builders ──────────────────────────────────────────

pub fn build_edit_fields(item: &Item) -> Vec<EditField> {
    let mut f = vec![
        EditField::new("Name", &item.name, false),
        EditField::read_only("Type", item_type_label(item.item_type)),
    ];
    if let Some(l) = &item.login {
        f.push(EditField::new("Username", l.username.as_deref().unwrap_or(""), false));
        f.push(EditField::new("Password", l.password.as_deref().unwrap_or(""), true));
        for uri in l.uris.iter().flatten() {
            f.push(EditField::new("URL", uri.uri.as_deref().unwrap_or(""), false));
        }
        if let Some(t) = &l.totp { f.push(EditField::new("TOTP seed", t, true)); }
    }
    if let Some(c) = &item.card {
        f.push(EditField::new("Cardholder", c.cardholder_name.as_deref().unwrap_or(""), false));
        f.push(EditField::new("Brand",      c.brand.as_deref().unwrap_or(""),            false));
        f.push(EditField::new("Number",     c.number.as_deref().unwrap_or(""),           true));
        f.push(EditField::new("Exp Month",  c.exp_month.as_deref().unwrap_or(""),        false));
        f.push(EditField::new("Exp Year",   c.exp_year.as_deref().unwrap_or(""),         false));
        f.push(EditField::new("CVV",        c.code.as_deref().unwrap_or(""),             true));
    }
    if let Some(id) = &item.identity {
        for (lbl, val, hid) in [
            ("Title",      id.title.as_deref(),       false),
            ("First Name", id.first_name.as_deref(),  false),
            ("Middle",     id.middle_name.as_deref(), false),
            ("Last Name",  id.last_name.as_deref(),   false),
            ("Email",      id.email.as_deref(),        false),
            ("Phone",      id.phone.as_deref(),        false),
            ("Company",    id.company.as_deref(),      false),
            ("Address",    id.address1.as_deref(),     false),
            ("Address 2",  id.address2.as_deref(),     false),
            ("City",       id.city.as_deref(),         false),
            ("State",      id.state.as_deref(),        false),
            ("ZIP",        id.postal_code.as_deref(),  false),
            ("Country",    id.country.as_deref(),      false),
            ("SSN",        id.ssn.as_deref(),           true),
            ("Passport",   id.passport.as_deref(),      true),
            ("License",    id.license.as_deref(),       true),
        ] { f.push(EditField::new(lbl, val.unwrap_or(""), hid)); }
    }
    for field in &item.fields {
        f.push(EditField::new(
            field.name.as_deref().unwrap_or("Field"),
            field.value.as_deref().unwrap_or(""),
            field.field_type == 1,
        ));
    }
    f.push(EditField::new("Notes", item.notes.as_deref().unwrap_or(""), false));
    f
}

pub fn build_create_fields(item_type: &CreateItemType) -> Vec<EditField> {
    let ef = |l, h| EditField::new(l, "", h);
    match item_type {
        CreateItemType::Login => vec![
            ef("Name", false), ef("Username", false), ef("Password", true),
            ef("URL",  false), ef("Notes",    false),
        ],
        CreateItemType::SecureNote => vec![ef("Name", false), ef("Notes", false)],
        CreateItemType::Card => vec![
            ef("Name", false), ef("Cardholder", false), ef("Brand", false),
            ef("Number", true), ef("Exp Month", false), ef("Exp Year", false),
            ef("CVV", true), ef("Notes", false),
        ],
        CreateItemType::Identity => vec![
            ef("Name",       false), ef("First Name", false), ef("Last Name", false),
            ef("Email",      false), ef("Phone",      false), ef("Company",   false),
            ef("Address",    false), ef("City",       false), ef("State",     false),
            ef("ZIP",        false), ef("Country",    false), ef("Notes",     false),
        ],
    }
}

pub fn build_item_json_from_fields(item_type: &CreateItemType, fields: &[EditField]) -> String {
    let get = |label: &str| fields.iter().find(|f| f.label == label).map(|f| f.value.as_str()).unwrap_or("");
    let esc = |s: &str| s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n");
    match item_type {
        CreateItemType::Login => format!(
            r#"{{"type":1,"name":"{name}","notes":"{notes}","login":{{"username":"{user}","password":"{pass}","uris":[{{"uri":"{url}","match":null}}]}}}}"#,
            name=esc(get("Name")), user=esc(get("Username")), pass=esc(get("Password")),
            url=esc(get("URL")), notes=esc(get("Notes")),
        ),
        CreateItemType::SecureNote => format!(
            r#"{{"type":2,"name":"{name}","notes":"{notes}","secureNote":{{"type":0}}}}"#,
            name=esc(get("Name")), notes=esc(get("Notes")),
        ),
        CreateItemType::Card => format!(
            r#"{{"type":3,"name":"{name}","notes":"{notes}","card":{{"cardholderName":"{holder}","brand":"{brand}","number":"{num}","expMonth":"{em}","expYear":"{ey}","code":"{cvv}"}}}}"#,
            name=esc(get("Name")), holder=esc(get("Cardholder")), brand=esc(get("Brand")),
            num=esc(get("Number")), em=esc(get("Exp Month")), ey=esc(get("Exp Year")),
            cvv=esc(get("CVV")), notes=esc(get("Notes")),
        ),
        CreateItemType::Identity => format!(
            r#"{{"type":4,"name":"{name}","notes":"{notes}","identity":{{"firstName":"{fn}","lastName":"{ln}","email":"{email}","phone":"{phone}","company":"{co}","address1":"{addr}","city":"{city}","state":"{state}","postalCode":"{zip}","country":"{country}"}}}}"#,
            name=esc(get("Name")), fn=esc(get("First Name")), ln=esc(get("Last Name")),
            email=esc(get("Email")), phone=esc(get("Phone")), co=esc(get("Company")),
            addr=esc(get("Address")), city=esc(get("City")), state=esc(get("State")),
            zip=esc(get("ZIP")), country=esc(get("Country")), notes=esc(get("Notes")),
        ),
    }
}

pub fn patch_item_json(base_json: &str, fields: &[EditField]) -> String {
    let Ok(mut val) = serde_json::from_str::<serde_json::Value>(base_json) else {
        return base_json.to_string();
    };
    let get = |label: &str| fields.iter().find(|f| f.label == label).map(|f| f.value.as_str());

    if let Some(v) = get("Name")  { val["name"]  = serde_json::json!(v); }
    if let Some(v) = get("Notes") { val["notes"] = serde_json::json!(v); }

    if val["type"] == 1 {
        if let Some(v) = get("Username")  { val["login"]["username"] = serde_json::json!(v); }
        if let Some(v) = get("Password")  { val["login"]["password"] = serde_json::json!(v); }
        if let Some(v) = get("URL")       { val["login"]["uris"] = serde_json::json!([{"uri": v, "match": null}]); }
        if let Some(v) = get("TOTP seed") { val["login"]["totp"] = serde_json::json!(v); }
    }
    if val["type"] == 3 {
        if let Some(v) = get("Cardholder") { val["card"]["cardholderName"] = serde_json::json!(v); }
        if let Some(v) = get("Brand")      { val["card"]["brand"]    = serde_json::json!(v); }
        if let Some(v) = get("Number")     { val["card"]["number"]   = serde_json::json!(v); }
        if let Some(v) = get("Exp Month")  { val["card"]["expMonth"] = serde_json::json!(v); }
        if let Some(v) = get("Exp Year")   { val["card"]["expYear"]  = serde_json::json!(v); }
        if let Some(v) = get("CVV")        { val["card"]["code"]     = serde_json::json!(v); }
    }
    if val["type"] == 4 {
        for (key, label) in [
            ("firstName","First Name"), ("lastName","Last Name"), ("email","Email"),
            ("phone","Phone"), ("company","Company"), ("address1","Address"),
            ("city","City"), ("state","State"), ("postalCode","ZIP"), ("country","Country"),
            ("ssn","SSN"), ("passportNumber","Passport"), ("licenseNumber","License"),
        ] {
            if let Some(v) = get(label) { val["identity"][key] = serde_json::json!(v); }
        }
    }
    serde_json::to_string(&val).unwrap_or_else(|_| base_json.to_string())
}

// ── Identity helpers ──────────────────────────────────────────────────────

pub fn build_full_name(title: Option<&str>, first: Option<&str>, middle: Option<&str>, last: Option<&str>) -> String {
    [title, first, middle, last]
        .iter()
        .filter_map(|s| s.filter(|x| !x.is_empty()))
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn identity_fields(id: &IdentityData) -> Vec<(&'static str, &Option<String>)> {
    vec![
        ("Email",     &id.email),   ("Phone",   &id.phone),
        ("Company",   &id.company), ("Address", &id.address1),
        ("Address 2", &id.address2),("City",    &id.city),
        ("State",     &id.state),   ("ZIP",     &id.postal_code),
        ("Country",   &id.country), ("SSN",     &id.ssn),
        ("Passport",  &id.passport),("License", &id.license),
    ]
}

// ── Config ────────────────────────────────────────────────────────────────

pub mod config {
    use std::{fs, path::PathBuf};

    pub fn config_path() -> PathBuf {
        let home = std::env::var("HOME").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("."));
        home.join(".config").join("bytewarden")
    }

    fn config_file() -> PathBuf { config_path().join("config.toml") }
    pub fn ensure_dir()         { let _ = fs::create_dir_all(config_path()); }

    #[derive(Default)]
    pub struct Config {
        pub save_email:      bool,
        pub email:           Option<String>,
        pub auto_lock:       bool,
        pub lock_after_secs: u64,
    }

    pub fn read() -> Config {
        ensure_dir();
        let mut cfg = Config { lock_after_secs: 15 * 60, ..Default::default() };
        let Ok(text) = fs::read_to_string(config_file()) else { return cfg };
        for line in text.lines() {
            let line = line.trim();
            if      let Some(v) = line.strip_prefix("save_email = ")        { cfg.save_email = v.trim() == "true"; }
            else if let Some(v) = line.strip_prefix("email = ")             { let v = v.trim().trim_matches('"').to_string(); if !v.is_empty() { cfg.email = Some(v); } }
            else if let Some(v) = line.strip_prefix("auto_lock = ")         { cfg.auto_lock = v.trim() == "true"; }
            else if let Some(v) = line.strip_prefix("lock_after_minutes = ") { if let Ok(m) = v.trim().parse::<u64>() { cfg.lock_after_secs = m * 60; } }
        }
        cfg
    }

    pub fn write_auto_lock(auto_lock: bool) {
        ensure_dir();
        let existing = fs::read_to_string(config_file()).unwrap_or_default();
        let mut lines: Vec<String> = existing.lines()
            .filter(|l| !l.trim().starts_with("auto_lock ="))
            .map(|l| l.to_string()).collect();
        let pos = lines.iter().position(|l| l.trim().starts_with("save_email =")).map(|i| i+1).unwrap_or(0);
        lines.insert(pos, format!("auto_lock = {auto_lock}"));
        let _ = fs::write(config_file(), lines.join("\n") + "\n");
    }

    pub fn write(save_email: bool, email: Option<&str>) {
        ensure_dir();
        let existing = fs::read_to_string(config_file()).unwrap_or_default();
        let mut preserved: Vec<String> = existing.lines()
            .filter(|l| { let t = l.trim(); !t.starts_with("save_email =") && !t.starts_with("email =") })
            .map(|l| l.to_string()).collect();
        while preserved.first().map_or(false, |l| l.trim().is_empty()) { preserved.remove(0); }
        let mut owned = vec![format!("save_email = {save_email}")];
        if save_email { if let Some(e) = email { owned.push(format!("email = \"{e}\"")); } }
        if !preserved.is_empty() { owned.push(String::new()); owned.extend(preserved); }
        let _ = fs::write(config_file(), owned.join("\n") + "\n");
    }
}

// ── Fuzzy scoring ─────────────────────────────────────────────────────────

fn fuzzy_score(item: &Item, query: &str) -> i32 {
    let name = item.name.to_lowercase();
    let mut s = 0i32;
    if name.contains(query)        { s += 100; if name.starts_with(query) { s += 20; } }
    else if is_subseq(query, &name) { s += 50; }
    if let Some(login) = &item.login {
        if let Some(u) = &login.username {
            let u = u.to_lowercase();
            if u.contains(query) { s += 30; } else if is_subseq(query, &u) { s += 10; }
        }
        for u in login.uris.iter().flatten() {
            if let Some(uri) = &u.uri { if uri.to_lowercase().contains(query) { s += 10; break; } }
        }
    }
    if let Some(notes) = &item.notes { if notes.to_lowercase().contains(query) { s += 5; } }
    s
}

fn is_subseq(needle: &str, haystack: &str) -> bool {
    let mut it = haystack.chars();
    needle.chars().all(|c| it.find(|&h| h == c).is_some())
}
