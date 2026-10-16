#!/usr/bin/env bash
# The drift of the nightly run: which indicators of the reports moved by more than a share since
# the night before. Information, never a failure (the exit code is 0 whatever it finds).
#
#   scripts/sim-drift.sh [OUT_DIR [PREVIOUS_DIR]]     (defaults: target/sim-nightly target/sim-previous)
#
# In CI (GITHUB_REPOSITORY and GH_TOKEN set, the `gh` CLI at hand) the reports of the night
# before are fetched from the newest `sim-nightly-*` artifact of the repository into PREVIOUS_DIR;
# anywhere else, PREVIOUS_DIR is whatever the caller put there (a downloaded artifact, say).
# PERCENT is the share, in percent of the previous value (default 5).
set -uo pipefail

out="${1:-target/sim-nightly}"
previous="${2:-target/sim-previous}"
percent="${PERCENT:-5}"
sim=target/release/neon-sim

if [ ! -x "$sim" ]; then
  echo "drift: no $sim to compare with"
  exit 0
fi

if [ -n "${GITHUB_REPOSITORY:-}" ] && [ -n "${GH_TOKEN:-}" ] && command -v gh >/dev/null; then
  rm -rf "$previous"
  # The artifact of the night before is the newest one that has not expired; the one of tonight
  # is not uploaded yet.
  found=$(gh api "repos/$GITHUB_REPOSITORY/actions/artifacts?per_page=100" --jq \
    '[.artifacts[] | select((.name | startswith("sim-nightly-")) and (.expired | not))]
     | sort_by(.created_at) | last // empty | "\(.workflow_run.id) \(.name)"') || found=""
  if [ -n "$found" ]; then
    gh run download "${found%% *}" --repo "$GITHUB_REPOSITORY" -n "${found#* }" -D "$previous" \
      || echo "drift: the artifact ${found#* } could not be downloaded"
  fi
fi

# The artifact holds the reports in `sim-nightly/` (the upload keeps the folder under `target/`).
if [ -d "$previous/sim-nightly" ]; then
  previous="$previous/sim-nightly"
fi
"$sim" drift --previous "$previous" --current "$out" --percent "$percent" || true
exit 0
