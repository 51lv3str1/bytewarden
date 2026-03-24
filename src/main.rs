/// main.rs — Entry point for bytewarden

mod app;
mod bw;
mod events;
mod theme;
mod ui;

use app::{ActionState, App, PendingAction};
use color_eyre::Result;
use crossterm::event;
use std::time::Duration;

fn main() -> Result<()> {
    color_eyre::install()?;

    ratatui::run(|terminal| {
        let mut app = App::new();
        // How many ticks a Done/Error state stays visible (each tick = 80ms)
        let mut done_ticks: u8 = 0;

        loop {
            // 1. RENDER — always draw before executing pending work
            terminal.draw(|frame| ui::draw(frame, &app))?;

            // 2. Execute pending action AFTER the Running frame is rendered
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
                done_ticks = 0;
                // Re-render immediately to show Done/Error state
                terminal.draw(|frame| ui::draw(frame, &app))?;
            }

            // 3. Poll for input with timeout based on state
            let timeout = match &app.action_state {
                ActionState::Running(_)            => Duration::from_millis(80),
                ActionState::Done(_) | ActionState::Error(_) => Duration::from_millis(80),
                ActionState::Idle                  => Duration::from_secs(60),
            };

            if event::poll(timeout)? {
                events::handle_events(&mut app)?;
                // Any keypress while Done/Error → clear immediately
                if matches!(&app.action_state, ActionState::Done(_) | ActionState::Error(_)) {
                    done_ticks = 0;
                }
            } else {
                match &app.action_state {
                    ActionState::Running(_) => app.tick_action(),
                    ActionState::Done(_) | ActionState::Error(_) => {
                        done_ticks += 1;
                        // Auto-clear after ~1.5s (≈19 ticks × 80ms)
                        if done_ticks >= 19 {
                            app.set_action(ActionState::Idle);
                            done_ticks = 0;
                        }
                    }
                    ActionState::Idle => {}
                }
            }

            if app.should_quit { break; }
        }
        Ok(())
    })
}
