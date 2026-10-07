# 🌆 Neon Hack

A cyberpunk text RPG for the terminal. You are a novice hacker in Neo-Tokyo, in 2087, up against the megacorporation Nexus Corp and its mysterious Project Aurora.

> An AI-generated game from 2025, left unfinished, then rebuilt once in C. It is now **being rewritten entirely in Rust, starting from scratch**.

[Français](README.md) · **English**

## Status

**The repository does not contain any Rust code yet: the game is not playable.** The design phase is finished and the rebuild is about to start.

| Step | State |
|---|---|
| Design (game systems, narrative, TUI and accessibility, architecture, CI) | done: [`docs/design/`](docs/design/README.md) |
| v1.0 roadmap | written: [`docs/ROADMAP.md`](docs/ROADMAP.md) |
| Old C code and stray files | removed (see [Previous version](#previous-version-in-c)) |
| Phase R0: Cargo skeleton, CI on three systems | **next step** |
| Engine, frontends, campaign | to come |

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
