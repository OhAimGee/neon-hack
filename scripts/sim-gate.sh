#!/usr/bin/env bash
# The quick balancing gate run on every pull request: plays the bots that carry a P threshold on
# a small set of games, then checks the reports. Exit code 1 when a P threshold fails.
#
#   scripts/sim-gate.sh [OUT_DIR]      (default: target/sim)
#
# Takes about a minute in a release build. The slow measures (the programs analysis, T9, three
# and a half minutes) and the objectives (O thresholds) are for the nightly run.
set -euo pipefail

out="${1:-target/sim}"
mkdir -p "$out"
cargo build --release --locked -p neon-sim
sim=target/release/neon-sim

# T1: the optimist finishes the representative plans, every ending is reached.
"$sim" run --bot optimist --difficulty all --seeds 1 --plans representative --out "$out/optimist.json"
# T2-T4: every site is won, how long it takes, how the margin falls with the difficulty.
"$sim" solve --difficulty all --out "$out/sites.json"
# T8: repeating a command pays nothing.
"$sim" run --bot farmer --difficulty normal --seeds 1 --plans representative --out "$out/farmer.json"
# T12: the monkey never panics, never traps the player, never breaks a save.
"$sim" run --bot monkey --seeds 1..=100 --out "$out/monkey.json"

"$sim" check "$out"/optimist-*.json "$out/sites.json" "$out/farmer.json" "$out/monkey.json"
