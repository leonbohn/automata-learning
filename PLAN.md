# PLAN: automata-learning

- [ ] Pin or release `automata` and depend on a version instead of a git branch; follow `automata`'s API redesign (its `PLAN.md`, Phase 2).
- [ ] Remove the blanket `allow(unused)` and `allow(missing_docs)`; document all public items; cite the algorithm each module implements (RPNI/sprout, DPAInf, L*).
- [ ] Un-ignore or delete the 3 `#[ignore]`d tests; add integration tests in `tests/` with realistic samples, plus property tests (learned automaton is consistent with the sample).
- [ ] README: scope, feature table of algorithms (passive/active, DFA/Moore/Mealy/omega), runnable examples.
- [ ] Add `rust-version` and a CI matrix (Linux/macOS/Windows); the current workflow only builds on Windows and macOS.
- [ ] Re-add benchmarks (`criterion`) for the sprout and L* hot paths.
- [ ] Decide on publishing to crates.io once `automata` is released.
