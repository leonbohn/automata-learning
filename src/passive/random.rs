use automata::prelude::*;
use automata::random::{
    generate_random_dba, generate_random_dpa, generate_random_omega_words
};
use automata::math::set::IndexSet;

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
    let training_set: Vec<(ReducedOmegaWord<char>, bool)> = dba_label_set(&dba, &training_set);
    let test_set: Vec<(ReducedOmegaWord<char>, bool)> = dba_label_set(&dba, &test_set);

    (dba, training_set, test_set)
}

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
    let training_set: Vec<(ReducedOmegaWord<char>, bool)> = dpa_label_set(&dpa, &training_set);
    let test_set: Vec<(ReducedOmegaWord<char>, bool)> = dpa_label_set(&dpa, &test_set);

    (dpa, training_set, test_set)
}

pub fn dba_label_set(
    aut: &DBA,
    set: &IndexSet<ReducedOmegaWord<char>>,
) -> Vec<(ReducedOmegaWord<char>, bool)> {
    set.into_iter()
        .map(|w| (w.clone(), aut.accepts(w)))
        .collect()
}

pub fn dpa_label_set(
    aut: &DPA,
    set: &IndexSet<ReducedOmegaWord<char>>,
) -> Vec<(ReducedOmegaWord<char>, bool)> {
    set.into_iter()
        .map(|w| (w.clone(), aut.accepts(w)))
        .collect()
}