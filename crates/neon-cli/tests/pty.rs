//! The real binary in a pseudo-terminal, read through a terminal emulator.
//!
//! These tests start `neon-hack` on the slave side of a pty, type on the master side and look
//! at what a terminal would show (`vt100` keeps the screen, the alternate screen, the cursor,
//! bracketed paste and mouse modes). They check what no `TestBackend` can: that the terminal is
//! taken, resized, and always given back.
//!
//! Hermetic: the environment is cleared and rebuilt, the data folder is a temporary one, the
//! seed is fixed, and no test touches the real data folder. Never flaky by design: nothing
//! sleeps for a fixed time; every wait polls for a condition with a deadline and prints the
//! screen when it runs out. Unix only (a `ConPTY` rewrites what it relays, so Windows relies on
//! `TestBackend` and the plain tests).

#![cfg(all(unix, feature = "tui"))]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test code: a failure is a panic with the screen that explains it"
)]

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

use portable_pty::{Child, CommandBuilder, ExitStatus, MasterPty, PtySize, native_pty_system};
use tempfile::TempDir;

const BIN: &str = env!("CARGO_BIN_EXE_neon-hack");
/// How long a condition may take before the test gives up and shows the screen.
const DEADLINE: Duration = Duration::from_secs(20);
const CTRL_C: &[u8] = b"\x03";
const CTRL_D: &[u8] = b"\x04";
const ESCAPE: &[u8] = b"\x1b";

struct Session {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    child: Box<dyn Child + Send + Sync>,
    output: Receiver<Vec<u8>>,
    parser: vt100::Parser,
    data: PathBuf,
    /// Owns the data folder of a session that made its own.
    #[allow(
        dead_code,
        reason = "kept alive so that the folder is removed when the test ends"
    )]
    own_data: Option<TempDir>,
    /// Bytes the program has written so far.
    received: usize,
    exit: Option<ExitStatus>,
}

/// What `settings.toml` holds when the test starts.
enum Settings<'a> {
    /// No file: the first launch.
    Absent,
    /// A file with this content.
    Text(&'a str),
}

fn size(rows: u16, cols: u16) -> PtySize {
    PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    }
}

impl Session {
    /// A game in a data folder of its own, with a settings file that settles the first
    /// question.
    fn start(rows: u16, cols: u16, args: &[&str]) -> Self {
        Self::start_with(rows, cols, args, &Settings::Text("# test\n"), &[])
    }

    fn start_with(
        rows: u16,
        cols: u16,
        args: &[&str],
        settings: &Settings<'_>,
        extra_env: &[(&str, &str)],
    ) -> Self {
        let folder = tempfile::tempdir().unwrap();
        if let Settings::Text(text) = settings {
            std::fs::write(folder.path().join("settings.toml"), text).unwrap();
        }
        let path = folder.path().to_path_buf();
        Self::spawn(rows, cols, args, &path, extra_env, Some(folder))
    }

    /// The program in a data folder the test keeps (to start it twice on the same saves).
    fn launch(
        rows: u16,
        cols: u16,
        args: &[&str],
        data: &Path,
        extra_env: &[(&str, &str)],
    ) -> Self {
        Self::spawn(rows, cols, args, data, extra_env, None)
    }

    fn spawn(
        rows: u16,
        cols: u16,
        args: &[&str],
        data: &Path,
        extra_env: &[(&str, &str)],
        own_data: Option<TempDir>,
    ) -> Self {
        let pair = native_pty_system().openpty(size(rows, cols)).unwrap();
        let mut command = CommandBuilder::new(BIN);
        // Nothing of the host's environment leaks in: the run depends on these lines only.
        command.env_clear();
        command.env("TERM", "xterm-256color");
        command.env("LANG", "C.UTF-8");
        command.env("NEON_HACK_DATA_DIR", data);
        for (name, value) in extra_env {
            command.env(name, value);
        }
        command.args(["--seed", "7"]);
        command.args(args);
        let child = pair.slave.spawn_command(command).unwrap();
        // Without this the reader never sees the end of the output.
        drop(pair.slave);
        let mut reader = pair.master.try_clone_reader().unwrap();
        let writer = pair.master.take_writer().unwrap();
        let (sender, output) = mpsc::channel();
        thread::spawn(move || {
            let mut buffer = [0_u8; 8192];
            // Linux reports EIO, not 0, when the slave side is closed: both end the stream.
            while let Ok(count @ 1..) = reader.read(&mut buffer) {
                if sender.send(buffer[..count].to_vec()).is_err() {
                    break;
                }
            }
        });
        Self {
            master: pair.master,
            writer,
            child,
            output,
            parser: vt100::Parser::new(rows, cols, 0),
            data: data.to_path_buf(),
            own_data,
            received: 0,
            exit: None,
        }
    }

