/// events.rs — Keyboard event handling via crossterm
///
/// Analogy: this is the app's event listener layer.
/// In TypeScript: document.addEventListener('keydown', handler)
/// Here, crossterm::event::read() blocks until a key is pressed —
/// like an awaited Promise that resolves on each keystroke.

use crate::app::{App, LoginField, Screen};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

/// Reads one keyboard event and mutates app state accordingly.
/// Dispatches to a per-screen handler — like a router switch statement.
pub fn handle_events(app: &mut App) -> std::io::Result<()> {
    if let Event::Key(key) = event::read()? {
        // Only handle KeyPress — ignore KeyRelease and KeyRepeat
        if key.kind != KeyEventKind::Press {
            return Ok(());
        }

        // Ctrl+C always quits, regardless of screen
        if is_quit(key) {
            app.should_quit = true;
            return Ok(());
        }

        // Dispatch to the handler for the active screen.
        // Like a Redux action dispatcher or Scala pattern match.
        match app.screen.clone() {
            Screen::Login  => handle_login(app, key),
            Screen::Vault  => handle_vault(app, key),
            Screen::Detail => handle_detail(app, key),
            Screen::Search => handle_search(app, key),
            Screen::Help   => handle_help(app, key),
        }
    }
    Ok(())
}

fn is_quit(key: KeyEvent) -> bool {
    key.code == KeyCode::Char('c') && key.modifiers == KeyModifiers::CONTROL
}

// ── Per-screen handlers ────────────────────────────────────────────────────

fn handle_login(app: &mut App, key: KeyEvent) {
    match key.code {
        // Tab toggles focus between email and password fields
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
        },
        KeyCode::Char(c) => {
            app.clear_login_error();
            match app.active_field {
                LoginField::Email    => app.email_input.push(c),
                LoginField::Password => app.password_input.push(c),
            }
        },
        _ => {}
    }
}

fn handle_vault(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down  => app.move_down(),
        KeyCode::Char('k') | KeyCode::Up    => app.move_up(),
        KeyCode::Enter | KeyCode::Char('l') => app.go_to_detail(),
        KeyCode::Char('/')                  => app.go_to_search(),
        KeyCode::Char('c')                  => app.copy_password_to_clipboard(),
        KeyCode::Char('s')                  => app.sync_vault(),
        KeyCode::Char('?')                  => app.screen = Screen::Help,
        KeyCode::Char('q')                  => {
            app.bw.lock();
            app.screen = Screen::Login;
            app.items.clear();
            app.password_input.clear();
            app.set_status("Session closed", false);
        }
        _ => {}
    }
}

fn handle_detail(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('h') => app.go_back(),
        KeyCode::Char('p')                => app.show_password = !app.show_password,
        KeyCode::Char('c')                => app.copy_password_to_clipboard(),
        KeyCode::Char('j') | KeyCode::Down => {
            app.show_password = false;
            app.move_down();
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.show_password = false;
            app.move_up();
        }
        _ => {}
    }
}

fn handle_search(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => app.go_back(),

        // Enter opens the selected search result in detail view
        KeyCode::Enter => {
            if !app.search_results.is_empty() {
                // Promote search results to the main list so detail/navigation work
                app.items = app.search_results.clone();
                app.screen = Screen::Detail;
                app.show_password = false;
            }
        }

        KeyCode::Char('j') | KeyCode::Down => app.move_down(),
        KeyCode::Char('k') | KeyCode::Up   => app.move_up(),

        KeyCode::Backspace => {
            app.search_query.pop();
            // Re-filter instantly — pure in-memory, no bw calls
            app.perform_search();
        }

        KeyCode::Char(c) => {
            app.search_query.push(c);
            app.perform_search();
        }

        _ => {}
    }
}

fn handle_help(app: &mut App, key: KeyEvent) {
    // Any key closes the help overlay
    let _ = key;
    app.go_back();
}