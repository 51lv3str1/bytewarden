/// events.rs — Keyboard & mouse event handling
///
/// Shared navigation helpers at the bottom eliminate repeated
/// Tab-wrap / clamp / text-input logic across all screens.

use crate::app::{App, EditField, Focus, LoginField, Screen, ITEM_FILTERS, CREATE_ITEM_TYPES, ItemFilter};
use crate::app::config;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers,
                        MouseEvent, MouseEventKind, MouseButton};

// ── Public entry point ────────────────────────────────────────────────────

/// Returns true if the key was pressed with Alt (either Left or Right).
/// Right Alt (AltGr) on Linux = ALT | CONTROL in crossterm.
/// We accept any modifier set containing ALT.
#[inline]
fn is_alt(key: &KeyEvent) -> bool {
    key.modifiers.contains(KeyModifiers::ALT)
}

/// Dispatch a pre-read crossterm event.
pub fn handle_events(app: &mut App, ev: Event) {
    match ev {
        Event::Key(key) => {
            if key.kind != KeyEventKind::Press { return; }
            if key.code == KeyCode::Char('c') && key.modifiers == KeyModifiers::CONTROL {
                app.should_quit = true;
                return;
            }
            match app.screen.clone() {
                Screen::Splash        => {}
                Screen::Login         => handle_login(app, key),
                Screen::Vault         => handle_vault(app, key),
                Screen::Detail        => handle_detail(app, key),
                Screen::Help          => { app.go_back(); }
                Screen::Create        => handle_create(app, key),
                Screen::ConfirmDelete => handle_confirm_delete(app, key),
            }
        }
        Event::Mouse(mouse) => handle_mouse(app, mouse),
        _ => {}
    }
}

// ── Login ─────────────────────────────────────────────────────────────────

fn handle_login(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Tab => {
            app.active_field = match app.active_field {
                LoginField::Email     => LoginField::Password,
                LoginField::Password  => if app.otp_required { LoginField::Otp } else { LoginField::SaveEmail },
                LoginField::Otp       => LoginField::SaveEmail,
                LoginField::SaveEmail => LoginField::AutoLock,
                LoginField::AutoLock  => LoginField::Email,
            };
        }
        KeyCode::Char(' ') if app.active_field == LoginField::SaveEmail => {
            app.toggle_save_email();
        }
        KeyCode::Char(' ') if app.active_field == LoginField::AutoLock => {
            app.auto_lock = !app.auto_lock;
            config::write_auto_lock(app.auto_lock);
        }
        KeyCode::Enter     => app.attempt_login(),
        KeyCode::F(2)      => app.login_password_visible = !app.login_password_visible,
        KeyCode::Left      => app.cursor_left(),
        KeyCode::Right     => app.cursor_right(),
        KeyCode::Home      => app.cursor_home(),
        KeyCode::End       => app.cursor_end(),
        KeyCode::Delete    => { app.clear_login_error(); app.delete_char_at(); }
        KeyCode::Backspace => { app.clear_login_error(); app.delete_char_before(); }
        KeyCode::Char(c)   => {
            if app.active_field != LoginField::SaveEmail
            && app.active_field != LoginField::AutoLock {
                app.clear_login_error();
                app.insert_char(c);
            }
        }
        KeyCode::BackTab => {
            app.active_field = match app.active_field {
                LoginField::AutoLock  => LoginField::SaveEmail,
                LoginField::SaveEmail => if app.otp_required { LoginField::Otp } else { LoginField::Password },
                LoginField::Otp       => LoginField::Password,
                LoginField::Password  => LoginField::Email,
                LoginField::Email     => LoginField::AutoLock,
            };
        }
        _ => {}
    }
}

// ── Vault ─────────────────────────────────────────────────────────────────

