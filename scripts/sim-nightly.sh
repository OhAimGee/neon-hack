#!/usr/bin/env bash
# The nightly balancing run: every bot at every difficulty, the sites, the economy and the programs
# analysis, then every threshold (P and O) is read. Exit code 1 when a P threshold fails; the
# objectives (O) are printed but never fail the run.
#
#   SEEDS=1..=20 PLANS=all scripts/sim-nightly.sh [OUT_DIR]      (default: target/sim-nightly)
#
# SEEDS is a number or a range of seeds for the games; PLANS is `all` (the 216 combinations of
# decisions) or `representative`. Takes about an hour with the defaults in a release build.
set -euo pipefail

out="${1:-target/sim-nightly}"
seeds="${SEEDS:-1}"
plans="${PLANS:-all}"
mkdir -p "$out"
cargo build --release --locked -p neon-sim
sim=target/release/neon-sim

for bot in optimist prudent reckless story farmer hoarder; do
  "$sim" run --bot "$bot" --difficulty all --seeds "$seeds" --plans "$plans" --out "$out/$bot.json"
done
"$sim" run --bot monkey --seeds 1..=500 --out "$out/monkey.json"
"$sim" solve --difficulty all --out "$out/sites.json"
"$sim" solve --difficulty all --loot all --out "$out/sites-all-loot.json"
"$sim" economy --out "$out/economy.json"
"$sim" programs --out "$out/programs.json"

# T13: the timings are not a canonical report (they hold durations): apart, not read by `check`.
mkdir -p "$out/timings"
"$sim" bench --out "$out/timings/bench.json"

# The thresholds decide the exit code; the games to replay come after them, whatever it is.
status=0
"$sim" check "$out"/*.json || status=$?

games=()
for report in "$out"/{optimist,prudent,reckless,story,farmer,hoarder,monkey}*.json; do
  [ -e "$report" ] && games+=(--report "$report")
done
echo
echo "Games to replay:"
"$sim" list-worst "${games[@]}"
exit "$status"
