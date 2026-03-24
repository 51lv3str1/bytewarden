/// main.rs — Entry point for bytewarden
///
/// TypeScript analogy:
///   async function main() {
///     const app = new App();
///     while (!app.shouldQuit) {
///       terminal.render(() => draw(app));
///       const event = await readKeyEvent();
///       handleEvents(app, event);
///     }
///   }
///
/// The Ratatui loop is synchronous — no async/await.
/// crossterm::event::read() blocks the thread until a key arrives.

mod app;
mod bw;
mod events;
mod ui;

use app::App;
use color_eyre::Result;

fn main() -> Result<()> {
    color_eyre::install()?;

    // ratatui::run handles terminal init, alternate screen, raw mode,
    // and restores the terminal on exit — even on panic.
    ratatui::run(|terminal| {
        let mut app = App::new();

        loop {
            // 1. RENDER — draw current state to the terminal buffer
            terminal.draw(|frame| ui::draw(frame, &app))?;

            // 2. EVENTS — block until a key arrives, then mutate app state
            events::handle_events(&mut app)?;

            // 3. EXIT — break if the user requested quit
            if app.should_quit {
                break;
            }
        }

        Ok(())
    })
}