    // ---- Looking ---------------------------------------------------------------------------

    /// Feeds what has arrived to the emulator; false once the stream is over.
    fn pump(&mut self, wait: Duration) -> bool {
        match self.output.recv_timeout(wait) {
            Ok(bytes) => {
                self.take(&bytes);
                while let Ok(more) = self.output.try_recv() {
                    self.take(&more);
                }
                true
            }
            Err(RecvTimeoutError::Timeout) => true,
            Err(RecvTimeoutError::Disconnected) => false,
        }
    }

    fn take(&mut self, bytes: &[u8]) {
        self.received += bytes.len();
        self.parser.process(bytes);
    }

    fn screen(&self) -> &vt100::Screen {
        self.parser.screen()
    }

    fn text(&self) -> String {
        self.screen().contents()
    }

    fn rows(&self) -> Vec<String> {
        let (_, cols) = self.screen().size();
        self.screen()
            .rows(0, cols)
            .map(|row| row.trim_end().to_owned())
            .collect()
    }

    fn contains(&self, needle: &str) -> bool {
        self.text().contains(needle)
    }

    /// The input line: the row above the help bar, the last one.
    fn input_row(&self) -> String {
        let rows = self.rows();
        rows[rows.len() - 2].clone()
    }

    /// Waits until the screen satisfies `condition`, or fails with the screen.
    fn wait_for(&mut self, what: &str, condition: impl Fn(&Session) -> bool) {
        let started = Instant::now();
        loop {
            self.pump(Duration::from_millis(10));
            if condition(self) {
                return;
            }
            assert!(
                started.elapsed() < DEADLINE,
                "timed out waiting for {what}.\nscreen ({:?}):\n{}\n",
                self.screen().size(),
                self.text()
            );
        }
    }

    fn wait_text(&mut self, needle: &str) {
        self.wait_for(&format!("{needle:?}"), |session| session.contains(needle));
    }

    fn wait_input(&mut self, line: &str) {
        self.wait_for(&format!("the input line {line:?}"), |session| {
            session.input_row() == line
        });
    }

    /// Waits until there is no more output for `quiet`: the screen has settled.
    fn settle(&mut self, quiet: Duration) {
        let started = Instant::now();
        loop {
            let before = self.received;
            self.pump(quiet);
            if self.received == before {
                return;
            }
            assert!(started.elapsed() < DEADLINE, "the output never settles");
        }
    }

    // ---- Typing ------------------------------------------------------------------------------

    fn send(&mut self, bytes: &[u8]) {
        self.writer.write_all(bytes).unwrap();
        self.writer.flush().unwrap();
    }

    fn type_line(&mut self, line: &str) {
        self.send(line.as_bytes());
        self.send(b"\r");
    }

    fn resize(&mut self, rows: u16, cols: u16) {
        self.master.resize(size(rows, cols)).unwrap();
        self.parser.screen_mut().set_size(rows, cols);
    }

    fn signal(&self, name: &str) {
        let pid = self.child.process_id().unwrap();
        let status = std::process::Command::new("kill")
            .args([format!("-{name}"), pid.to_string()])
            .status()
            .unwrap();
        assert!(status.success());
    }

    // ---- Ending --------------------------------------------------------------------------------

    /// Waits for the program to end, then reads everything it wrote.
    fn finish(&mut self) -> ExitStatus {
        let started = Instant::now();
        let status = loop {
            self.pump(Duration::from_millis(10));
            if let Some(status) = self.child.try_wait().unwrap() {
                break status;
            }
            assert!(
                started.elapsed() < DEADLINE,
                "the program did not end.\nscreen:\n{}\n",
                self.text()
            );
        };
        // The reader ends when the pty closes: everything written is then in the emulator.
        while self.pump(Duration::from_millis(50)) && started.elapsed() < DEADLINE {}
        self.exit = Some(status.clone());
        status
    }

