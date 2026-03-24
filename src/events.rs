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
            Screen::Search => handle_search(app, key),
            Screen::Help   => { app.go_back(); }
        }
    }
    Ok(())
}

fn handle_login(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Tab => {
            app.active_field = match app.active_field {
                LoginField::Email    => LoginField::Password,
                LoginField::Password => LoginField::Email,
            };
        }
        KeyCode::Enter => app.attempt_login(),
        KeyCode::Backspace => {
            app.clear_login_error();
            match app.active_field {
                LoginField::Email    => { app.email_input.pop(); }
                LoginField::Password => { app.password_input.pop(); }
            }
        }
        KeyCode::Char(c) => {
            app.clear_login_error();
            match app.active_field {
                LoginField::Email    => app.email_input.push(c),
                LoginField::Password => app.password_input.push(c),
            }
        }
        _ => {}
    }
}

fn handle_vault(app: &mut App, key: KeyEvent) {
    app.clear_status();

    match app.focus.clone() {
        // ── Items filter panel focused ────────────────────────────────────
        Focus::Items => match key.code {
            KeyCode::Char('j') | KeyCode::Down  => app.filter_move_down(),
            KeyCode::Char('k') | KeyCode::Up    => app.filter_move_up(),
            KeyCode::Enter                       => app.apply_filter(),
            KeyCode::Tab | KeyCode::Esc          => app.cycle_focus(),
            _ => {}
        },

        // ── Main list focused ─────────────────────────────────────────────
        Focus::List | Focus::Vaults => match key.code {
            KeyCode::Char('j') | KeyCode::Down  => app.move_down(),
            KeyCode::Char('k') | KeyCode::Up    => app.move_up(),
            KeyCode::Enter | KeyCode::Char('l') => app.go_to_detail(),
            KeyCode::Tab                         => app.cycle_focus(),
            KeyCode::Char('/')                   => app.go_to_search(),
            KeyCode::Char('c')                   => app.copy_password_to_clipboard(),
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
    }
}

fn handle_detail(app: &mut App, key: KeyEvent) {
    app.clear_status();
    match key.code {
        KeyCode::Esc | KeyCode::Char('h') => app.go_back(),
        KeyCode::Char('p')                => app.show_password = !app.show_password,
        KeyCode::Char('c')                => app.copy_password_to_clipboard(),
        KeyCode::Char('j') | KeyCode::Down => { app.show_password = false; app.move_down(); }
        KeyCode::Char('k') | KeyCode::Up   => { app.show_password = false; app.move_up(); }
        _ => {}
    }
}

fn handle_search(app: &mut App, key: KeyEvent) {
    app.clear_status();
    match key.code {
        KeyCode::Esc => app.go_back(),
        KeyCode::Enter => {
            if !app.search_results.is_empty() {
                app.items = app.search_results.clone();
                app.screen = Screen::Detail;
                app.show_password = false;
            }
        }
        KeyCode::Char('j') | KeyCode::Down => app.move_down(),
        KeyCode::Char('k') | KeyCode::Up   => app.move_up(),
        KeyCode::Backspace => { app.search_query.pop(); app.perform_search(); }
        KeyCode::Char(c)   => { app.search_query.push(c); app.perform_search(); }
        _ => {}
    }
}