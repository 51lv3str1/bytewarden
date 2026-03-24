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
            Screen::Search => {}
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

    // ── F1-F5: jump to panels by index shown on screen ────────────────────
    // F-keys work reliably in all terminals, no modifier needed,
    // and never conflict with text input.
    //   [5]-Status  → F5      [0]-Search → F1 (or /)
    //   [1]-Vaults  → F1+1    mapped as:
    // Screen labels:  [5] [0] [1] [2] [3] [4]
    // F-key mapping:  F5  F1  F2  F3  F4   (F5=status shown at top)
    // Use the NUMBER shown in brackets: 0=F1 workaround, but simpler:
    // Map directly: F1=[1]-Vaults, F2=[2]-Items, F3=[3]-Vault,
    //               F4=[4]-CmdLog, F5=[5]-Status, /=[0]-Search
    match key.code {
        KeyCode::F(1) => { app.focus_panel(1); return; }
        KeyCode::F(2) => { app.focus_panel(2); return; }
        KeyCode::F(3) => { app.focus_panel(3); return; }
        KeyCode::F(4) => { app.focus_panel(4); return; }
        KeyCode::F(5) => { app.focus_panel(5); return; }
        _ => {}
    }

    // '/' always focuses search (works from any pane)
    if key.modifiers == KeyModifiers::NONE {
        if let KeyCode::Char('/') = key.code {
            app.focus_panel(0);
            return;
        }
    }

    match app.focus.clone() {
        // ── [5]-Status ────────────────────────────────────────────────────
        Focus::Status => match key.code {
            KeyCode::Tab | KeyCode::Esc => app.cycle_focus(),
            _ => {}
        },

        // ── [1]-Vaults ────────────────────────────────────────────────────
        Focus::Vaults => match key.code {
            KeyCode::Tab | KeyCode::Esc => app.cycle_focus(),
            _ => {}
        },

        // ── [2]-Items ─────────────────────────────────────────────────────
        Focus::Items => match key.code {
            KeyCode::Char('j') | KeyCode::Down  => app.filter_move_down(),
            KeyCode::Char('k') | KeyCode::Up    => app.filter_move_up(),
            KeyCode::PageDown                    => app.filter_move_down(),
            KeyCode::PageUp                      => app.filter_move_up(),
            KeyCode::Enter                       => app.apply_filter(),
            KeyCode::Tab | KeyCode::Esc          => app.cycle_focus(),
            _ => {}
        },

        // ── [0]-Search ────────────────────────────────────────────────────
        Focus::Search => match key.code {
            KeyCode::Esc       => app.clear_search(),
            KeyCode::Tab       => app.cycle_focus(),
            KeyCode::Char('j') | KeyCode::Down => app.move_down(),
            KeyCode::Char('k') | KeyCode::Up   => app.move_up(),
            KeyCode::PageDown                   => app.move_down_page(),
            KeyCode::PageUp                     => app.move_up_page(),
            KeyCode::Enter => {
                if !app.filtered_items().is_empty() {
                    app.screen = Screen::Detail;
                    app.show_password = false;
                }
            }
            KeyCode::Backspace => { app.search_query.pop(); app.perform_search(); }
            KeyCode::Char(c)   => { app.search_query.push(c); app.perform_search(); }
            _ => {}
        },

        // ── [3]-Vault (main list) ─────────────────────────────────────────
        Focus::List => match key.code {
            KeyCode::Char('j') | KeyCode::Down  => app.move_down(),
            KeyCode::Char('k') | KeyCode::Up    => app.move_up(),
            KeyCode::PageDown                    => app.move_down_page(),
            KeyCode::PageUp                      => app.move_up_page(),
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

        // ── [4]-Command Log ───────────────────────────────────────────────
        Focus::CmdLog => match key.code {
            KeyCode::Char('j') | KeyCode::Down  => app.cmd_log_scroll_up(1),
            KeyCode::Char('k') | KeyCode::Up    => app.cmd_log_scroll_down(1),
            KeyCode::PageDown                    => app.cmd_log_scroll_down(5),
            KeyCode::PageUp                      => app.cmd_log_scroll_up(5),
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
