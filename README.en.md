# 🌆 Neon Hack

A cyberpunk text RPG for the terminal. You are a novice hacker in Neo-Tokyo, in 2087, up against the megacorporation Nexus Corp and its mysterious Project Aurora.

> An AI-generated game from 2025, left unfinished, then rebuilt once in C. It is now **being rewritten entirely in Rust, starting from scratch**.

[Français](README.md) · **English**

## Status

**The game is not playable yet**: the repository holds the Rust technical skeleton (three crates, CI, lint rules) but not the game engine yet. The groundwork, the foundation (phase R1) and the specifications (phase P1) are done; the content and campaign (phase R2) are the next step.

| Step | State |
|---|---|
| Scoping and design documents (game systems, narrative, TUI and accessibility, architecture, CI) | done: [`docs/design/`](docs/design/README.md); the detailed specifications are in [`docs/spec/`](docs/spec/README.md) and the design decisions are validated |
| v1.0 roadmap | written: [`docs/ROADMAP.md`](docs/ROADMAP.md) |
| Old C code and stray files | removed (see [Previous version](#previous-version-in-c)) |
| Phase R0: Cargo skeleton, CI on three systems | done |
| Phase R1.1: engine ↔ frontends contract, two interfaces (plain and full-screen), demo game | done |
| Phase R1.2: embedded TOML texts, plurals, strictly 7-bit `--ascii`, FR/EN parity checks | done |
| Phase R1.3: saving (autosave, checkpoints, slots, backup copy, format versions) | done |
| Phases R1.4a and R1.4b: command registry, settings (`settings.toml`, environment variables, three families of options) | done |
| Phase R1.4c: colours, `NO_COLOR`, palettes (`default`, `high-contrast`, `cvd`, `mono`) | done |
| Phase R1.5: complete toy game, end to end (goal, win, loss, epilogue) | done |
| Phase R1 (foundation) | **finished** |
| Phase P1: specifications (glossary, commands, mission language, run screen, solver, balancing harness) and validated design decisions | **finished**; next step: content and campaign (R2) |
| Game engine, TUI, campaign | to come |

## Play locally

> **The complete game does not exist yet.** What you can run today is the **engine demo**: a mini-game (prologue, commands, a stall, an alert gauge) that shows both interfaces. There is no prebuilt binary before v1.0: you build the project yourself, which takes from a few dozen seconds to a few minutes the first time (downloading the dependencies included).

### 1. Install the tools (once)

- **Git**, to download the project ([git-scm.com](https://git-scm.com)).
- **Rust**, through [rustup](https://rustup.rs) (on Linux and macOS: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`; on Windows: `rustup-init.exe`). The project's `rust-toolchain.toml` picks the right compiler version; if `cargo` says it is missing, run `rustup toolchain install`.
- **A linker**, which Rust uses to build the executable:
  - Windows: "Build Tools for Visual Studio", workload *Desktop development with C++* (`rustup-init.exe` offers it);
  - macOS: `xcode-select --install`;
  - Linux: `gcc` (for example `sudo apt install build-essential`).
- **A modern UTF-8 terminal**: Windows Terminal (not the old `cmd` console), Terminal or iTerm2 on macOS, any common Linux terminal. On Windows, WSL works too, but it is then a Linux environment: install Git, Rust and `gcc` **inside WSL**, following the Linux steps above (not the Build Tools), then run the commands in the WSL terminal.

### 2. Download the project

```bash
git clone https://github.com/OhAimGee/neon-hack.git
cd neon-hack
```

Without Git: on the project's GitHub page, *Code → Download ZIP*, unzip it, then open a terminal in the resulting folder.

### 3. Run the demo

```bash
cargo run --release -p neon-cli -- --demo
```

The first time, Cargo downloads and builds the dependencies; later runs are immediate. To get a `neon-hack` command usable from anywhere:

```bash
cargo install --path crates/neon-cli --locked
neon-hack --demo
```

`rustup` has already put `~/.cargo/bin` on your `PATH`. To update: with a clone, `git pull` then run the command above again (with `--force` for `cargo install`); with the ZIP, download it again. To uninstall: `cargo uninstall neon-cli`.

### 4. How to play

In a real terminal the demo opens **full screen** (status bar, log, side panel). If the input is redirected, or with `--plain`, it is played **line by line**.

- At the start: *Enter* to continue, a handle, then `y` or `n` to confirm.
- Goal: buy the **deck upgrade** (120 credits) at R4Z0R's stall before your **trace** reaches 100. `scan` earns credits but raises the trace; `laylow` brings it down; the cloak module at the stall makes it rise half as fast. Reaching 100 loses the game; buying the deck wins it. Either way, running the game again resumes just before the end (`--new` to start over).
- Commands: `help` lists what is possible; `status`, `scan`, `laylow`, `shop` (the stall), `save` and `quit`. There are shortcuts (`h`, `st`, `buy`, `exit`) and case does not matter.
- Menus: type an entry's **number** (in the stall, `proxy`, `cloak` and `deck` work too); `0` or an empty line goes back (*Esc* in full screen).
- Saving: the game is **saved automatically** and resumed at the next launch (`--new` to start over); `save` or `save 2` writes it to a slot (1 to 9); entering the stall makes a **checkpoint** (the last 3 are kept). `neon-hack --demo --list-saves` shows the folder and what it holds.
- Full screen: *TAB* completes a command, *Ctrl+D* or *Ctrl+C* ends the game, then one more key closes the interface.

| Option | Effect |
|---|---|
| `--plain` | line-by-line interface |
| `--screen-reader` | screen-reader mode: line-by-line interface, no symbols to spell out |
| `--ascii` | 7-bit ASCII: symbols, accents and typed text transliterated, no decoration |
| `--lang fr` / `--lang en` | language of the texts (English by default) |
| `--verbosity brief\|normal\|full` | how much atmosphere is shown |
| `--seed 7` | reproducible game (for a new game) |
| `--color auto\|always\|never`, `--no-color` | colour: on by default for a terminal that can show it; `NO_COLOR` turns it off, and these options win over it |
| `--palette default\|high-contrast\|cvd\|mono` | colours of the full-screen interface |
| `--print-settings` | show the settings in effect and where each one comes from |
| `--new` | start over instead of resuming the autosave (the old one becomes `auto.toml.bak`) |
| `--load auto\|checkpoint-1\|slot-2` | resume that save (`checkpoint-1` to `checkpoint-3`, `slot-1` to `slot-9`) |
| `--list-saves` | list the saves and the folder they are in |
| `--no-save` | play without writing anything |
| `--data-dir DIR` | folder for the saves, instead of the system's |

The full-screen interface needs at least **64×20** characters, and **100×28** to show the side panel; below that, a message says so: enlarge the window or use `--plain`.

Saves live in the user's data folder, in a `saves` subfolder: `~/.local/share/neon-hack` on Linux, `~/Library/Application Support/neon-hack` on macOS, `%APPDATA%\neon-hack\data` on Windows. A damaged save is never overwritten: the game resumes from the previous copy (`.bak`) and says so, or explains how to start over.

### Settings

The presentation options (`lang`, `verbosity`, `ascii`, `screen_reader`) can be set once and for all in a `settings.toml` file, to be written by hand in the data folder (the parent of the folder `--list-saves` shows, which is `saves`):

```toml
lang = "fr"
verbosity = "brief"
ascii = false
screen_reader = false
```

For each setting the most specific source wins: the command line, then the environment variable (`NEON_HACK_LANG` for the language), then `settings.toml`, then the system's configuration (`LANG` for the language; a locale that names a charset other than UTF-8, such as `fr_FR.ISO-8859-1`, turns `--ascii` on), then the default. `NEON_HACK_DATA_DIR` changes the data folder. For colours: `color` (`auto`, `always`, `never`) and `palette` can also be set in `settings.toml` (`NEON_HACK_PALETTE` for the palette); the standard `NO_COLOR` variable turns colour off (the `mono` palette only uses bold, underline and reverse video), unless `--color` is given; screen-reader mode never sends an escape sequence. `high-contrast` (white on black) and `cvd` (colours chosen for colour-vision deficiencies) paint their own background; their exact colours need a 24-bit terminal (`COLORTERM=truecolor`), otherwise the terminal approximates them. A damaged file is reported, never modified, and the defaults apply. `neon-hack --print-settings` shows the result.

### If something goes wrong

- `cargo: command not found`: close and reopen the terminal after installing Rust, or add `~/.cargo/bin` to your `PATH`.
- `linker 'cc' not found` or `link.exe not found`: install the linker (step 1).
- Frames, symbols or accents displayed wrongly: the terminal is probably not in UTF-8; try `--ascii`, which writes plain 7-bit ASCII only (accents become simple letters).
- An error about the Rust version: run `rustup toolchain install` in the project folder.

## Development

Requirement: [rustup](https://rustup.rs) (`rust-toolchain.toml` pins the compiler version).

```bash
cargo test --workspace      # unit, property and binary tests
cargo lint                  # clippy, exactly like the CI
cargo run -p neon-cli --    # runs the binary (a placeholder for now)
```

## v1.0 goals

1. **A complete campaign**: beginning, middle, end and epilogue, with choices that matter.
2. **A full-screen TUI** with a side panel, and an equivalent "plain" frontend (pipes, tests, screen readers).
3. **Accessibility built in from the first batch of work**: `NO_COLOR`, high-contrast palettes, a screen-reader mode, no time pressure.
4. Complete French and English; native Linux, macOS and Windows; published binaries.

## Documentation

The documents under `docs/` are written in French.

- [`docs/ROADMAP.md`](docs/ROADMAP.md): phases, risks, out of scope.
- [`docs/design/DECISIONS.md`](docs/design/DECISIONS.md): decisions made, decisions to validate.
- [`docs/design/README.md`](docs/design/README.md): index of the design documents and precedence rule.
- [`docs/design/CROSS-CHECK.md`](docs/design/CROSS-CHECK.md): contradictions between the documents and how they are resolved.

## Previous version in C

The C game has been removed from the repository, but its history is intact:

- last C state: commit `653bc46` (`git checkout 653bc46`, then `make`; its sources can also be read with `git show 653bc46:src/game/world.c`);
- original version, before any rebuild: tag `legacy-v2.087`.

## License

[CC0 1.0 Universal](LICENSE): public domain, you may copy, modify and redistribute the project without any condition.
