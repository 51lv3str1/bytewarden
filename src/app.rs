/// app.rs — Global application state

use crate::bw::{BwClient, Item};

// ── Enums ─────────────────────────────────────────────────────────────────

#[derive(Debug, PartialEq, Clone)]
pub enum Screen { Login, Vault, Detail, Help }

#[derive(Debug, PartialEq, Clone)]
pub enum Focus {
    Status,  // [5] status pane
    Search,  // [0] search bar
    Vaults,  // [1] vaults panel
    Items,   // [2] items filter
    List,    // [3] vault list
    CmdLog,  // [4] command log
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
pub enum LoginField { Email, Password, SaveEmail }

#[derive(Debug, Clone, PartialEq)]
pub enum ActionState {
    Idle,
    Running(String),
    Done(String),
    Error(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum PendingAction {
    None,
    CopyUsername,
    CopyPassword,
    SyncVault,
    ToggleFavorite,
    CopyRaw(String, String), // (text, success_msg)
    CopyTotp(String),        // item_id
}

// ── Supporting structs ────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct CmdEntry {
    pub cmd:    String,
    pub ok:     bool,
    pub detail: String,
}

#[derive(Debug, Clone, Default)]
pub struct StatusMessage {
    pub text:     String,
    pub is_error: bool,
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

    /// 0-based row within the vault list (accounts for border).
    pub fn list_row(&self, row: u16) -> Option<usize> {
        let r = self.list?;
        if row < r.y + 1 || row >= r.y + r.height.saturating_sub(1) { return None; }
        Some((row - r.y - 1) as usize)
    }

    /// 0-based row within the items filter panel.
    pub fn items_row(&self, row: u16) -> Option<usize> {
        let r = self.items?;
        if row < r.y + 1 || row >= r.y + r.height.saturating_sub(1) { return None; }
        Some((row - r.y - 1) as usize)
    }
}

pub fn rect_contains(r: ratatui::layout::Rect, col: u16, row: u16) -> bool {
    col >= r.x && col < r.x + r.width && row >= r.y && row < r.y + r.height
}

// ── App struct ────────────────────────────────────────────────────────────

pub struct App {
    // Navigation
    pub screen:       Screen,
    pub should_quit:  bool,

    // Sidebar
    pub focus:           Focus,
    pub active_filter:   ItemFilter,
    pub filter_selected: usize,

    // Vault data
    pub items:          Vec<Item>,
    pub selected_index: usize,
    pub scroll_offset:  usize,

    // Login
    pub email_input:    String,
    pub email_cursor:   usize,
    pub password_input: String,
    pub password_cursor:usize,
    pub active_field:   LoginField,
    pub login_error:    bool,
    pub save_email:     bool,

    // Search
    pub search_query: String,

    // Detail
    pub show_password: bool,
    pub detail_field:  usize,

    // Status / feedback
    pub status:         Option<StatusMessage>,
    pub cmd_log:        Vec<CmdEntry>,
    pub cmd_log_scroll: usize,
    pub action_state:   ActionState,
    pub action_tick:    u8,
    pub pending_action: PendingAction,

    // Mouse
    pub mouse_areas: MouseAreas,
    pub last_click:  Option<(u16, u16)>,

    // Core
    pub bw:    BwClient,
    pub theme: crate::theme::Theme,
}

impl App {
    pub fn new() -> Self {
        let cfg          = config::read();
        let saved_email  = cfg.email.unwrap_or_default();
        let email_cursor = saved_email.len();
        App {
            screen: Screen::Login, should_quit: false,
            focus: Focus::List, active_filter: ItemFilter::All, filter_selected: 0,
            items: Vec::new(), selected_index: 0, scroll_offset: 0,
            email_input: saved_email, email_cursor,
            password_input: String::new(), password_cursor: 0,
            active_field: if cfg.save_email { LoginField::Password } else { LoginField::Email },
            login_error: false, save_email: cfg.save_email,
            search_query: String::new(),
            show_password: false, detail_field: 0,
            status: None,
            cmd_log: Vec::new(), cmd_log_scroll: 0,
            action_state: ActionState::Idle, action_tick: 0,
            pending_action: PendingAction::None,
            mouse_areas: MouseAreas::default(), last_click: None,
            bw: BwClient::new(),
            theme: crate::theme::load(&config::config_path()),
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
            self.detail_field = 0;
        }
    }

