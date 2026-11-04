# automata-learning

> **Deprecated: this crate has moved into [`automata`](https://crates.io/crates/automata).**
> From `automata` 0.2.1 on, the code lives in the module `automata::learning`, behind the
> feature `learning`. This crate will not be developed further.
>
> | before | after |
> |--------|-------|
> | `automata-learning = "0.1"` | `automata = { version = "0.2.1", features = ["learning"] }` |
> | `automata_learning::passive::...` | `automata::learning::passive::...` |
> | `automata_learning::active::...` | `automata::learning::active::...` |
> | `automata_learning::passive::fwpm::FWPM` | `automata::families::FWPM` |
> | `fwpm.into_precise_dpa()` | `PreciseDPA::try_from(fwpm)` |
>
> Functions that used to panic on bad input return `Result`s with `automata::learning::LearningError`
> now. See the changelog of `automata` for the full migration notes.

Rust library implementing a selection of learning algorithms for automata.
At the moment, it is mainly used as a testing ground for the `automata` crate and for me to implement some algorithms.
It would be to have a more complete list of implemented algorithms.


#### Sprout:
Learning DFA (actually Moore machines) from positive and negative examples or equivalently, learning Mealy machines from positive and negative example words can be done through the `sprout` method implemented in `passive/sprout.rs`.

