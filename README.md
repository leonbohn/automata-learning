# automata-learning

Rust library implementing a selection of learning algorithms for automata.
At the moment, it is mainly used as a testing ground for the `automata` crate and for me to implement some algorithms.
It would be to have a more complete list of implemented algorithms.


#### Sprout:
Learning DFA (actually Moore machines) from positive and negative examples or equivalently, learning Mealy machines from positive and negative example words can be done through the `sprout` method implemented in `passive/sprout.rs`.

