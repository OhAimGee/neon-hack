# 🌆 Neon Hack

A cyberpunk text RPG for the terminal. You are a novice hacker in Neo-Tokyo, in 2087, up against the megacorporation Nexus Corp and its mysterious Project Aurora.

> An AI-generated game from 2025, left unfinished, then rebuilt once in C. It is now **being rewritten entirely in Rust, starting from scratch**.

[Français](README.md) · **English**

## Status

**The campaign is playable from a new game to the epilogue, but the game is not finished.** It is the game that runs by default (`neon-hack`): prologue, guided tutorial, hub commands, five endings and an epilogue, in English and French, with intrusions resolved automatically (the tactical engine is phase R4). The texts of chapters 1 and 2 are written; those of chapters 3 to 6, the endings and the epilogue are still `TODO <key>` drafts (502 per language). The engine demo is still available with `--demo`. The groundwork, the foundation (phase R1), the specifications (phase P1) and content and campaign (phase R2) are done; the full-screen interface (R3) has its core (R3.1: input line, scrolling, size tiers, terminal always given back); its complete side panel, screens and settings are lots R3.2 and R3.3.

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
| Phase R2.1: campaign content engine and data (quests, decisions, endings, validation, state saving) | done |
| Phase R2.2: command registry of the full game (R2.2a) and campaign core, hub commands, automatic intrusions, saving (R2.2b) | done |
| Phase R2.3: texts of lot L1 (chapters 1 and 2), prologue and guided tutorial | done |
| Phase R2.4: campaign end to end (optimistic player through the commands, 216 plans, five endings), campaign as the default game | done |
| Phase R2 (content and campaign) | **finished** |
| Phase R3.1: TUI core (choice of interface and first-launch question, input line with history and TAB, scrolling log, 64×20 and 100×28 tiers, terminal given back by every way out, tests in a pseudo-terminal) | done |
| Side panel, screens, frontend commands (R3.2 and R3.3), tactical run (R4), texts of chapters 3 to 6 (R5), balancing (R6) | to come |

## Play locally

> **The game is not finished**: the campaign is playable from start to end, but its texts for chapters 3 to 6 are drafts and intrusions are resolved automatically. There is no prebuilt binary before v1.0: you build the project yourself, which takes from a few tens of seconds to a few minutes the first time (downloading the dependencies included).

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

### 3. Run the game

```bash
cargo run --release -p neon-cli
```

The first time, Cargo downloads and builds the dependencies; later runs are immediate. To get a `neon-hack` command usable from anywhere:

```bash
cargo install --path crates/neon-cli --locked
neon-hack
```

`rustup` has already put `~/.cargo/bin` on your `PATH`. To update: with a clone, `git pull` then run the command above again (with `--force` for `cargo install`); with the ZIP, download it again. To uninstall: `cargo uninstall neon-cli`.

Without an option, `neon-hack` plays the **campaign**: it resumes the last game, or starts one. `neon-hack --demo` runs the engine demo instead, a small game with saves of its own (a save of one is never read by the other: the game says so).

### 4. How to play

In a real terminal that is big enough, the game opens **full screen** (status bar, log, side panel). If the input or the output is redirected, if the terminal is too small, or with `--plain`, it is played **line by line**. The very first time on a terminal, a question asks for the display mode (full screen, line by line, screen reader) and saves it in `settings.toml`.

- At the start: a handle (*Enter* keeps `Neon`), the prologue (*Enter* to continue, `skip` to jump ahead), then the offer of the **guided tutorial** (`y` or `n`).
- Goal: follow the **quests** of the journal (`quests`), talk to the contacts (`talk`), hack sites (`hack`), buy (`shop`, `buy`), keep your **Notoriety** low (`laylow`), and decide. `help` lists what is possible right now; `hint` asks ECHO-7 for a hint.
- Menus: type an entry's **number** or its name; `0` or an empty line goes back (*Esc* in full screen).
- Saving: the game is **saved automatically** and resumed at the next launch (`--new` to start over); `save` or `save 2` writes it to a slot (1 to 9); the game makes a **checkpoint** when each main quest opens, before a decision and before a purchase (the last 3 are kept). `neon-hack --list-saves` shows the folder and what it holds.
- Full screen: *TAB* completes the command or argument under the cursor (and lists what is still ambiguous), *Up*/*Down* recall the commands already typed, *Ctrl+A/E/U/K/W* and the arrows edit the line, *PgUp*/*PgDn* (and *Home*/*End* when the line is empty) scroll the log, *Esc* comes back down. *Ctrl+C* asks to quit, like the `quit` command; *Ctrl+D* on an empty line closes the input; one more key then closes the interface. A multi-line paste becomes a single line, which is never run without *Enter*. The mouse is not captured (the terminal's selection and copy keep working).
- The demo (`--demo`): buy the **deck upgrade** (120 credits) at R4Z0R's stall before the **trace** reaches 100 (`scan`, `laylow`, `shop`).

| Option | Effect |
|---|---|
| `--plain` | line-by-line interface, even on a terminal |
| `--tui` | full-screen interface, with a word on `stderr` if the terminal cannot show it |
| `--screen-reader` | screen-reader mode: line-by-line interface, no symbols to spell out |
| `--ascii` | 7-bit ASCII: symbols, accents and typed text transliterated, no decoration |
| `--lang fr` / `--lang en` | language of the texts (English by default) |
| `--verbosity brief\|normal\|full` | how much atmosphere is shown |
| `--demo` | play the engine demo instead of the campaign (separate saves, folder `saves/`) |
| `--difficulty story\|normal\|expert\|hardcore` | difficulty of a new campaign (a resumed one keeps its own) |
| `--seed 7` | reproducible game (for a new game) |
| `--color auto\|always\|never`, `--no-color` | colour: on by default for a terminal that can show it; `NO_COLOR` turns it off, and these options win over it |
| `--palette default\|high-contrast\|cvd\|mono` | colours of the full-screen interface |
| `--print-settings` | show the settings in effect and where each one comes from |
| `--new` | start over instead of resuming the autosave (the old one becomes `auto.toml.bak`) |
| `--load auto\|checkpoint-1\|slot-2` | resume that save (`checkpoint-1` to `checkpoint-3`, `slot-1` to `slot-9`) |
| `--list-saves` | list the saves and the folder they are in |
| `--no-save` | play without writing anything |
| `--data-dir DIR` | folder for the saves, instead of the system's |

The full-screen interface needs at least **64×20** characters, and **100×28** to show the permanent side panel. At launch, a smaller terminal gets the line-by-line display with one explanatory line on `stderr`; during a game, a terminal that shrinks only shows "Terminal too small" and the game stays intact until it is enlarged. A `TERM=dumb` terminal or screen-reader mode never opens the full screen.

Saves live in the user's data folder, in a `saves-campaign` subfolder (`saves` for the demo): `~/.local/share/neon-hack` on Linux, `~/Library/Application Support/neon-hack` on macOS, `%APPDATA%\neon-hack\data` on Windows. A damaged save is never overwritten: the game resumes from the previous copy (`.bak`) and says so, or explains how to start over.

### Settings

The presentation options (`lang`, `verbosity`, `ascii`, `screen_reader`, `display`) can be set once and for all in a `settings.toml` file in the data folder (the parent of the folder `--list-saves` shows, which is `saves-campaign`). The game only creates this file for the first-launch question (key `display`, plus `screen_reader` for screen-reader mode) and never modifies a file that exists; the rest is written by hand:

```toml
lang = "fr"
verbosity = "brief"
ascii = false
screen_reader = false
display = "auto"   # auto, tui or plain
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