    /// The saved autosave of the campaign, if the game wrote one.
    fn autosave(&self) -> PathBuf {
        self.data.join("saves-campaign").join("auto.toml")
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        if self.exit.is_none() {
            let _ = self.child.kill();
        }
    }
}

/// The terminal is as a shell expects to find it: normal screen, visible cursor, no paste
/// mode, no mouse reporting.
fn assert_restored(session: &Session) {
    let screen = session.screen();
    assert!(!screen.alternate_screen(), "still on the alternate screen");
    assert!(!screen.hide_cursor(), "the cursor was left hidden");
    assert!(!screen.bracketed_paste(), "bracketed paste was left on");
    assert_eq!(
        screen.mouse_protocol_mode(),
        vt100::MouseProtocolMode::None,
        "mouse reporting was left on"
    );
}

/// The game is on its own screen: alternate screen, bracketed paste on, no mouse reporting.
fn assert_taken(session: &Session) {
    let screen = session.screen();
    assert!(screen.alternate_screen());
    assert!(screen.bracketed_paste(), "a paste must never run a command");
    assert_eq!(
        screen.mouse_protocol_mode(),
        vt100::MouseProtocolMode::None,
        "the mouse is not captured, so the terminal keeps selection and copy"
    );
}

/// Waits for the full-screen interface to ask for the handle.
fn wait_for_the_handle_question(session: &mut Session) {
    session.wait_for("the full-screen interface", |s| {
        s.screen().alternate_screen() && s.contains("Handle [")
    });
}

/// Starts a game full screen and gets through the opening to the command line: a handle, the
/// prologue skipped with Escape, the tutorial declined.
fn at_command_line_as(rows: u16, cols: u16, handle: &str) -> Session {
    let mut session = Session::start(rows, cols, &[]);
    wait_for_the_handle_question(&mut session);
    play_the_opening(&mut session, handle);
    session
}

fn at_command_line(rows: u16, cols: u16) -> Session {
    at_command_line_as(rows, cols, "Neon")
}

fn play_the_opening(session: &mut Session, handle: &str) {
    session.type_line(handle);
    session.wait_text("[Enter] to continue");
    // Escape backs out of the prologue to its last page.
    session.send(ESCAPE);
    session.wait_text("[Y/n]");
    session.type_line("n");
    session.wait_input(">");
}

/// Quits the way the player does: Ctrl-C asks, `y` confirms, any key leaves.
fn quit_with_control_c(session: &mut Session) -> ExitStatus {
    session.send(CTRL_C);
    session.wait_text("> quit");
    session.wait_for("the question", |s| {
        s.input_row().contains("[Y/n]") || s.input_row().contains("[y/N]")
    });
    session.type_line("y");
    session.wait_text("Press any key to leave.");
    session.send(b" ");
    session.finish()
}

// ---- The full-screen interface --------------------------------------------------------------

#[test]
fn a_new_game_opens_full_screen_with_the_layout_of_the_wide_tier() {
    let mut session = Session::start(28, 100, &[]);
    wait_for_the_handle_question(&mut session);
    assert_taken(&session);
    session.settle(Duration::from_millis(100));
    let rows = session.rows();
    assert_eq!(rows.len(), 28);
    assert!(rows[0].contains("Notoriety 0/100 (Discreet)"), "{rows:#?}");
    assert!(
        rows[1].starts_with("╭Terminal") && rows[1].contains("╭Panel"),
        "{rows:#?}"
    );
    assert!(rows[27].starts_with("Tab complete"), "{rows:#?}");
    assert!(session.contains("== N E O N   H A C K =="));
    assert_eq!(session.input_row(), "Handle [Neon]:");
    // The cursor is on the input line, after the label.
    let (row, col) = session.screen().cursor_position();
    assert_eq!((row, col), (26, 15));
    assert!(
        !session.screen().hide_cursor(),
        "the cursor is visible to type"
    );
}