    pub fn go_back(&mut self) {
        match self.screen {
            Screen::Detail | Screen::Help => {
                self.screen = Screen::Vault;
            }
            _ => {}
        }
    }

    // ── Focus ─────────────────────────────────────────────────────────────

    pub fn cycle_focus(&mut self) {
        self.focus = match self.focus {
            Focus::Status => Focus::Search,
            Focus::Search => Focus::Vaults,
            Focus::Vaults => Focus::Items,
            Focus::Items  => Focus::List,
            Focus::List   => Focus::CmdLog,
            Focus::CmdLog => Focus::Status,
        };
    }

    pub fn focus_panel(&mut self, n: u8) {
        self.focus = match n {
            0 => Focus::Search,
            1 => Focus::Vaults,
            2 => Focus::Items,
            3 => Focus::List,
            4 => Focus::CmdLog,
            5 => Focus::Status,
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
            .filter_map(|item| {
                let s = fuzzy_score(item, &query);
                if s > 0 { Some((s, item)) } else { None }
            })
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

    pub fn perform_search(&mut self) {
        self.selected_index = 0;
        self.scroll_offset = 0;
    }

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
        let email    = self.email_input.clone();
        let password = self.password_input.clone();

        let result = if self.bw.is_logged_in() {
            self.bw.unlock(&password)
        } else {
            self.bw.login(&email, &password)
        };

        match result {
            Ok(_) => {
                if self.save_email { config::write(true, Some(&email)); }
                self.load_items();
                self.go_to_vault();
            }
            Err(_) => {
                self.push_cmd("bw auth *** --raw", false, "invalid credentials");
                self.set_login_error();
            }
        }
    }

    pub fn load_items(&mut self) {
        let cmd = format!("bw list items --session {}", self.session_key_display());
        match self.bw.list_items() {
            Ok(items) => {
                let count = items.len();
                let mut sorted = items;
                sorted.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
                self.items = sorted;
                self.push_cmd(&cmd, true, &format!("{count} items loaded"));
            }
            Err(e) => {
                self.push_cmd(&cmd, false, &e);
                self.set_status(&format!("Error loading items: {e}"), true);
            }
        }
    }

    // ── Clipboard / actions ───────────────────────────────────────────────

    /// Queue a copy action with Running state (deferred to next frame).
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

    pub fn sync_vault(&mut self) {
        self.queue_copy(PendingAction::SyncVault, "Syncing…");
    }

    pub fn copy_selected_field(&mut self) {
        let item = match self.selected_item() { Some(i) => i.clone(), None => return };
        let mut idx = 0usize;

        // Name (idx 0)
        if self.detail_field == idx {
            return self.queue_copy(PendingAction::CopyRaw(item.name.clone(), "Name copied ✓".into()), "Copying…");
        }
        idx += 1;
        // Type (idx 1) — not useful to copy
        if self.detail_field == idx { return; }
        idx += 1;

        // Login
        if let Some(login) = item.login.as_ref() {
            if login.username.is_some() {
                if self.detail_field == idx { return self.copy_username_to_clipboard(); }
                idx += 1;
            }
            if self.detail_field == idx { return self.copy_password_to_clipboard(); }
            idx += 1;
            for uri in login.uris.iter().flat_map(|u| u.iter()).filter_map(|u| u.uri.as_ref()) {
                if self.detail_field == idx {
                    return self.queue_copy(PendingAction::CopyRaw(uri.clone(), "URL copied ✓".into()), "Copying…");
                }
                idx += 1;
            }
            if login.totp.is_some() {
                if self.detail_field == idx {
                    return self.queue_copy(PendingAction::CopyTotp(item.id.clone()), "Copying TOTP…");
                }
                idx += 1;
            }
        }

        // Card
        if let Some(card) = item.card.as_ref() {
            for (val, lbl) in [
                (card.cardholder_name.as_deref(), "Cardholder"),
                (card.brand.as_deref(),           "Brand"),
                (card.number.as_deref(),          "Number"),
            ] {
                if let Some(v) = val { if !v.is_empty() {
                    if self.detail_field == idx {
                        return self.queue_copy(PendingAction::CopyRaw(v.into(), format!("{lbl} copied ✓")), "Copying…");
                    }
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
                if self.detail_field == idx {
                    return self.queue_copy(PendingAction::CopyRaw(v.into(), "CVV copied ✓".into()), "Copying…");
                }
                idx += 1;
            }}
        }

        // Identity
        if let Some(id) = item.identity.as_ref() {
            let full_name = build_full_name(id.title.as_deref(), id.first_name.as_deref(),
                                            id.middle_name.as_deref(), id.last_name.as_deref());
            if !full_name.is_empty() {
                if self.detail_field == idx {
                    return self.queue_copy(PendingAction::CopyRaw(full_name, "Name copied ✓".into()), "Copying…");
                }
                idx += 1;
            }
            for (lbl, val) in identity_fields(id) {
                if let Some(v) = val { if !v.is_empty() {
                    if self.detail_field == idx {
                        return self.queue_copy(PendingAction::CopyRaw(v.to_string(), format!("{lbl} copied ✓")), "Copying…");
                    }
                    idx += 1;
                }}
            }
        }

        // Custom fields
        for field in &item.fields {
            let value = field.value.as_deref().unwrap_or("");
            let label = field.name.as_deref().unwrap_or("Field");
            if self.detail_field == idx {
                return self.queue_copy(PendingAction::CopyRaw(value.into(), format!("{label} copied ✓")), "Copying…");
            }
            idx += 1;
        }

        // Notes
        if let Some(notes) = &item.notes { if !notes.is_empty() {
            if self.detail_field == idx {
                self.queue_copy(PendingAction::CopyRaw(notes.clone(), "Notes copied ✓".into()), "Copying…");
            }
        }}
    }

    // ── Deferred action executors ─────────────────────────────────────────

    pub fn do_copy_username(&mut self) {
        let Some(item) = self.selected_item() else { return };
        let (id, name) = (item.id.clone(), item.name.clone());
        let cmd = format!("bw get username {} --session {}", id, self.session_key_display());
        match self.bw.get_username(&id) {
            Ok(v) => { self.set_action(ActionState::Done("Copied ✓".into())); self.push_cmd(&cmd, true, &format!("username for {name}")); self.write_clipboard(v, "Username copied ✓"); }
            Err(e) => { self.set_action(ActionState::Error("Failed".into())); self.push_cmd(&cmd, false, &e); }
        }
    }

    pub fn do_copy_password(&mut self) {
        let Some(item) = self.selected_item() else { return };
        let (id, name) = (item.id.clone(), item.name.clone());
        let cmd = format!("bw get password {} --session {}", id, self.session_key_display());
        match self.bw.get_password(&id) {
            Ok(v) => { self.set_action(ActionState::Done("Copied ✓".into())); self.push_cmd(&cmd, true, &format!("password for {name} [hidden]")); self.write_clipboard(v, "Password copied ✓"); }
            Err(e) => { self.set_action(ActionState::Error("Failed".into())); self.push_cmd(&cmd, false, &e); }
        }
    }

    pub fn do_copy_raw(&mut self, text: String, msg: String) {
        self.set_action(ActionState::Done("Copied ✓".into()));
        self.push_cmd("clipboard", true, &msg);
        self.write_clipboard(text, &msg);
    }

    pub fn do_copy_totp(&mut self, item_id: String) {
        let cmd = format!("bw get totp {} --session {}", item_id, self.session_key_display());
        match self.bw.get_totp(&item_id) {
            Ok(v) => { self.set_action(ActionState::Done("TOTP copied ✓".into())); self.push_cmd(&cmd, true, "totp [hidden]"); self.write_clipboard(v, "TOTP copied ✓"); }
            Err(e) => { self.set_action(ActionState::Error("Failed".into())); self.push_cmd(&cmd, false, &e); }
        }
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
            Err(e) => { self.set_action(ActionState::Error("Failed".into())); self.push_cmd(&cmd, false, &e); }
        }
    }

    pub fn do_sync_vault(&mut self) {
        let cmd = format!("bw sync --session {}", self.session_key_display());
        match self.bw.sync() {
            Ok(()) => { self.set_action(ActionState::Done("Synced ✓".into())); self.push_cmd(&cmd, true, "vault synced"); self.load_items(); }
            Err(e) => { self.set_action(ActionState::Error("Sync failed".into())); self.push_cmd(&cmd, false, &e); }
        }
    }

    // ── Clipboard ─────────────────────────────────────────────────────────

    fn write_clipboard(&mut self, text: String, success_msg: &str) {
        use std::process::{Command, Stdio};
        use std::io::Write;

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
        } else {
            None
        };

        let Some(args) = args else {
            let msg = "No clipboard tool found (install wl-copy or xclip)";
            self.push_cmd("clipboard", false, msg);
            self.set_status(msg, true);
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
                self.set_status(success_msg, false);
            }
            Err(e) => {
                self.push_cmd(args[0], false, &format!("spawn failed: {e}"));
                self.set_status(&format!("Clipboard error: {e}"), true);
            }
        }
    }

