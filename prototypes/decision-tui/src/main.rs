// THROWAWAY PROTOTYPE for wayfinder ticket #7 -- for reactive evaluation, not production.
// "How does the developer decision interaction look and behave in Forge's TUI?"
// Three structurally different variants of the same scripted §16 pause moment.
// Switch with [ / ], replay with ctrl+r. `cargo run -- --snapshot` prints static renders.

mod script;
mod variants;

use std::time::Duration;

use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use script::{App, Phase, VStage};

fn main() -> std::io::Result<()> {
    if std::env::args().any(|arg| arg == "--snapshot") {
        variants::print_snapshot();
        return Ok(());
    }
    let mut terminal = ratatui::init();
    let mut app = App::new();
    let result = run(&mut terminal, &mut app);
    ratatui::restore();
    result
}

fn run(terminal: &mut ratatui::DefaultTerminal, app: &mut App) -> std::io::Result<()> {
    loop {
        terminal.draw(|f| variants::render_app(f, app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    if !handle_global(app, key) {
                        match app.variant {
                            0 => {
                                variants::handle_v1(app, key);
                            }
                            1 => {
                                variants::handle_v2(app, key);
                            }
                            _ => {
                                variants::handle_v3(app, key);
                            }
                        }
                    }
                }
            }
        }

        if app.quit {
            return Ok(());
        }

        if app.phase == Phase::Running && app.last_tick.elapsed() >= Duration::from_millis(700) {
            app.advance();
            app.last_tick = std::time::Instant::now();
        }
    }
}

fn handle_global(app: &mut App, key: KeyEvent) -> bool {
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('c') => {
                app.quit = true;
                return true;
            }
            KeyCode::Char('r') => {
                let variant = app.variant;
                *app = App::new_with_variant(variant);
                return true;
            }
            _ => {}
        }
    }

    if app.show_journal {
        if matches!(key.code, KeyCode::Char('h') | KeyCode::Esc) {
            app.show_journal = false;
        }
        return true;
    }

    let typing = app.phase == Phase::Paused
        && app.verdict.as_ref().map(|v| v.stage == VStage::Rationale).unwrap_or(false);

    if !typing {
        match key.code {
            KeyCode::Char(']') => {
                app.variant = (app.variant + 1) % 3;
                return true;
            }
            KeyCode::Char('[') => {
                app.variant = (app.variant + 2) % 3;
                return true;
            }
            KeyCode::Char('h') => {
                app.show_journal = true;
                return true;
            }
            KeyCode::Char('q') => {
                app.quit = true;
                return true;
            }
            _ => {}
        }
    }

    false
}
