#!/usr/bin/env bash
# The numbers of the human trial (docs/playtest/PROTOCOLE.md), read from the transcripts that the
# `export` command writes: typed lines, quests completed, hints, skips, loads, highest Notoriety.
#
#   scripts/playtest-summary.sh TRANSCRIPT...
#
# A transcript holds one session: pass them all, in the order they were played. The counts are
# comparable with the `commands` of the bots (neon-sim), not identical to them.
set -euo pipefail

if [ "$#" -eq 0 ]; then
  echo "usage: $0 TRANSCRIPT..." >&2
  exit 2
fi

total_lines=0
total_quests=0
total_hints=0
total_skips=0
total_loads=0
highest=0

for file in "$@"; do
  [ -f "$file" ] || { echo "$file: no such file" >&2; exit 2; }
  # The typed lines: the transcript puts `> ` before what the player typed.
  lines=$(grep -c '^> ' "$file" || true)
  quests=$(grep -c '^Quest completed: ' "$file" || true)
  hints=$(grep -c '^> hint\b' "$file" || true)
  skips=$(grep -c '^> skip\b' "$file" || true)
  loads=$(grep -c '^> load\b' "$file" || true)
  # The Notoriety, as `status` says it (`Notoriety: 31/100`) or as a change says it (`4 → 32`).
  top=$(grep 'Notoriety' "$file" \
    | sed -nE 's/.*Notoriety: ([0-9]+)\/100.*/\1/p; s/.*Notoriety \+[0-9]+ \([0-9]+ (→|->) ([0-9]+).*/\2/p' \
    | sort -n | tail -n 1)
  top=${top:-0}
  printf '%s: %s typed lines, %s quests, %s hints, %s skips, %s loads, highest Notoriety seen %s\n' \
    "$file" "$lines" "$quests" "$hints" "$skips" "$loads" "$top"
  total_lines=$((total_lines + lines))
  total_quests=$((total_quests + quests))
  total_hints=$((total_hints + hints))
  total_skips=$((total_skips + skips))
  total_loads=$((total_loads + loads))
  if [ "$top" -gt "$highest" ]; then highest=$top; fi
done

echo "total: $total_lines typed lines, $total_quests quests completed, $total_hints hints, $total_skips skips, $total_loads loads, highest Notoriety seen $highest"
echo "pace: divide the typed lines by the minutes of play (the spec's reference is 8 per minute)"
