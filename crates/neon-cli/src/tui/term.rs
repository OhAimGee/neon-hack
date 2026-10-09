//! Entering the full-screen mode, and always giving the terminal back.
//!
//! This is where the most is lost if a mistake is made: a game that dies and leaves the
//! shell in raw mode, on the alternate screen, with no cursor and a terminal that sends
//! escape sequences for every paste. So the terminal is restored by every way out there is:
//!
//! * a normal end and an error: the [`TerminalGuard`] restores it when it is dropped;
//! * a panic: a hook restores it **before** the message is printed (the message then lands
//!   on the normal screen, where it can be read), and it is the only thing that runs when
//!   the program is built with `panic = "abort"`; ratatui's own `init()` is not used
//!   because its hook would restore a second time, after ours, on the normal screen;
//! * `SIGTERM`, `SIGHUP`, `SIGQUIT` and `SIGINT` sent from outside (Unix): a thread
//!   restores the terminal and lets the signal do what it would have done.
//!
//! Ctrl-C typed at the keyboard is none of these: in raw mode it is a key, and the
//! interface turns it into `quit`. `ratatui::restore()` alone would not be enough: it leaves
//! raw mode and the alternate screen but not bracketed paste, mouse capture or the cursor.

use std::io::{self, stdout};
use std::sync::atomic::{AtomicBool, Ordering};

use ratatui::DefaultTerminal;
use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::cursor::Show;
use ratatui::crossterm::event::{
    DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste,
};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};

/// The terminal is in full-screen mode and has to be given back.
static ACTIVE: AtomicBool = AtomicBool::new(false);

/// Gives the terminal back when dropped. At most one lives at a time.
pub(crate) struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        restore();
    }
}

/// Gives the terminal back: cursor, mouse, bracketed paste, then the alternate screen and
/// cooked mode. Idempotent, and a no-op when the terminal was never taken, so that the
/// guard, the panic hook and the signal thread can all call it.
pub(crate) fn restore() {
    if ACTIVE.swap(false, Ordering::SeqCst) {
        // Nothing can be done about a failure here: the terminal may already be gone.
        let _ = execute!(
            stdout(),
            DisableMouseCapture,
            DisableBracketedPaste,
            Show,
            LeaveAlternateScreen
        );
        let _ = disable_raw_mode();
    }
}

/// Takes the terminal: raw mode, alternate screen, bracketed paste. The mouse is left to
/// the terminal, so that selecting and copying keep working.
pub(crate) fn enter() -> io::Result<(DefaultTerminal, TerminalGuard)> {
    enable_raw_mode()?;
    // From here on every way out, including `?`, gives the terminal back.
    ACTIVE.store(true, Ordering::SeqCst);
    let guard = TerminalGuard;
    install_panic_hook();
    execute!(stdout(), EnterAlternateScreen, EnableBracketedPaste)?;
    #[cfg(unix)]
    unix::restore_on_signals()?;
    let terminal = DefaultTerminal::new(CrosstermBackend::new(stdout()))?;
    Ok((terminal, guard))
}

/// The hook restores everything, then lets the previous hook print the message, which now
/// reaches the normal screen.
fn install_panic_hook() {
    static INSTALLED: AtomicBool = AtomicBool::new(false);
    if INSTALLED.swap(true, Ordering::SeqCst) {
        return;
    }
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore();
        previous(info);
    }));
}

#[cfg(unix)]
mod unix {
    use std::io;
    use std::sync::Once;

    use signal_hook::consts::{SIGHUP, SIGINT, SIGQUIT, SIGTERM};
    use signal_hook::iterator::Signals;
    use signal_hook::low_level::emulate_default_handler;

    /// Restores the terminal when the process is told to stop from the outside, then dies as
    /// it would have without us (so the exit status still says which signal killed it).
    pub(super) fn restore_on_signals() -> io::Result<()> {
        static INSTALLED: Once = Once::new();
        let mut result = Ok(());
        INSTALLED.call_once(|| {
            result = spawn_watcher();
        });
        result
    }

    fn spawn_watcher() -> io::Result<()> {
        let mut signals = Signals::new([SIGHUP, SIGINT, SIGQUIT, SIGTERM])?;
        std::thread::Builder::new()
            .name("neon-hack-signals".to_owned())
            .spawn(move || {
                if let Some(signal) = signals.forever().next() {
                    super::restore();
                    // The default action ends the process; nothing is left to do if it fails.
                    let _ = emulate_default_handler(signal);
                }
            })?;
        Ok(())
    }
}
