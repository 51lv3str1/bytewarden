/// main.rs — Entry point for bytewarden

mod app;
mod bw;
mod events;
mod theme;
mod ui;

use app::{ActionState, App, PendingAction, Screen};
use color_eyre::Result;
use crossterm::{event, execute};
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use std::time::Duration;

fn main() -> Result<()> {
    color_eyre::install()?;

    ratatui::run(|terminal| {
        let mut app = App::new();
        execute!(std::io::stdout(), EnableMouseCapture)?;

        // Show splash with spinner while bw status runs
        app.set_action(ActionState::Running("Checking session…".into()));
        terminal.draw(|frame| ui::draw(frame, &mut app))?;

        app.resume_from_status();

        // After status check: if we went straight to vault, stay there.
        // Otherwise switch to Login and clear the spinner.
        if app.screen != Screen::Vault {
            app.screen = Screen::Login;
        }
        if matches!(app.action_state, ActionState::Running(_)) {
            app.set_action(ActionState::Idle);
        }

        let result = run_loop(terminal, &mut app);
        let _ = execute!(std::io::stdout(), DisableMouseCapture);
        result
    })
}

fn run_loop(terminal: &mut ratatui::DefaultTerminal, app: &mut App) -> color_eyre::Result<()> {
    // Counts frames since last Done/Error state — clears the status after ~1.5 s.
    let mut done_ticks: u8 = 0;

    loop {
        terminal.draw(|frame| ui::draw(frame, app))?;

        // Dispatch any pending action AFTER the Running frame is drawn,
        // so the spinner is visible before the blocking bw call.
        if app.pending_action != PendingAction::None {
            dispatch_pending(app);
            done_ticks = 0;
            terminal.draw(|frame| ui::draw(frame, app))?;
        }

        if event::poll(poll_timeout(&app.action_state))? {
            events::handle_events(app)?;
            app.reset_activity();
        } else {
            app.check_auto_lock();
            tick_state(app, &mut done_ticks);
        }

        if app.should_quit { break; }
    }
    Ok(())
}

/// Execute the pending action and clear it.
fn dispatch_pending(app: &mut App) {
    let pending = std::mem::replace(&mut app.pending_action, PendingAction::None);
    match pending {
        PendingAction::Login                     => app.do_login(),
        PendingAction::CopyUsername              => app.do_copy_username(),
        PendingAction::CopyPassword              => app.do_copy_password(),
        PendingAction::SyncVault                 => app.do_sync_vault(),
        PendingAction::ToggleFavorite            => app.do_toggle_favorite(),
        PendingAction::CopyRaw(text, msg)        => app.do_copy_raw(text, msg),
        PendingAction::CopyTotp(item_id)         => app.do_copy_totp(item_id),
        PendingAction::SaveEdit                  => app.do_save_edit(),
        PendingAction::CreateItem                => app.do_create_item(),
        PendingAction::DeleteItem { permanent }  => app.do_delete_item(permanent),
        PendingAction::RestoreItem               => app.do_restore_item(),
        PendingAction::LoadTrash                 => app.load_trash(),
        PendingAction::LoadItems                 => app.load_items(),
        PendingAction::None                      => {}
    }
}

/// Poll timeout — fast during animation, slow when idle.
fn poll_timeout(state: &ActionState) -> Duration {
    match state {
        ActionState::Idle => Duration::from_secs(60),
        _                 => Duration::from_millis(80),
    }
}

/// Advance spinner or expire Done/Error feedback after ~1.5 s (19 × 80 ms).
fn tick_state(app: &mut App, done_ticks: &mut u8) {
    match &app.action_state {
        ActionState::Running(_) => app.tick_action(),
        ActionState::Done(_) | ActionState::Error(_) => {
            *done_ticks += 1;
            if *done_ticks >= 19 {
                app.set_action(ActionState::Idle);
                *done_ticks = 0;
            }
        }
        ActionState::Idle => {}
    }
}