    // ── Command log ───────────────────────────────────────────────────────

    pub fn push_cmd(&mut self, cmd: &str, ok: bool, detail: &str) {
        let redacted = cmd.replace(
            self.bw.session_key.as_deref().unwrap_or("__NO_SESSION__"), "***"
        );
        self.cmd_log.push(CmdEntry { cmd: redacted, ok, detail: detail.to_string() });
        if self.cmd_log.len() > 50 { self.cmd_log.remove(0); }
        self.cmd_log_scroll = 0;
    }

    pub fn cmd_log_scroll_up(&mut self, n: usize)   { let max = self.cmd_log.len().saturating_sub(1); self.cmd_log_scroll = (self.cmd_log_scroll + n).min(max); }
    pub fn cmd_log_scroll_down(&mut self, n: usize) { self.cmd_log_scroll = self.cmd_log_scroll.saturating_sub(n); }

    // ── Action state ──────────────────────────────────────────────────────

    pub fn set_action(&mut self, state: ActionState) { self.action_state = state; self.action_tick = 0; }
    pub fn tick_action(&mut self) { self.action_tick = self.action_tick.wrapping_add(1); }

    // ── Status / errors ───────────────────────────────────────────────────

    pub fn set_status(&mut self, msg: &str, is_error: bool) {
        self.status = Some(StatusMessage { text: msg.to_string(), is_error });
    }
    pub fn clear_status(&mut self) { self.status = None; }

