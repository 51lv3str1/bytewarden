/// main.rs — Entry point for bytewarden

mod app;
mod bw;
mod events;
mod theme;
mod ui;

use app::{ActionState, App, PendingAction};
use color_eyre::Result;
use crossterm::{event, execute};
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use std::time::Duration;

fn main() -> Result<()> {
    color_eyre::install()?;

    ratatui::run(|terminal| {
        let mut app = App::new();
        let mut done_ticks: u8 = 0;

        // Enable mouse capture
        execute!(std::io::stdout(), EnableMouseCapture)?;

        let result = run_loop(terminal, &mut app, &mut done_ticks);

        // Always disable mouse on exit
        let _ = execute!(std::io::stdout(), DisableMouseCapture);
        result
    })
}

fn run_loop(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut App,
    done_ticks: &mut u8,
) -> color_eyre::Result<()> {
    loop {
        terminal.draw(|frame| ui::draw(frame, app))?;

        // Execute pending action AFTER the Running frame is rendered
        if app.pending_action != PendingAction::None {
            let pending = app.pending_action.clone();
            app.pending_action = PendingAction::None;
            match pending {
                PendingAction::CopyUsername              => app.do_copy_username(),
                PendingAction::CopyPassword              => app.do_copy_password(),
                PendingAction::SyncVault                 => app.do_sync_vault(),
                PendingAction::ToggleFavorite            => app.do_toggle_favorite(),
                PendingAction::CopyRaw(text, msg)        => app.do_copy_raw(text, msg),
                PendingAction::CopyTotp(item_id)         => app.do_copy_totp(item_id),
                PendingAction::None                      => {}
            }
            *done_ticks = 0;
            terminal.draw(|frame| ui::draw(frame, app))?;
        }

        let timeout = match &app.action_state {
            ActionState::Running(_)                    => Duration::from_millis(80),
            ActionState::Done(_) | ActionState::Error(_) => Duration::from_millis(80),
            ActionState::Idle                          => Duration::from_secs(60),
        };

        if event::poll(timeout)? {
            events::handle_events(app)?;
        } else {
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

        if app.should_quit { break; }
    }
    Ok(())
}
