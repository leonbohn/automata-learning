# CLAUDE.md

`automata-learning` implements learning algorithms for automata on top of the `automata` crate (`leonbohn/automata`, from crates.io): passive learning from samples and active learning with membership/equivalence queries. Edition 2024, version 0.1.0, MIT. Mostly research code.

## History

This repository's history is the original standalone history (to June 2024, including the `sprout` and `random-samples` branches on the remote) followed by one import commit with the crate's state at the time it joined the `leonbohn/frieda` monorepo, followed by the non-merge commits made there (replayed; manifest conflicts resolved in favor of the monorepo). A final commit makes the crate standalone again.

## Commands

```shell
cargo build --all-targets
cargo test                         # 22 pass, 3 ignored
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
./scripts/check
```

## Layout

- `src/passive/`: `sample/` (positive/negative word samples, `characterize`), `sprout/` (consistency checks and the sprout algorithm, learns Moore/Mealy machines from samples), `dpainf` (DPA from omega samples), `precise`, `fwpm`.
- `src/active/`: `observationtable`, `lstar`, `oracle`, `hypothesis`, `mealy`, `moore`.
- `src/priority_mapping.rs`: annotated congruences and weak priority mappings used by the passive omega learners.
- `src/prefixtree.rs`: internal prefix tree.

The crate uses `automata::alphabet`, `automata::word`, `automata::math` etc. and the `hoa`, `render`, `graphviz` features of `automata`.

## Known sharp edges

- `lib.rs` has blanket `allow(missing_docs)` and `allow(unused)`; much code is undocumented.
- The README is a stub; algorithms are not documented with references.
- The `automata` dependency is the released `0.1` from crates.io. To develop against an unreleased `automata`, use a `[patch.crates-io]` entry with a local path (e.g. in `.cargo/config.toml`).