    pub fn set_login_error(&mut self) {
        self.login_error = true;
        self.password_input.clear();
        self.password_cursor = 0;
    }
    pub fn clear_login_error(&mut self) { self.login_error = false; }

    // ── Login field editing ───────────────────────────────────────────────

    pub fn insert_char(&mut self, c: char) {
        match self.active_field {
            LoginField::SaveEmail => {}
            LoginField::Email => {
                let idx = self.byte_offset(&self.email_input, self.email_cursor);
                self.email_input.insert(idx, c);
                self.email_cursor += 1;
                if self.save_email { let e = self.email_input.clone(); config::write(true, Some(&e)); }
            }
            LoginField::Password => {
                let idx = self.byte_offset(&self.password_input, self.password_cursor);
                self.password_input.insert(idx, c);
                self.password_cursor += 1;
            }
        }
    }

    pub fn delete_char_before(&mut self) {
        match self.active_field {
            LoginField::SaveEmail => {}
            LoginField::Email => {
                if self.email_cursor > 0 {
                    let idx = self.byte_offset(&self.email_input, self.email_cursor - 1);
                    self.email_input.remove(idx);
                    self.email_cursor -= 1;
                    if self.save_email { let e = self.email_input.clone(); config::write(true, Some(&e)); }
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

    pub fn delete_char_at(&mut self) {
        match self.active_field {
            LoginField::SaveEmail => {}
            LoginField::Email => {
                if self.email_cursor < self.email_input.chars().count() {
                    let idx = self.byte_offset(&self.email_input, self.email_cursor);
                    self.email_input.remove(idx);
                    if self.save_email { let e = self.email_input.clone(); config::write(true, Some(&e)); }
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
            LoginField::Email    => { if self.email_cursor    > 0 { self.email_cursor    -= 1; } }
            LoginField::Password => { if self.password_cursor > 0 { self.password_cursor -= 1; } }
            LoginField::SaveEmail => {}
        }
    }

    pub fn cursor_right(&mut self) {
        match self.active_field {
            LoginField::Email    => { if self.email_cursor    < self.email_input.chars().count()    { self.email_cursor    += 1; } }
            LoginField::Password => { if self.password_cursor < self.password_input.chars().count() { self.password_cursor += 1; } }
            LoginField::SaveEmail => {}
        }
    }

    pub fn cursor_home(&mut self) {
        match self.active_field {
            LoginField::Email     => self.email_cursor    = 0,
            LoginField::Password  => self.password_cursor = 0,
            LoginField::SaveEmail => {}
        }
    }

    pub fn cursor_end(&mut self) {
        match self.active_field {
            LoginField::Email     => self.email_cursor    = self.email_input.chars().count(),
            LoginField::Password  => self.password_cursor = self.password_input.chars().count(),
            LoginField::SaveEmail => {}
        }
    }

    pub fn toggle_save_email(&mut self) {
        self.save_email = !self.save_email;
        if self.save_email { let e = self.email_input.clone(); config::write(true, Some(&e)); }
        else { config::write(false, None); }
    }

    // ── Detail field count ────────────────────────────────────────────────

    /// Total field count for currently selected item — must match build_detail_fields().
    pub fn detail_field_count(&self) -> usize {
        let Some(item) = self.selected_item() else { return 0 };
        let mut n = 2; // Name + Type

        if let Some(l) = &item.login {
            if l.username.is_some() { n += 1; }
            n += 1; // password
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

    // ── Helpers ───────────────────────────────────────────────────────────

    fn byte_offset(&self, s: &str, char_idx: usize) -> usize {
        s.char_indices().nth(char_idx).map(|(b, _)| b).unwrap_or(s.len())
    }

    fn session_key_display(&self) -> &str {
        self.bw.session_key.as_deref().unwrap_or("***")
    }
}

// ── Identity helpers (shared between ui.rs and app.rs) ───────────────────

pub fn build_full_name(title: Option<&str>, first: Option<&str>, middle: Option<&str>, last: Option<&str>) -> String {
    [title, first, middle, last]
        .iter()
        .filter_map(|s| s.filter(|x| !x.is_empty()))
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn identity_fields(id: &crate::bw::IdentityData) -> Vec<(&'static str, &Option<String>)> {
    vec![
        ("Email",     &id.email),
        ("Phone",     &id.phone),
        ("Company",   &id.company),
        ("Address",   &id.address1),
        ("Address 2", &id.address2),
        ("City",      &id.city),
        ("State",     &id.state),
        ("ZIP",       &id.postal_code),
        ("Country",   &id.country),
        ("SSN",       &id.ssn),
        ("Passport",  &id.passport),
        ("License",   &id.license),
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

    pub fn ensure_dir() { let _ = fs::create_dir_all(config_path()); }

    #[derive(Default)]
    pub struct Config { pub save_email: bool, pub email: Option<String> }

    pub fn read() -> Config {
        ensure_dir();
        let mut cfg = Config::default();
        let Ok(text) = fs::read_to_string(config_file()) else { return cfg };
        for line in text.lines() {
            let line = line.trim();
            if let Some(v) = line.strip_prefix("save_email = ") { cfg.save_email = v.trim() == "true"; }
            else if let Some(v) = line.strip_prefix("email = ") {
                let v = v.trim().trim_matches('"').to_string();
                if !v.is_empty() { cfg.email = Some(v); }
            }
        }
        cfg
    }

    pub fn write(save_email: bool, email: Option<&str>) {
        ensure_dir();
        let existing = fs::read_to_string(config_file()).unwrap_or_default();
        let mut preserved: Vec<String> = existing.lines()
            .filter(|l| { let t = l.trim(); !t.starts_with("save_email =") && !t.starts_with("email =") })
            .map(|l| l.to_string())
            .collect();
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
    let mut score = 0i32;
    if name.contains(query)       { score += 100; if name.starts_with(query) { score += 20; } }
    else if is_subseq(query, &name) { score += 50; }

    if let Some(login) = &item.login {
        if let Some(u) = &login.username {
            let u = u.to_lowercase();
            if u.contains(query)      { score += 30; }
            else if is_subseq(query, &u) { score += 10; }
        }
        if let Some(uris) = &login.uris {
            for u in uris { if let Some(uri) = &u.uri { if uri.to_lowercase().contains(query) { score += 10; break; } } }
        }
    }
    if let Some(notes) = &item.notes { if notes.to_lowercase().contains(query) { score += 5; } }
    score
}

fn is_subseq(needle: &str, haystack: &str) -> bool {
    let mut it = haystack.chars();
    needle.chars().all(|c| it.find(|&h| h == c).is_some())
}