#[test]
fn the_start_of_the_campaign_plays_through_typed_keys_tab_and_history() {
    let mut session = at_command_line_as(28, 100, "Zoë日");
    // Unicode survives the whole way: typed, echoed, kept, and shown in the status bar.
    assert!(session.rows()[0].contains("Zoë日"), "{:#?}", session.rows());

    // TAB completes the command: `sta` is only `status`.
    session.send(b"sta\t");
    session.wait_input("> status");
    session.send(b"\r");
    session.wait_for("the status sheet", |s| {
        s.rows()
            .iter()
            .filter(|row| row.contains("> status"))
            .count()
            >= 1
            && s.input_row() == ">"
    });

    // Up recalls it; typing over it edits a copy.
    session.send(b"\x1b[A");
    session.wait_input("> status");
    session.send(b"\x15"); // Ctrl-U
    session.wait_input(">");

    // What TAB cannot decide is listed under the log: `q` is `quests` or `quit`.
    session.send(b"q\t");
    session.wait_for("the candidates", |s| {
        let rows = s.rows();
        let listing = &rows[rows.len() - 3];
        listing.contains("quests") && listing.contains("quit")
    });
    assert_eq!(
        session.input_row(),
        "> qu",
        "completed as far as they agree"
    );
    session.send(b"e\t");
    session.wait_input("> quests");
    session.send(b"\r");
    session.wait_for("the list of quests", |s| s.contains("> quests"));
    session.wait_input(">");

    // An argument is completed from the game: `talk ec` is `talk echo7`.
    session.send(b"talk ec\t");
    session.wait_input("> talk echo7");
    session.send(b"\x15");

    // Page keys scroll the log and say so; Escape comes back down.
    session.send(b"\x1b[5~");
    session.wait_text("Scrolled back");
    session.send(ESCAPE);
    session.wait_for("the bottom of the log", |s| !s.contains("Scrolled back"));

    let status = quit_with_control_c(&mut session);
    assert!(status.success(), "{status:?}");
    assert_restored(&session);
    // The game was saved by the shared persistence, as the plain interface does.
    assert!(session.autosave().exists());
}

#[test]
fn the_compact_tier_at_64x20_has_no_panel() {
    let mut session = at_command_line(20, 64);
    let rows = session.rows();
    assert!(
        rows[0].contains("Neon") && rows[0].contains("Notoriety 0/100"),
        "{rows:#?}"
    );
    assert!(!session.contains("Panel"), "{rows:#?}");
    assert!(
        !session.contains('╭'.to_string().as_str()),
        "no boxes in the compact tier"
    );
    assert!(rows[19].starts_with("Tab complete"));
    assert_eq!(session.input_row(), ">");
    session.send(b"help\r");
    session.wait_text("> help");
    let status = quit_with_control_c(&mut session);
    assert!(status.success());
    assert_restored(&session);
}

#[test]
fn resizing_redraws_every_tier_and_loses_nothing() {
    let mut session = at_command_line(28, 100);
    assert!(session.contains("Panel"));
    session.send(b"sta");
    session.wait_input("> sta");

    session.resize(20, 64);
    session.wait_for("the compact tier", |s| {
        !s.contains("Panel") && s.rows()[19].starts_with("Tab complete")
    });
    assert_eq!(session.input_row(), "> sta", "the typed line is kept");

    session.resize(15, 50);
    session.wait_text("Terminal too small (50x15)");
    assert!(session.contains("64x20"));
    // The centred sentence replaces the whole screen, and what is typed meanwhile is ignored.
    assert!(!session.contains("Tab complete"));
    session.send(b"zzz\r\t");
    session.settle(Duration::from_millis(100));

    session.resize(28, 100);
    session.wait_for("the wide tier again", |s| {
        s.contains("Panel") && s.input_row() == "> sta"
    });
    assert!(
        !session.contains("zzz"),
        "nothing was typed while it was too small"
    );
    let status = quit_with_control_c(&mut session);
    assert!(status.success());
    assert_restored(&session);
}