fn handle_vault(app: &mut App, key: KeyEvent) {

    // Number keys 0-4 jump to panels — disabled in Search to allow text input
    if key.modifiers == KeyModifiers::NONE && app.focus != Focus::Search {
        match key.code {
            KeyCode::Char('0') => { app.focus_panel(0); return; }
            KeyCode::Char('1') => { app.focus_panel(1); return; }
            KeyCode::Char('2') => { app.focus_panel(2); return; }
            KeyCode::Char('3') => { app.focus_panel(3); return; }
            KeyCode::Char('4') => { app.focus_panel(4); return; }
            _ => {}
        }
    }

    // Alt+S: sync from any panel, never conflicts with text input
    if key.code == KeyCode::Char('s') && is_alt(&key)
        && !app.is_trash_view() {
        app.sync_vault();
        return;
    }

    if key.code == KeyCode::Char('l') && is_alt(&key) {
        app.lock_vault();
        return;
    }

    if key.code == KeyCode::Char('/') && key.modifiers == KeyModifiers::NONE {
        app.focus = Focus::Search;
        return;
    }

    match app.focus.clone() {
        Focus::Status | Focus::Vaults => match key.code {
            KeyCode::Tab | KeyCode::Esc  => app.cycle_focus(),
            KeyCode::Char('?')           => app.screen = Screen::Help,
            _ => {}
        },

        Focus::Items => match key.code {
            KeyCode::Char('j') | KeyCode::Down  | KeyCode::PageDown => app.filter_move_down(),
            KeyCode::Char('k') | KeyCode::Up    | KeyCode::PageUp   => app.filter_move_up(),
            KeyCode::Enter                                            => app.apply_filter(),
            KeyCode::Tab | KeyCode::Esc                              => app.cycle_focus(),
            KeyCode::Char('?')                                        => app.screen = Screen::Help,
            _ => {}
        },

        Focus::Search => match key.code {
            KeyCode::Esc       => app.clear_search(),
            KeyCode::Tab       => app.cycle_focus(),
            KeyCode::Char('j') | KeyCode::Down  => app.move_down(),
            KeyCode::Char('k') | KeyCode::Up    => app.move_up(),
            KeyCode::PageDown                    => app.move_down_page(),
            KeyCode::PageUp                      => app.move_up_page(),
            KeyCode::Enter => {
                if !app.filtered_items().is_empty() {
                    app.screen = Screen::Detail;
                    app.show_password = false;
                }
            }
            KeyCode::Backspace => { app.search_query.pop(); app.perform_search(); }
            _ if is_alt(&key)  => handle_alt_shortcuts(app, key),
            // Plain char — only feed into search query when no modifiers active
            KeyCode::Char(c) if key.modifiers == KeyModifiers::NONE => {
                app.search_query.push(c); app.perform_search();
            }
            _ => {}
        },

        Focus::List => match key.code {
            KeyCode::Char('j') | KeyCode::Down  => app.move_down(),
            KeyCode::Char('k') | KeyCode::Up    => app.move_up(),
            KeyCode::PageDown                    => app.move_down_page(),
            KeyCode::PageUp                      => app.move_up_page(),
            KeyCode::Enter | KeyCode::Char('l') => app.go_to_detail(),
            KeyCode::Tab                         => app.cycle_focus(),
            KeyCode::Char('?')                   => app.screen = Screen::Help,
            _ if is_alt(&key)                    => handle_alt_shortcuts(app, key),
            _ => {}
        },

        Focus::CmdLog => match key.code {
            KeyCode::Char('j') | KeyCode::Down  => app.cmd_log_scroll_up(1),
            KeyCode::Char('k') | KeyCode::Up    => app.cmd_log_scroll_down(1),
            KeyCode::PageDown                    => app.cmd_log_scroll_down(5),
            KeyCode::PageUp                      => app.cmd_log_scroll_up(5),
            KeyCode::Tab | KeyCode::Esc          => app.cycle_focus(),
            KeyCode::Char('?')                   => app.screen = Screen::Help,
            _ => {}
        },
    }
}

// ── Detail ────────────────────────────────────────────────────────────────

fn handle_detail(app: &mut App, key: KeyEvent) {

    if app.edit_mode {
        let n = app.edit_fields.len();
        match key.code {
            KeyCode::Esc     => { app.edit_mode = false; }
            KeyCode::Enter   => app.queue_save_edit(),
            KeyCode::Tab     => nav_wrap(&mut app.edit_field_idx, n, 1),
            KeyCode::BackTab => nav_wrap(&mut app.edit_field_idx, n, -1),
            KeyCode::Down    => nav_clamp(&mut app.edit_field_idx, n, 1),
            KeyCode::Up      => nav_clamp(&mut app.edit_field_idx, n, -1),
            KeyCode::F(2)    => app.edit_toggle_reveal(),
            _                => text_input(app.edit_field_mut(), key),
        }
        return;
    }

    let n = app.detail_field_count();
    match key.code {
        KeyCode::Esc | KeyCode::Char('h') => {
            app.show_password = false;
            app.detail_field = 0;
            app.go_back();
        }
        KeyCode::Tab => {
            app.show_password = false;
            nav_wrap(&mut app.detail_field, n, 1);
        }
        KeyCode::BackTab => {
            app.show_password = false;
            nav_wrap(&mut app.detail_field, n, -1);
        }
        KeyCode::Char('j') | KeyCode::Down | KeyCode::PageDown => {
            app.show_password = false;
            nav_clamp(&mut app.detail_field, n, 1);
        }
        KeyCode::Char('k') | KeyCode::Up | KeyCode::PageUp => {
            app.show_password = false;
            nav_clamp(&mut app.detail_field, n, -1);
        }
        KeyCode::F(2)      => app.show_password = !app.show_password,
        KeyCode::Char('c') if is_alt(&key) => app.copy_selected_field(),
        KeyCode::Char('e') if is_alt(&key) && !app.is_trash_view() => app.enter_edit_mode(),
        KeyCode::Char('r') if is_alt(&key) &&  app.is_trash_view() => app.queue_restore_item(),
        KeyCode::Char('d') if is_alt(&key) => app.open_confirm_delete(),
        _ => {}
    }
}

