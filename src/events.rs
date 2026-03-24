/// events.rs — Keyboard event handling

use crate::app::{App, Focus, LoginField, Screen};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

pub fn handle_events(app: &mut App) -> std::io::Result<()> {
    if let Event::Key(key) = event::read()? {
        if key.kind != KeyEventKind::Press {
            return Ok(());
        }
        if key.code == KeyCode::Char('c') && key.modifiers == KeyModifiers::CONTROL {
            app.should_quit = true;
            return Ok(());
        }
        match app.screen.clone() {
            Screen::Login  => handle_login(app, key),
            Screen::Vault  => handle_vault(app, key),
            Screen::Detail => handle_detail(app, key),
            Screen::Search => {} // Search is now inline in Vault — should never reach here
            Screen::Help   => { app.go_back(); }
        }
    }
    Ok(())
}

fn handle_login(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Tab => {
            app.active_field = match app.active_field {
                LoginField::Email     => LoginField::Password,
                LoginField::Password  => LoginField::SaveEmail,
                LoginField::SaveEmail => LoginField::Email,
            };
        }
        KeyCode::Char(' ') if app.active_field == LoginField::SaveEmail => {
            app.toggle_save_email();
        }
        KeyCode::Enter     => app.attempt_login(),
        KeyCode::Left      => app.cursor_left(),
        KeyCode::Right     => app.cursor_right(),
        KeyCode::Home      => app.cursor_home(),
        KeyCode::End       => app.cursor_end(),
        KeyCode::Delete    => { app.clear_login_error(); app.delete_char_at(); }
        KeyCode::Backspace => { app.clear_login_error(); app.delete_char_before(); }
        KeyCode::Char(c)   => {
            if app.active_field != LoginField::SaveEmail {
                app.clear_login_error();
                app.insert_char(c);
            }
        }
        _ => {}
    }
}

fn handle_vault(app: &mut App, key: KeyEvent) {
    app.clear_status();

    // ── Global keys — work regardless of focused panel ────────────────────
    match key.code {
        // PgUp/PgDn scroll the main list from anywhere
        KeyCode::PageUp   => { app.move_up_page(); return; }
        KeyCode::PageDown => { app.move_down_page(); return; }
        _ => {}
    }

    // Number keys and / jump directly to panels (like lazygit)
    if let KeyCode::Char(c) = key.code {
        if key.modifiers == KeyModifiers::CONTROL {
            match c {
                '0' => { app.focus_panel(0); return; }
                '1' => { app.focus_panel(1); return; }
                '2' => { app.focus_panel(2); return; }
                '3' => { app.focus_panel(3); return; }
                '4' => { app.focus_panel(4); return; }
                '5' => { app.focus_panel(5); return; }
                _ => {}
            }
        }
        // '/' without modifier focuses search (doesn't conflict — search
        // bar handles its own chars once focused)
        if key.modifiers == KeyModifiers::NONE && c == '/' {
            app.focus_panel(0); return;
        }
    }

    match app.focus.clone() {
        // ── [5] Status pane ───────────────────────────────────────────────
        Focus::Status => match key.code {
            KeyCode::Tab | KeyCode::Esc => app.cycle_focus(),
            _ => {}
        },

        // ── [1] Vaults panel ─────────────────────────────────────────────
        Focus::Vaults => match key.code {
            KeyCode::Tab | KeyCode::Esc => app.cycle_focus(),
            _ => {}
        },

        // ── [2] Items filter panel ────────────────────────────────────────
        Focus::Items => match key.code {
            KeyCode::Char('j') | KeyCode::Down  => app.filter_move_down(),
            KeyCode::Char('k') | KeyCode::Up    => app.filter_move_up(),
            KeyCode::Enter                       => app.apply_filter(),
            KeyCode::Tab | KeyCode::Esc          => app.cycle_focus(),
            _ => {}
        },

        // ── [/] Search bar ────────────────────────────────────────────────
        Focus::Search => match key.code {
            KeyCode::Esc => app.clear_search(),
            KeyCode::Tab => app.cycle_focus(),
            // j/k navigate results while in search
            KeyCode::Char('j') | KeyCode::Down  => app.move_down(),
            KeyCode::Char('k') | KeyCode::Up    => app.move_up(),
            KeyCode::Enter => {
                // Open selected result in detail
                if !app.filtered_items().is_empty() {
                    app.screen = Screen::Detail;
                    app.show_password = false;
                }
            }
            KeyCode::Backspace => {
                app.search_query.pop();
                app.perform_search();
            }
            KeyCode::Char(c) => {
                app.search_query.push(c);
                app.perform_search();
            }
            _ => {}
        },

        // ── [3] Main list ─────────────────────────────────────────────────
        Focus::List => match key.code {
            KeyCode::Char('j') | KeyCode::Down  => app.move_down(),
            KeyCode::Char('k') | KeyCode::Up    => app.move_up(),
            KeyCode::Enter | KeyCode::Char('l') => app.go_to_detail(),
            KeyCode::Tab                         => app.cycle_focus(),
            KeyCode::Char('u')                   => app.copy_username_to_clipboard(),
            KeyCode::Char('c')                   => app.copy_password_to_clipboard(),
            KeyCode::Char('f')                   => app.toggle_favorite(),
            KeyCode::Char('s')                   => app.sync_vault(),
            KeyCode::Char('?')                   => app.screen = Screen::Help,
            KeyCode::Char('q')                   => {
                app.bw.lock();
                app.screen = Screen::Login;
                app.items.clear();
                app.password_input.clear();
                app.set_status("Session closed", false);
            }
            _ => {}
        },

        // ── [4] Command log ───────────────────────────────────────────────
        Focus::CmdLog => match key.code {
            KeyCode::Char('j') | KeyCode::Down  => app.cmd_log_scroll_up(1),
            KeyCode::Char('k') | KeyCode::Up    => app.cmd_log_scroll_down(1),
            KeyCode::PageUp                      => app.cmd_log_scroll_up(5),
            KeyCode::PageDown                    => app.cmd_log_scroll_down(5),
            KeyCode::Tab | KeyCode::Esc          => app.cycle_focus(),
            _ => {}
        },
    }
}

fn handle_detail(app: &mut App, key: KeyEvent) {
    app.clear_status();
    match key.code {
        KeyCode::Esc | KeyCode::Char('h') => app.go_back(),
        KeyCode::Char('p')                => app.show_password = !app.show_password,
        KeyCode::Char('c')                => app.copy_password_to_clipboard(),
        KeyCode::Char('j') | KeyCode::Down  => { app.show_password = false; app.move_down(); }
        KeyCode::Char('k') | KeyCode::Up    => { app.show_password = false; app.move_up(); }
        KeyCode::PageDown                   => { app.show_password = false; app.move_down_page(); }
        KeyCode::PageUp                     => { app.show_password = false; app.move_up_page(); }
        _ => {}
    }
}