#[test]
fn control_c_asks_before_quitting_and_the_answer_can_be_no() {
    let mut session = at_command_line(28, 100);
    session.send(CTRL_C);
    session.wait_text("> quit");
    session.wait_for("the question", |s| {
        s.input_row().contains("[Y/n]") || s.input_row().contains("[y/N]")
    });
    assert_taken(&session);
    session.type_line("n");
    session.wait_input(">");
    assert!(session.screen().alternate_screen(), "the game goes on");
    let status = quit_with_control_c(&mut session);
    assert!(status.success());
    assert_restored(&session);
}

#[test]
fn control_d_on_an_empty_line_closes_the_input_and_the_terminal_is_given_back() {
    let mut session = at_command_line(28, 100);
    session.send(CTRL_D);
    session.wait_text("Input closed: ending the session.");
    session.wait_text("Press any key to leave.");
    session.send(b"x");
    let status = session.finish();
    assert!(status.success());
    assert_restored(&session);
}

#[test]
fn a_paste_lands_in_the_line_as_one_line_and_runs_nothing() {
    let mut session = at_command_line(28, 100);
    session.send(b"\x1b[200~status\nstatus\r\nquit\x1b[201~");
    session.wait_input("> status status quit");
    session.settle(Duration::from_millis(100));
    assert!(
        !session.rows().iter().any(|row| row.contains("│> status")),
        "nothing was run: no echo of a command in the log: {:#?}",
        session.rows()
    );
    assert!(session.screen().alternate_screen());
}

#[test]
fn an_idle_interface_writes_nothing() {
    let mut session = at_command_line(28, 100);
    // Quiet for a while first, so that a slow machine has delivered everything it will.
    session.settle(Duration::from_millis(300));
    let before = session.received;
    session.pump(Duration::from_millis(500));
    assert_eq!(
        session.received, before,
        "no timer, no animation, no polling"
    );
}

#[test]
fn a_saved_game_is_resumed_in_the_full_screen_interface() {
    let folder = tempfile::tempdir().unwrap();
    std::fs::write(folder.path().join("settings.toml"), "# test\n").unwrap();
    let mut first = Session::launch(28, 100, &[], folder.path(), &[]);
    wait_for_the_handle_question(&mut first);
    play_the_opening(&mut first, "Neon");
    first.send(b"talk echo7\r");
    first.wait_text("> talk echo7");
    first.send(ESCAPE);
    first.wait_input(">");
    assert!(quit_with_control_c(&mut first).success());

    let mut second = Session::launch(28, 100, &[], folder.path(), &[]);
    second.wait_for("the resumed game", |s| {
        s.screen().alternate_screen() && s.contains("Resuming your saved game.")
    });
    assert!(second.rows()[0].contains("Neon"), "{:#?}", second.rows());
    assert!(
        !second.contains("Handle ["),
        "the opening is not played twice"
    );
    second.wait_input(">");
    assert!(quit_with_control_c(&mut second).success());
    assert_restored(&second);
}

// ---- The terminal is always given back --------------------------------------------------------

#[test]
fn the_terminal_is_given_back_after_a_panic() {
    if !cfg!(debug_assertions) {
        // The failure is wired in debug builds only.
        return;
    }
    let mut session = Session::start_with(
        28,
        100,
        &[],
        &Settings::Text("# test\n"),
        &[("NEON_HACK_TEST_FAULT", "panic")],
    );
    let status = session.finish();
    assert!(!status.success(), "{status:?}");
    assert_restored(&session);
    // The message is on the normal screen, where it can be read.
    assert!(session.contains("forced panic"), "{}", session.text());
}

#[test]
fn the_terminal_is_given_back_after_an_error() {
    if !cfg!(debug_assertions) {
        return;
    }
    let mut session = Session::start_with(
        28,
        100,
        &[],
        &Settings::Text("# test\n"),
        &[("NEON_HACK_TEST_FAULT", "error")],
    );
    let status = session.finish();
    assert_eq!(status.exit_code(), 1);
    assert_restored(&session);
    assert!(
        session.contains("neon-hack: forced error"),
        "{}",
        session.text()
    );
}