// ── Create ────────────────────────────────────────────────────────────────

fn handle_create(app: &mut App, key: KeyEvent) {
    if app.create_choosing_type {
        let n = CREATE_ITEM_TYPES.len();
        match key.code {
            KeyCode::Esc                       => app.go_back(),
            KeyCode::Enter                     => app.create_select_type(),
            KeyCode::Tab                       => nav_wrap(&mut app.create_type_idx, n, 1),
            KeyCode::BackTab                   => nav_wrap(&mut app.create_type_idx, n, -1),
            KeyCode::Char('j') | KeyCode::Down => nav_clamp(&mut app.create_type_idx, n, 1),
            KeyCode::Char('k') | KeyCode::Up   => nav_clamp(&mut app.create_type_idx, n, -1),
            _ => {}
        }
    } else {
        let n = app.create_fields.len();
        match key.code {
            KeyCode::Esc     => app.go_back(),
            KeyCode::Enter   => app.queue_create_item(),
            KeyCode::Tab     => nav_wrap(&mut app.create_field_idx, n, 1),
            KeyCode::BackTab => nav_wrap(&mut app.create_field_idx, n, -1),
            KeyCode::Down    => nav_clamp(&mut app.create_field_idx, n, 1),
            KeyCode::Up      => nav_clamp(&mut app.create_field_idx, n, -1),
            KeyCode::F(2)    => {
                if let Some(f) = app.create_fields.get_mut(app.create_field_idx) {
                    if f.hidden { f.revealed = !f.revealed; }
                }
            }
            _                => text_input(app.create_field_mut(), key),
        }
    }
}

// ── Confirm delete ────────────────────────────────────────────────────────

fn handle_confirm_delete(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('n') => { app.screen = Screen::Vault; }
        // In trash view Enter = permanent (already trashed); in vault Enter = trash
        KeyCode::Enter => app.queue_delete_item(app.is_trash_view()),
        KeyCode::Char('D') if !app.is_trash_view() => app.queue_delete_item(true),
        _ => {}
    }
}

// ── Mouse ─────────────────────────────────────────────────────────────────

fn handle_mouse(app: &mut App, mouse: MouseEvent) {
    let col = mouse.column;
    let row = mouse.row;

    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            app.last_click = Some((col, row));
            match app.screen {
                Screen::Login  => mouse_login(app, col, row),
                Screen::Vault  => mouse_vault(app, col, row),
                Screen::Detail => mouse_detail(app, col, row),
                _ => {}
            }
        }
        MouseEventKind::ScrollDown => mouse_scroll(app, col, row, 1),
        MouseEventKind::ScrollUp   => mouse_scroll(app, col, row, -1),
        _ => {}
    }
}

fn mouse_login(app: &mut App, col: u16, row: u16) {
    let Some(form) = app.mouse_areas.login else { return };
    if col < form.x || col >= form.x + form.width
    || row < form.y || row >= form.y + form.height { return; }

    let inner_row = row.saturating_sub(form.y + 1);
    if inner_row < 4 {
        app.active_field = LoginField::Email;
    } else if inner_row < 8 {
        app.active_field = LoginField::Password;
    } else if inner_row < 10 {
        app.active_field = LoginField::SaveEmail;
        app.toggle_save_email();
    } else {
        app.active_field = LoginField::AutoLock;
        app.auto_lock = !app.auto_lock;
        config::write_auto_lock(app.auto_lock);
    }
}

