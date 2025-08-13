use std::io::{self, stderr, stdout, Stdout};

use crossterm::{
    event::{KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags},
    execute,
    terminal::*,
};
use ratatui::prelude::*;

/// A type alias for the terminal type used in this application
pub type Tui = Terminal<CrosstermBackend<Stdout>>;

/// Initialize the terminal
pub fn init() -> io::Result<Tui> {
    execute!(stdout(), EnterAlternateScreen)?;
    execute!(stderr(), EnterAlternateScreen)?;
    if let Ok(true) = crossterm::terminal::supports_keyboard_enhancement() {
        execute!(
            stdout(),
            PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
        )?;
    }

    enable_raw_mode()?;

    Terminal::new(CrosstermBackend::new(stdout()))
}

/// Restore the terminal to its original state
pub fn restore() -> io::Result<()> {
    execute!(stdout(), LeaveAlternateScreen)?;
    execute!(stderr(), LeaveAlternateScreen)?;
    if let Ok(true) = crossterm::terminal::supports_keyboard_enhancement() {
        execute!(stdout(), PopKeyboardEnhancementFlags)?;
    }

    disable_raw_mode()?;

    Ok(())
}
