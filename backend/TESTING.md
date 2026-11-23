# Testing the backend

All cargo commands need the nix shell from the repository root, which brings
the Rust toolchain, `sqlite`, a `bitcoind` binary for the integration test and
the coverage/mutation tooling:

```sh
nix-shell --run 'cd backend && cargo test'
```

## Running the tests

| command | what it runs | how long |
| --- | --- | --- |
| `cargo test --lib` | the unit tests in `src/` | ~20 s |
| `cargo test --lib --test cli --test db --test pipeline --test rest --test rest_pooling` | the unit tests plus every fast integration test | ~1 min |
| `cargo test` | everything, including `tests/minimal.rs` | several minutes |

The integration tests:

| file | what it covers |
| --- | --- |
| `tests/common/mod.rs` | no tests of its own: `MockCore`, an in-process stand-in for Bitcoin Core's REST interface, plus a temp directory and database helpers |
| `tests/cli.rs` | the command line binary, run as a real process |
| `tests/db.rs` | the queries and inserts in `src/db.rs` |
| `tests/pipeline.rs` | `collect_statistics()` end to end against `MockCore` |
| `tests/rest.rs` | the REST client and the serde structs the block JSON is parsed into |
| `tests/rest_pooling.rs` | that the REST client keeps its connections alive |
| `tests/minimal.rs` | a regtest `bitcoind` and the real code path, see below |

`tests/minimal.rs` spins up a regtest `bitcoind`, syncs a small chain through
the real code path and checks the resulting CSV files. It is the slow one; the
nix shell sets `BITCOIND_SKIP_DOWNLOAD=1` and `BITCOIND_EXE` so it uses the nix
`bitcoind` instead of downloading one.

Everything else talks to `MockCore` instead of a node and is fast enough to
run on every change.

## Coverage

```sh
nix-shell --run 'backend/scripts/coverage.sh'
```

`scripts/coverage.sh` runs `cargo llvm-cov` over the library unit tests and
every `tests/*.rs` target except `minimal.rs`, writes `backend/lcov.info` and
prints a per-file summary. Set `WITH_BITCOIND=1` to include `minimal.rs` as
well:

```sh
nix-shell --run 'WITH_BITCOIND=1 backend/scripts/coverage.sh'
```

`backend/scripts/coverage.sh --html` additionally renders a browsable report to
`backend/target/llvm-cov/html/`. Any other argument is passed through to the
`cargo llvm-cov` run.

There is no rustup here, so `cargo llvm-cov` cannot install the
`llvm-tools-preview` component. `shell.nix` instead points `LLVM_COV` and
`LLVM_PROFDATA` at the nixpkgs LLVM binaries, which match the LLVM version
rustc was built with.

## Mutation testing

```sh
nix-shell --run 'backend/scripts/mutants.sh'
```

`cargo-mutants` makes small changes to the source — flipping a comparison,
returning a default value, deleting a `!` — and checks that some test fails.
A mutant that survives ("missed") marks behaviour no test asserts on, which is
usually a more useful signal than a bare coverage percentage: a line can be
executed by a test without anything checking what it computes.

Useful invocations:

```sh
backend/scripts/mutants.sh -f src/db.rs                  # one file only
backend/scripts/mutants.sh --in-diff <(git diff origin/master)  # changed lines only
backend/scripts/mutants.sh --list                        # list mutants, run nothing
backend/scripts/mutants.sh -j 8                          # more parallelism
backend/scripts/mutants.sh -F 'performance_tune'         # only mutants whose name matches
```

`-F` matches the mutant names from `--list`, which is the quick way to check
whether a new test really kills the survivors it was written for, instead of
running the whole file again.

The script defaults to `-j 2` so a run does not swamp a shared machine. On an
idle box raise it — `-j 8` or higher is fine and scales close to linearly.

cargo-mutants builds every mutant in its own copy of the source tree under
`$TMPDIR`, which needs a few gigabytes per build job. The nix shell points
`$TMPDIR` at a directory on `/tmp`, and on a machine where `/tmp` is a small
tmpfs a run dies halfway through with "no space left on device". The script
therefore redirects `$TMPDIR` to `~/.cache/mainnet-observer-mutants` on the
big disk, unless the caller set a `$TMPDIR` of their own.

Results land in `backend/mutants.out/`: `caught.txt`, `missed.txt`,
`timeout.txt`, `unviable.txt` and `outcomes.json`. cargo-mutants moves the
previous run to `backend/mutants.out.old/`; both are git-ignored. The script
prints a summary and a per-file count of the missed mutants at the end.

Configuration is in `backend/.cargo/mutants.toml`. It

- skips `src/schema.rs`, which diesel generates from the migrations,
- skips the `Display` / `Error::source` / `From` error-plumbing impls, which
  only forward or format an inner error (this drops 22 of 629 mutants),
- skips two mutants in `collect_statistics()` that no test can see: the
  database batch size only decides how many rows go into one `INSERT`, and the
  counter it mutates next to it only ever reaches a log line,
- skips three mutants in `stats.rs` that are equivalent to the code they
  replace, so no test can ever kill them (each has a one-line reason there),
- passes `--lib --test cli --test db --test pipeline --test rest --test
  rest_pooling` to `cargo test`, so every fast test counts and the `bitcoind`
  integration test is never part of a mutation run, and
- allows 3x the baseline test time, never less than 60 s, so a loaded machine
  does not produce bogus timeouts.

A new `tests/*.rs` file has to be added to that list, otherwise its tests
can't catch anything.