fn mouse_vault(app: &mut App, col: u16, row: u16) {
    let Some(focus) = app.mouse_areas.focus_for(col, row) else { return };
    app.focus = focus.clone();

    if focus == Focus::List {
        if let Some(row_idx) = app.mouse_areas.list_row(row) {
            let visible_idx = app.scroll_offset + row_idx;
            if visible_idx < app.filtered_items().len() {
                if app.selected_index == visible_idx {
                    app.go_to_detail();
                } else {
                    app.selected_index = visible_idx;
                }
            }
        }
    }

    if focus == Focus::Items {
        if let Some(row_idx) = app.mouse_areas.items_row(row) {
            // Separator is injected before Trash (last filter), so rows after SSH Key are +1
            let trash_display_row = ITEM_FILTERS.len(); // separator row is len-1, trash is len
            let filter_idx = if row_idx >= trash_display_row {
                ITEM_FILTERS.len() - 1 // Trash
            } else if row_idx == ITEM_FILTERS.len() - 1 {
                return; // clicked the separator — do nothing
            } else {
                row_idx
            };
            if filter_idx < ITEM_FILTERS.len() {
                app.filter_selected = filter_idx;
                app.active_filter = ITEM_FILTERS[filter_idx].clone();
                app.selected_index = 0;
                app.scroll_offset = 0;
                if app.active_filter == ItemFilter::Trash {
                    app.pending_action = crate::app::PendingAction::LoadTrash;
                }
            }
        }
    }
}

fn mouse_detail(app: &mut App, col: u16, row: u16) {
    if row < 2 {
        app.show_password = false;
        app.detail_field = 0;
        app.go_back();
        return;
    }
    let Some(area) = app.mouse_areas.detail else { return };
    if col < area.x || col >= area.x + area.width
    || row < area.y || row >= area.y + area.height { return; }

    let field_idx = (row.saturating_sub(area.y) / 4) as usize;
    let total = app.detail_field_count();
    if field_idx < total {
        if field_idx == app.detail_field {
            app.show_password = !app.show_password;
        } else {
            app.show_password = false;
            app.detail_field = field_idx;
        }
    }
}

fn mouse_scroll(app: &mut App, col: u16, row: u16, dir: i8) {
    match app.screen {
        Screen::Vault => match app.mouse_areas.focus_for(col, row) {
            Some(Focus::Items)  => if dir > 0 { app.filter_move_down() } else { app.filter_move_up() },
            Some(Focus::CmdLog) => if dir > 0 { app.cmd_log_scroll_up(1) } else { app.cmd_log_scroll_down(1) },
            _                   => if dir > 0 { app.move_down() } else { app.move_up() },
        },
        Screen::Detail => {
            let total = app.detail_field_count();
            if dir > 0 {
                if app.detail_field + 1 < total { app.show_password = false; app.detail_field += 1; }
            } else {
                if app.detail_field > 0 { app.show_password = false; app.detail_field -= 1; }
            }
        }
        _ => {}
    }
}

// ── Navigation helpers ────────────────────────────────────────────────────

/// Wrapping navigation — Tab/BackTab. Wraps from last→first and first→last.
fn nav_wrap(idx: &mut usize, len: usize, dir: i8) {
    if len == 0 { return; }
    if dir > 0 { *idx = (*idx + 1) % len; }
    else       { *idx = (*idx + len - 1) % len; }
}

/// Clamping navigation — j/k/arrows. Stops at 0 and len-1.
fn nav_clamp(idx: &mut usize, len: usize, dir: i8) {
    if len == 0 { return; }
    if dir > 0 { if *idx + 1 < len { *idx += 1; } }
    else       { if *idx > 0 { *idx -= 1; } }
}

// ── Shared helpers ────────────────────────────────────────────────────────

/// Alt+key vault actions shared by Search and List panels.
fn handle_alt_shortcuts(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('q')                               => app.lock_vault(),
        KeyCode::Char('d')                               => app.open_confirm_delete(),
        KeyCode::Char('r') if  app.is_trash_view()       => app.queue_restore_item(),
        KeyCode::Char('u') if !app.is_trash_view()       => app.copy_username_to_clipboard(),
        KeyCode::Char('c') if !app.is_trash_view()       => app.copy_password_to_clipboard(),
        KeyCode::Char('f') if !app.is_trash_view()       => app.toggle_favorite(),
        KeyCode::Char('n') if !app.is_trash_view()       => app.open_create(),
        _ => {}
    }
}

/// Cursor + typing keys for a form field (edit and create share this).
fn text_input(field: Option<&mut EditField>, key: KeyEvent) {
    let Some(f) = field else { return };
    match key.code {
        KeyCode::Left      => f.cursor_left(),
        KeyCode::Right     => f.cursor_right(),
        KeyCode::Home      => f.cursor_home(),
        KeyCode::End       => f.cursor_end(),
        KeyCode::Backspace => f.delete_before(),
        KeyCode::Delete    => f.delete_at(),
        KeyCode::Char(c)   => f.insert(c),
        _ => {}
    }
}