#[test]
fn the_terminal_is_given_back_when_the_process_is_told_to_stop() {
    for signal in ["TERM", "HUP"] {
        let mut session = at_command_line(28, 100);
        assert_taken(&session);
        session.signal(signal);
        let status = session.finish();
        assert!(
            !status.success(),
            "{signal}: still the signal's exit status: {status:?}"
        );
        assert_restored(&session);
    }
}

// ---- Falling back to the plain interface -------------------------------------------------------------

#[test]
fn a_terminal_smaller_than_the_minimum_falls_back_to_the_plain_interface() {
    let mut session = Session::start(15, 50, &[]);
    session.wait_text("Handle [");
    assert!(
        !session.screen().alternate_screen(),
        "no full screen below 64x20"
    );
    assert!(
        session.contains("neon-hack: The terminal is too small (50x15, at least 64x20 needed)")
    );
    assert!(session.contains("line-by-line interface"));
    // The plain game goes on: the terminal echoes what is typed, and Ctrl-D closes the input.
    session.type_line("Neon");
    session.wait_text("[Enter] to continue");
    session.send(CTRL_D);
    let status = session.finish();
    assert!(status.success(), "{status:?}");
    assert_restored(&session);
}

#[test]
fn the_plain_flag_keeps_a_big_terminal_line_by_line_without_a_word() {
    let mut session = Session::start(28, 100, &["--plain"]);
    session.wait_text("Handle [");
    assert!(!session.screen().alternate_screen());
    assert!(
        !session.contains("neon-hack:"),
        "nothing to excuse: it was asked for"
    );
    session.send(CTRL_D);
    assert!(session.finish().success());
}

#[test]
fn asking_for_the_full_screen_in_a_small_terminal_is_answered_too() {
    let mut session = Session::start(15, 50, &["--tui"]);
    session.wait_text("Handle [");
    assert!(session.contains("too small"));
    session.send(CTRL_D);
    assert!(session.finish().success());
}

// ---- The first question ------------------------------------------------------------------------------

fn settings_text(session: &Session) -> String {
    std::fs::read_to_string(session.data.join("settings.toml")).unwrap()
}

#[test]
fn the_first_launch_asks_how_to_play_and_remembers_the_answer() {
    let mut session = Session::start_with(28, 100, &[], &Settings::Absent, &[]);
    session.wait_text("Your choice [1]:");
    assert!(
        !session.screen().alternate_screen(),
        "the question is plain text"
    );
    for entry in ["[1] Full screen", "[2] Line by line", "[3] Screen reader"] {
        assert!(session.contains(entry), "{entry}");
    }
    assert!(
        !session.data.join("settings.toml").exists(),
        "nothing is written before the answer"
    );
    session.type_line("maybe");
    session.wait_text("Answer 1, 2 or 3.");
    session.type_line("2");
    session.wait_text("Handle [");
    assert!(session.contains("Your choice is saved in"));
    assert!(
        !session.screen().alternate_screen(),
        "answer 2 is line by line"
    );
    assert!(settings_text(&session).contains("display = \"plain\""));
    session.send(CTRL_D);
    assert!(session.finish().success());
}

#[test]
fn answering_full_screen_starts_the_full_screen_interface() {
    let mut session = Session::start_with(28, 100, &[], &Settings::Absent, &[]);
    session.wait_text("Your choice [1]:");
    session.type_line("");
    wait_for_the_handle_question(&mut session);
    assert_taken(&session);
    assert!(settings_text(&session).contains("display = \"tui\""));
    // Ctrl-C at a question (the handle) closes the input: there is no `quit` to ask for.
    session.send(CTRL_C);
    session.wait_text("Input closed: ending the session.");
    session.send(b" ");
    assert!(session.finish().success());
    assert_restored(&session);
}

#[test]
fn answering_screen_reader_starts_the_plain_interface_with_the_reader_settings() {
    let mut session = Session::start_with(28, 100, &[], &Settings::Absent, &[]);
    session.wait_text("Your choice [1]:");
    session.type_line("3");
    session.wait_text("Handle [");
    assert!(!session.screen().alternate_screen());
    let settings = settings_text(&session);
    assert!(
        settings.contains("display = \"plain\"") && settings.contains("screen_reader = true"),
        "{settings}"
    );
    session.send(CTRL_D);
    assert!(session.finish().success());
}

