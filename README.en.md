# 🌆 Neon Hack

A cyberpunk text RPG for the terminal, written in C. You are a novice hacker in Neo-Tokyo, in 2087, up against the megacorporation Nexus Corp and its mysterious Project Aurora.

> An AI-generated game from 2025, left unfinished. It is **being rebuilt** into a game that can really be released (see [Status](#status)).

[Français](README.md) · **English**

The game is playable in French and in English (`--lang en`, or the *Language* entry of the menu). A few screens are still French-only: see [Status](#status). The documents under `docs/` are written in French.

## Status

The game **builds and runs**, but it is unfinished: part of the story is still to be written and advanced hacking has not been reworked yet. The details, checked by playing:

| Works | Does not work yet |
|---|---|
| Game loop, `scan`, `bruteforce`, `decrypt` | 5 contacts out of 9 have no profile yet (Shadow Broker, Neon Angel, Ghost Walker, Data Miner, Nexus Insider) |
| Levels and command unlocks | Quests 5 to 9 and the epilogue (not written yet) |
| **Quests**: the first four can be played from start to finish (tutorial, *Baptism of Fire*, *Information Networks*, *The Eye of the Storm*), `quests` journal in FR/EN, objectives measured on the game state, rewards paid only once, chapter announcements | `stealthmode` |
| **Alert 0-100** (gauge, cooldown, shop closed, game over, `laylow`); **levels 1-6** with an experience curve and unlocks; **unified world** (7 linked systems, `exploit`, one-time rewards) | A single save slot; the welcome screen is still in French only |
| **Launch menu** (continue, new game, language, options); **automatic saving**; **prologue and tutorial** led by ECHO-7, where you choose the hero's name ("Case" by default) | Completion of shop items, history search (Ctrl+R), history kept between sessions; tested on Linux only, and on native Windows the input falls back to a plain line read |
| **Input "like in a real terminal"**: TAB completion (available commands, discovered systems, contacts, messages), line editing, history (↑/↓) | General balancing (prices, caps, rewards: provisional values, to be tuned by simulation) |
| **R4Z0R's shop**: the whole catalogue fits on screen, in two columns if the terminal is wide enough; **all ten items have a real effect**, purchases are translated, and no source of credits or experience can be repeated endlessly | Complete English text (help, status, alert, world, quests, shop, contacts and messages are translated; advanced hacking and some hacking messages are not) |
| **Contacts and mail**: ECHO-7, R4Z0R, Phoenix and AURA unlock according to level, reputation and quests; menu-driven conversations, profiles, messages kept by the save | |
| End of input (Ctrl+D) and invalid input handled; `--seed`, `--fast`, `--lang`, automated tests | |
| Atmosphere, ASCII art, lore | |

The rebuild (unified architecture, tests, saving, French/English, binary releases) takes place on the `refonte/v1` branch. The original version can still be found under the `legacy-v2.087` tag. What comes next, phase by phase, up to v1.0 (complete campaign, cross-platform) is laid out in the [roadmap](docs/ROADMAP.md) (in French).

## Build and run

Requirements: `gcc` (or `clang`), `make`, a UTF-8 terminal with ANSI colors (Linux, macOS, or Windows through WSL).

```bash
make        # build
./neon_hack # start the game
make run    # build, then start
```

Options (`./neon_hack --help`):

| Option | Effect |
|---|---|
| `--seed N` | reproducible game (fixed random seed) |
| `--fast` | removes the animation pauses (automatic when the output is redirected) |
| `--lang fr\|en` | language (default: the last choice made in the menu, otherwise taken from `$LANG`); some screens are not translated yet |
| `--new` | start a new game right away, without the menu (the old save is replaced at the end of the prologue) |
| `--data-dir DIR` | folder for saves and settings (default: `$XDG_DATA_HOME/neon-hack`, otherwise `~/.local/share/neon-hack`; `%APPDATA%\neon-hack` on Windows) |
| `--no-hud` | disables the fixed bars (they only show up on a real terminal of at least 80×24 anyway) |
| `--no-color` | disables colors (help, status, prompt, shop, contacts and mail; not yet the original hacking screens) |
| `--version`, `--help` | version, help |

An option given on the command line (`--lang`, `--no-color`, `--color`, `--fast`, `--no-hud`) wins, for the session, over the settings saved by the menu; those in turn win over the environment (`$LANG`, `NO_COLOR`).

## Development

```bash
make test   # unit tests + end-to-end tests of the game; if python3 is available, also those of the interface,
            # the shop and the input, played in a real pseudo-terminal
make asan   # the same, built with AddressSanitizer + UBSan
```

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for how the code is organized and the rules of the rebuild, and [docs/ROADMAP.md](docs/ROADMAP.md) for the phases to come (both in French).

## Playing

On a terminal of at least 80×24, a **status bar** stays fixed at the top (name, level, credits, alert gauge) and a **command bar** at the bottom; the text scrolls between the two. The bars disappear by themselves if the window becomes too small, if the output is redirected, or with `--no-hud`.

**Input behaves like a real terminal's.** The **TAB key completes** what you type: for the first word, the commands you can use *right now* (the same as in `help`, never a locked command); after a command name, its argument: systems discovered by `scan` (`bruteforce`, `traceroute`…), unlocked contacts (`contact`), numbers of the messages received (`read`). If there is a single possibility, it is inserted. If there are several, the line advances up to their common part, and a second TAB lists them, as in bash. Case is ignored (`SCA` + TAB gives `scan `). You can also edit the line: `←` `→`, `Home`, `End`, `Delete`, `Ctrl+A` / `Ctrl+E` (start / end), `Ctrl+U` / `Ctrl+K` (erase before / after the cursor), `Ctrl+W` (erase the word); `↑` `↓` (or `Ctrl+P` / `Ctrl+N`) recall the last 32 lines. This only applies to the command prompt, and only on a real terminal: with redirected input (pipe, file, tests), lines are read as they are.

**At launch**, the menu offers: *Continue* (with the name, level and credits of the saved game), *New game*, *Language* (Français / English, applied immediately), *Options* (colors, text animations, fixed bars) and *Quit*. Language and options are remembered from one session to the next; Enter alone picks *Continue* if there is a save, *New game* otherwise.

**A new game** starts with a short prologue: your contact, **ECHO-7**, asks for your *handle*. This is how the hero's name is chosen (Enter for "Case", 20 characters at most, confirmation requested). ECHO-7 then offers a **tutorial**: a step-by-step mission inside the real game (`quests`, `help`, `scan`, `status`, reach level 2, `bruteforce localhost`, `laylow`). It comments on each action, reminds you of the objective if you stray (unknown, locked or failed command) and blocks nothing; `quests` shows the mission's progress. Completing it earns 100 ¢ and 10 reputation, once. You can also answer "I'll manage" and skip it (the tutorial is then closed without a reward).

**Quests** start by themselves as soon as their level and prerequisites are reached, and announce it on screen ("NEW QUEST", with a chapter announcement for the first quest of a chapter). They are measured on the game state — compromised systems, extracted files, purchases, contacts met, reputation, milestones — and not on counters: what you have already done counts. `quests` opens the journal; giving a quest's number shows its details (background, objectives, rewards). Some objectives are *conditions* checked at the moment you conclude ("keep the alert below 50": `laylow` lowers it); others are secret until completed. A completed quest pays its rewards (credits, reputation, experience) **only once** and may open another one. Four quests can be played for now; the following ones will come with phase 4 of the [roadmap](docs/ROADMAP.md).

**R4Z0R's shop** (`shop`) shows its whole catalogue at once, with no need to scroll back through the terminal history: ten items, each with its number, price, a short description and its state (available, level required, missing credits, sold out). From 72 terminal columns up, they are laid out in **two columns**; on a narrower or shorter terminal the layout tightens (compact cards, then a plain list) instead of overflowing. Type the item's number to buy it, `0` to leave. **Every item has a real effect**, announced when you buy it and visible in `status`: *Stealth Module* (+2 stealth, 10 at most), *Ghost Protocol* (a stock that the `laylow` menu consumes to lower the alert by 30), *Malware Arsenal* (the three viruses), *Proxy Chain Pro* (the alert rises by a third less for 5 hacks), *Quantum Encryption Key* (opens encrypted files of level 1 to 2), *Neural Assistant* (`aihack`, `aiassist`), *Quantum Processing Chip* (`quantumdecrypt`), *Street Cred Booster* (+20 reputation), *Neural Accelerator* (the experience of your next 3 gains is doubled, +30 at most per gain) and *Dark Web VPN* (the alert cools down faster on every hack). An item that would change nothing (already owned, stock full) is not sold: you are never charged for nothing. Nothing is handed out for free: the AI and the quantum computer have to be bought, and each level-up announces what R4Z0R now has for sale.

**Contacts and mail**: ECHO-7 can be reached from the start; **R4Z0R** (level 2, 10 reputation), **Phoenix** (level 4, 50 reputation, after *Information Networks*) and **AURA** (level 5, 50 reputation, after *The Eye of the Storm*) unlock by themselves and say so ("NEW CONTACT UNLOCKED"), with an introduction message. `contacts` lists them (number, relationship, what they can do) and the number typed at its prompt, or `contact <number or name>` (TAB completes), opens the conversation: a menu that comes back until "End the conversation" (or `0`, or an empty line), with advice, their comment on your level, the mission they gave you, their file, and, for R4Z0R, the shop. Every conversation builds trust. `messages` lists the inbox (new ones are marked), `read <number>` opens a message; the mail received and its "read" state are kept by the save.

**Saving**: the game is saved after every command (and by `quit` or `save`) in `savegame.sav`, a readable text file, written atomically: an interruption in the middle of a write leaves the previous save intact. A corrupted or truncated file, or one written by a newer version, is refused without half-loading anything. A lost game (alert at 100) does not overwrite the last save: *Continue* resumes just before the fatal command.

Type `help` in game: the help only lists the commands that are already unlocked (case and spaces ignored; `upload_virus`, `ai_hack`, `quantum_decrypt`, `exit` work too). When in doubt, TAB suggests what is possible.

- **Start**: `scan` (unlocked). The first 5 scans are worth 5, 4, 3, 2, 1 points: they lead to level 2 and unlock `bruteforce`; the following ones give nothing more. Experience then comes from hacks (each target pays only once).
- **Levels** (cumulative experience): 1 Novice · 2 Apprentice (15) · 3 Hacker (60) · 4 Expert (140) · 5 Master (260) · 6 Legend (420, maximum level, unlocks `temporalhack`). `status` shows your progress; each level-up announces the commands it unlocks.
- **Hacking**: `scan`, `bruteforce <target>`, `decrypt <text>`, `backdoor`, `traceroute`, `exploit`, `uploadvirus`, `aihack`.
- **Advanced hacking**: `advhack <target>`, `analyzedefenses`, `socialeng`, `aiassist`, `neuralsync`, `quantumdecrypt`, `temporalhack`.
- **The network**: 7 systems, revealed by `scan` according to your level and linked to one another. To attack a system you must have **compromised its relay** (`scan` marks `[NO ROUTE]`); `traceroute` names the relay and, once a system has been traced, `exploit` (level 4) breaks in directly. Each system pays only **once** (credits, experience, files): `socialeng` gives you inside access (+15% on all attacks). The `advhack` methods require a tool, active as soon as you own the equipment (quantum computer, AI assistant, virus, `neuralsync`…).
- **World**: `shop`, `laylow`, `quests`, `contacts`, `contact <number or name>`, `messages`, `read <number>`.
- **System**: `status`, `save`, `clear`, `quit`.

A message to decrypt, to try it out: `decrypt WKLV#LV#D#WHVW` (Caesar cipher, shift −3).

## Layout

```
src/main.c         entry point
src/game/          single game state, command table, event bus, quests, shop, contacts and mail, completion; advanced hacking (an original module) being ported
src/core/, ui/, i18n/   tested foundation (input/output, options, RNG, terminal, fixed interface, line input, FR/EN texts)
tests/             unit tests (unit/) and end-to-end tests (e2e/)
docs/              architecture and roadmap (in French); docs/legacy/ = reports generated at the time (unreliable)
```

## License

[CC0 1.0 Universal](LICENSE): public domain, you may copy, modify and redistribute the project without any condition.
