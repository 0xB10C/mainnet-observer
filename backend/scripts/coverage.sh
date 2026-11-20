#!/usr/bin/env bash
#
# Collect test coverage for the backend crate with cargo-llvm-cov.
#
# Runs the library unit tests plus every integration test in backend/tests/
# except minimal.rs, which spawns a regtest bitcoind and takes minutes. Set
# WITH_BITCOIND=1 to include it as well.
#
# Writes backend/lcov.info and prints a per-file summary.
#
# Usage, from anywhere:
#
#   nix-shell --run 'backend/scripts/coverage.sh'
#   nix-shell --run 'WITH_BITCOIND=1 backend/scripts/coverage.sh'
#
# Pass --html to also write a browsable report to target/llvm-cov/html:
#
#   nix-shell --run 'backend/scripts/coverage.sh --html'
#
# Any other argument is passed through to the cargo llvm-cov run, e.g.
#
#   nix-shell --run 'backend/scripts/coverage.sh -- stats::'   # a subset
#
# The nix shell sets LLVM_COV and LLVM_PROFDATA; without them cargo-llvm-cov
# would look for the llvm-tools-preview rustup component, which doesn't exist
# in this setup.

set -euo pipefail

cd "$(dirname "$(readlink -f "$0")")/.."

# The lib unit tests, plus every tests/*.rs target. minimal.rs is skipped
# unless WITH_BITCOIND=1, everything else is picked up automatically so new
# test files are covered without touching this script.
targets=(--lib)
for test_file in tests/*.rs; do
    [ -e "$test_file" ] || continue
    name="$(basename "$test_file" .rs)"
    if [ "$name" = "minimal" ] && [ "${WITH_BITCOIND:-0}" != "1" ]; then
        echo "skipping tests/minimal.rs (set WITH_BITCOIND=1 to include it)"
        continue
    fi
    targets+=(--test "$name")
done

# --html is handled here instead of being passed through, because
# cargo-llvm-cov refuses --html together with --no-report. It becomes an extra
# report step over the same profile data below.
html=0
run_args=()
for arg in "$@"; do
    if [ "$arg" = "--html" ]; then
        html=1
    else
        run_args+=("$arg")
    fi
done

echo "coverage targets: ${targets[*]}"

# Run the tests once, then render the reports from the same profile data.
cargo llvm-cov --no-report --all-features "${targets[@]}" "${run_args[@]}"
cargo llvm-cov report --lcov --output-path lcov.info
echo
cargo llvm-cov report --summary-only

if [ "$html" = 1 ]; then
    cargo llvm-cov report --html
fi

echo
echo "lcov written to $(pwd)/lcov.info"
if [ "$html" = 1 ]; then
    echo "html written to ${CARGO_TARGET_DIR:-$(pwd)/target}/llvm-cov/html/index.html"
fi