#[test]
fn no_answer_writes_nothing_and_an_existing_file_is_never_asked_about_or_touched() {
    // The input closes at the question: nothing is decided, nothing is written.
    let mut session = Session::start_with(28, 100, &[], &Settings::Absent, &[]);
    session.wait_text("Your choice [1]:");
    session.send(CTRL_D);
    wait_for_the_handle_question(&mut session);
    assert!(!session.data.join("settings.toml").exists());

    // A damaged file is not rewritten and not asked about: the game says so and plays.
    let damaged = "this is = not [toml";
    let mut session = Session::start_with(28, 100, &[], &Settings::Text(damaged), &[]);
    wait_for_the_handle_question(&mut session);
    assert!(!session.contains("Your choice"));
    session.wait_text("settings.toml could not be used");
    assert_eq!(settings_text(&session), damaged);
}

#[test]
fn the_question_is_not_asked_when_a_flag_already_decides() {
    for flag in ["--plain", "--tui", "--screen-reader"] {
        let mut session = Session::start_with(28, 100, &[flag], &Settings::Absent, &[]);
        session.wait_text("Handle [");
        assert!(!session.contains("Your choice"), "{flag}");
        assert!(!session.data.join("settings.toml").exists(), "{flag}");
    }
}

// ---- Characters and colour on a real terminal ---------------------------------------------------

/// Every cell of the screen.
fn cells(session: &Session) -> Vec<vt100::Cell> {
    let (rows, cols) = session.screen().size();
    (0..rows)
        .flat_map(|row| (0..cols).map(move |col| (row, col)))
        .filter_map(|(row, col)| session.screen().cell(row, col).cloned())
        .collect()
}

#[test]
fn ascii_mode_puts_only_seven_bit_characters_on_the_terminal() {
    let mut session = Session::start(28, 100, &["--ascii"]);
    wait_for_the_handle_question(&mut session);
    play_the_opening(&mut session, "Zoë");
    session.send(b"help\r");
    session.wait_text("> help");
    session.settle(Duration::from_millis(100));
    for row in session.rows() {
        assert!(row.is_ascii(), "not ASCII: {row:?}");
    }
    let rows = session.rows();
    assert!(
        rows[1].starts_with("+Terminal-") && rows[1].contains("+Panel-"),
        "{rows:#?}"
    );
    assert!(
        rows[0].contains("Zoe"),
        "what is typed is transliterated: {:?}",
        rows[0]
    );
    assert!(
        session.contains("[#") || session.contains("[."),
        "an ASCII bar"
    );
}

#[test]
fn without_colour_the_terminal_gets_attributes_only() {
    let mut session = Session::start_with(
        28,
        100,
        &[],
        &Settings::Text("# test\n"),
        &[("NO_COLOR", "1")],
    );
    wait_for_the_handle_question(&mut session);
    play_the_opening(&mut session, "Neon");
    session.settle(Duration::from_millis(100));
    let cells = cells(&session);
    assert!(
        cells
            .iter()
            .all(|cell| cell.fgcolor() == vt100::Color::Default
                && cell.bgcolor() == vt100::Color::Default),
        "NO_COLOR means no colour on any cell"
    );
    assert!(
        cells.iter().any(vt100::Cell::inverse),
        "the status bar is reverse video"
    );
    assert!(cells.iter().any(vt100::Cell::dim), "the help bar is dim");
}

#[test]
fn with_colour_the_default_palette_uses_the_named_colours_of_the_terminal() {
    let mut session = at_command_line(28, 100);
    session.send(b"help\r");
    session.wait_text("> help");
    session.settle(Duration::from_millis(100));
    let cells = cells(&session);
    assert!(
        cells
            .iter()
            .any(|cell| matches!(cell.fgcolor(), vt100::Color::Idx(_)))
    );
    assert!(
        cells
            .iter()
            .all(|cell| !matches!(cell.fgcolor(), vt100::Color::Rgb(..))),
        "no exact colour without COLORTERM"
    );
    assert!(
        cells
            .iter()
            .all(|cell| cell.bgcolor() == vt100::Color::Default),
        "the default palette is transparent: the theme shows through"
    );
}
