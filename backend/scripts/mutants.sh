#!/usr/bin/env bash
#
# Run cargo-mutants over the backend crate.
#
# Mutation testing changes the source in small ways (flipping a comparison,
# returning a default value, ...) and checks that some test fails. A mutant
# that survives points at behaviour no test asserts on.
#
# Configuration lives in backend/.cargo/mutants.toml. Among other things it
# makes sure tests/minimal.rs (which spawns a regtest bitcoind and takes
# minutes) is never part of a mutation run.
#
# Usage, from anywhere:
#
#   backend/scripts/mutants.sh              # full run, writes backend/mutants.out
#   backend/scripts/mutants.sh -f src/db.rs # only mutate one file
#   backend/scripts/mutants.sh -f src/db.rs -f src/rest.rs
#   backend/scripts/mutants.sh --in-diff <(git diff origin/master)  # only changed lines
#   backend/scripts/mutants.sh --list       # just list the mutants
#   backend/scripts/mutants.sh -j 8         # override the parallelism
#
# Needs the nix shell for a working toolchain:
#
#   nix-shell --run 'backend/scripts/mutants.sh'
#
# A full run takes a while (roughly 1-2 hours at -j 2). Results end up in
# backend/mutants.out: caught.txt, missed.txt, timeout.txt, unviable.txt and
# outcomes.json.

set -euo pipefail

cd "$(dirname "$(readlink -f "$0")")/.."

# cargo-mutants builds every mutant in its own copy of the source tree under
# $TMPDIR, and a small tmpfs /tmp fills up long before a run is done. Build on
# the big disk instead, unless the caller picked a $TMPDIR themselves. The nix
# shell points $TMPDIR at its own scratch directory under /tmp, which does not
# count as picking one.
case "${TMPDIR:-}" in
"" | */nix-shell-*)
    TMPDIR="$HOME/.cache/mainnet-observer-mutants"
    mkdir -p "$TMPDIR"
    export TMPDIR
    ;;
esac

# Default to -j 2 so a run doesn't swamp a shared machine. Raise it on an
# idle box. Only applied when the caller didn't pass -j / --jobs themselves.
jobs_args=(-j 2)
for arg in "$@"; do
    case "$arg" in
    -j | --jobs | -j* | --jobs=*)
        jobs_args=()
        break
        ;;
    esac
done

status=0
cargo mutants "${jobs_args[@]}" "$@" || status=$?

echo
echo "=== mutants summary ==="
for f in missed timeout unviable caught; do
    if [ -f mutants.out/$f.txt ]; then
        printf '%-10s %s\n' "$f" "$(wc -l <mutants.out/$f.txt)"
    fi
done
if [ -s mutants.out/missed.txt ]; then
    echo
    echo "missed mutants per file:"
    sed 's/:.*//' mutants.out/missed.txt | sort | uniq -c | sort -rn
    echo
    echo "full list: $(pwd)/mutants.out/missed.txt"
fi

exit $status
