use automata::automaton::InfiniteWordAutomaton;
use automata::prelude::*;
use automata::random::{
    generate_random_dba, generate_random_dpa, generate_random_omega_words
};
use automata::math::set::IndexSet;

/// Generate a passive learning task with a [`DBA`] as target automaton. The target automaton is
/// drawn randomly for an alphabet of size `num_symbols` and the automaton has size up to `aut_size`.
/// For details about the random generation see documentation of [`generate_random_dba`].
/// A training set and a test set of random omega words (of sizes `training_size` and `test_size` respectively)
/// is drawn and labeled by running the words on the target automaton.
pub fn generate_dba_set(
    num_symbols: usize,
    aut_size: usize,
    training_size: usize,
    test_size: usize,
) -> (
    DBA,
    Vec<(ReducedOmegaWord<char>, bool)>,
    Vec<(ReducedOmegaWord<char>, bool)>,
) {
    let alphabet = CharAlphabet::of_size(num_symbols);
    // draw dba
    let dba = generate_random_dba(num_symbols, aut_size);
    // draw training and test set
    let len_spoke = 2 * ((aut_size as f64).log2().ceil() as usize) - 1;
    let len_cycle = (2 * aut_size - len_spoke) * len_spoke;
    let training_set =
        generate_random_omega_words(&alphabet, 0, len_spoke, 1, len_cycle, training_size);
    let test_set = generate_random_omega_words(&alphabet, 0, len_spoke, 1, len_cycle, test_size);
    // label sets
    let training_set: Vec<(ReducedOmegaWord<char>, bool)> = label_set(&dba, &training_set);
    let test_set: Vec<(ReducedOmegaWord<char>, bool)> = label_set(&dba, &test_set);

    (dba, training_set, test_set)
}

/// Generate a passive learning task with a [`DPA`] as target automaton. The target automaton is
/// drawn randomly for an alphabet of size `num_symbols` and the automaton has size up to `aut_size`.
/// For details about the random generation see documentation of [`generate_random_dpa`].
/// A training set and a test set of random omega words (of sizes `training_size` and `test_size` respectively)
/// is drawn and labeled by running the words on the target automaton.
pub fn generate_dpa_set(
    num_symbols: usize,
    aut_size: usize,
    num_prios: u8,
    training_size: usize,
    test_size: usize,
) -> (
    DPA,
    Vec<(ReducedOmegaWord<char>, bool)>,
    Vec<(ReducedOmegaWord<char>, bool)>,
) {
    let alphabet = CharAlphabet::of_size(num_symbols);
    // draw dpa
    let dpa = generate_random_dpa(num_symbols, aut_size, num_prios);
    // draw training and test set
    let len_spoke = 2 * ((aut_size as f64).log2().ceil() as usize) - 1;
    let len_cycle = (2 * aut_size - len_spoke) * len_spoke;
    let training_set =
        generate_random_omega_words(&alphabet, 0, len_spoke, 1, len_cycle, training_size);
    let test_set = generate_random_omega_words(&alphabet, 0, len_spoke, 1, len_cycle, test_size);
    // label sets
    let training_set: Vec<(ReducedOmegaWord<char>, bool)> = label_set(&dpa, &training_set);
    let test_set: Vec<(ReducedOmegaWord<char>, bool)> = label_set(&dpa, &test_set);

    (dpa, training_set, test_set)
}

/// Label a `set` of [`ReducedOmegaWord`]s with the result of the given automaton.
pub fn label_set<Z, C>(
    aut: &InfiniteWordAutomaton<CharAlphabet, Z, Void, C, true>,
    set: &IndexSet<ReducedOmegaWord<char>>,
) -> Vec<(ReducedOmegaWord<char>, bool)>
where 
    Z : OmegaSemantics<Void, C, Output = bool>,
    C: Color,
{
    set.into_iter()
        .map(|w| (w.clone(), aut.accepts(w)))
        .collect()
}
