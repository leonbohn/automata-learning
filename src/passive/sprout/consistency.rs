use itertools::{Either, Itertools, all};
use std::collections::HashMap;
use std::iter;
use std::ops::Not;

use crate::prefixtree::prefix_tree;
use automata::automaton::{BuchiCondition, DBA, DPA, MinEvenParityCondition};
use automata::core::Void;
use automata::core::alphabet::{Alphabet, CharAlphabet};
use automata::core::math::OrderedSet;
use automata::representation::CollectTs;
use automata::ts::run::InfiniteRunOutput::Successful;
use automata::ts::{Deterministic, EdgeColor, Sproutable, run};
use automata::{
    Pointed, TransitionSystem,
    ts::path::{self, LassoIn},
};

use super::OmegaSample;

type EdgeSet = OrderedSet<(u32, char)>;

/// Used to define consistency checks on various types of omega acceptance conditions
/// required by the sprout algorithm for passively learning omega automata
pub trait ConsistencyCheck<T> {
    /// the type of the automaton to be returned
    type Aut: TransitionSystem;
    /// Checks if the given transition system is consistent with the sample
    fn consistent(
        &self,
        ts: &T,
        sample: &OmegaSample,
        pos_sets: Vec<EdgeSet>,
        neg_sets: Vec<EdgeSet>,
    ) -> (bool, Vec<EdgeSet>, Vec<EdgeSet>);
    /// If the transition system is consistent with the sample,
    /// returns an automaton with underlying transition system ts
    /// that is consistent with the sample
    fn consistent_automaton(
        &self,
        ts: &T,
        sample: &OmegaSample,
        pos_sets: Vec<EdgeSet>,
        neg_sets: Vec<EdgeSet>,
    ) -> Self::Aut;
    /// Automaton that accepts precisely the positive example words
    /// in case no other solution can be found
    fn default_automaton(&self, sample: &OmegaSample) -> Self::Aut;
}

impl<T> ConsistencyCheck<T> for BuchiCondition
where
    T: TransitionSystem<Alphabet = CharAlphabet, StateIndex = u32> + Deterministic + Pointed,
    <T as TransitionSystem>::EdgeColor: Eq + std::hash::Hash,
{
    type Aut = DBA;
    fn consistent(
        &self,
        ts: &T,
        sample: &OmegaSample,
        pos_sets: Vec<EdgeSet>,
        neg_sets: Vec<EdgeSet>,
    ) -> (bool, Vec<EdgeSet>, Vec<EdgeSet>) {
        if let Some([mut pos_sets_new, mut neg_sets_new]) = to_infinity_sets(ts, sample) {
            // check if the infinity set of a positive word is subset of
            // the union of all infinity sets of negative words (see paper for details)
            neg_sets_new.extend(neg_sets);
            let neg_union: EdgeSet = neg_sets_new.iter().flatten().cloned().collect();
            pos_sets_new.extend(pos_sets);

            let is_consistent = pos_sets_new.iter().any(|s| s.is_subset(&neg_union)).not();
            (is_consistent, pos_sets_new, neg_sets_new)
        } else {
            // bad pair was found when running sample words on transition system
            (false, pos_sets, neg_sets)
        }
    }

    fn consistent_automaton(
        &self,
        ts: &T,
        sample: &OmegaSample,
        pos_sets: Vec<EdgeSet>,
        neg_sets: Vec<EdgeSet>,
    ) -> Self::Aut {
        // check consistency
        assert!(self.consistent(ts, sample, pos_sets, neg_sets.clone()).0);

        // derive acceptance condition: accepting transitions
        // -> all transitions besides the union of negative infinity sets
        let [_, mut neg_sets_new] =
            to_infinity_sets(ts, sample).expect("ts cannot be consistent with sample");
        neg_sets_new.extend(neg_sets);

        let neg_union: EdgeSet = neg_sets_new.iter().flatten().cloned().collect();

        let all_transitions: EdgeSet = ts
            .state_indices()
            .cartesian_product(ts.alphabet().universe())
            .collect();

        let accepting: EdgeSet = all_transitions.difference(&neg_union).cloned().collect();

        // make DBA
        let mut dba = ts
            .map_edge_colors_full(move |a, b, _, _| accepting.contains(&(a, *b)))
            .erase_state_colors()
            .collect_dba();

        // complete with sink state
        dba.complete_with_colors(Void, false);
        dba
    }

    fn default_automaton(&self, sample: &OmegaSample) -> Self::Aut {
        let mut dba = prefix_tree(sample.alphabet().clone(), sample.positive_words())
            .map_edge_colors(|_| true)
            .erase_state_colors()
            .with_initial(0)
            .collect_dba();
        dba.complete_with_colors(Void, false);
        dba
    }
}

impl<T> ConsistencyCheck<T> for MinEvenParityCondition
where
    T: TransitionSystem<Alphabet = CharAlphabet, StateIndex = u32> + Deterministic + Pointed,
    <T as TransitionSystem>::EdgeColor: Eq + std::hash::Hash,
{
    type Aut = DPA;

    fn consistent(
        &self,
        ts: &T,
        sample: &OmegaSample,
        pos_sets: Vec<EdgeSet>,
        neg_sets: Vec<EdgeSet>,
    ) -> (bool, Vec<EdgeSet>, Vec<EdgeSet>) {
        if let Some([mut pos_sets_new, mut neg_sets_new]) = to_infinity_sets(ts, sample) {
            pos_sets_new.extend(pos_sets);
            neg_sets_new.extend(neg_sets);
            // check how set with all transitions should be handled
            let all_transitions: EdgeSet = ts
                .state_indices()
                .cartesian_product(ts.alphabet().universe())
                .collect();
            let is_consistent = match (
                pos_sets_new.contains(&all_transitions),
                neg_sets_new.contains(&all_transitions),
            ) {
                (true, false) => {
                    // set of all transitions is accepting
                    has_zielonka_path(&pos_sets_new, &neg_sets_new, &all_transitions, true)
                }
                (false, true) => {
                    // set of all transitions is non-accepting
                    has_zielonka_path(&pos_sets_new, &neg_sets_new, &all_transitions, false)
                }
                (false, false) => {
                    // class of set of all transitions not clear, check both options
                    has_zielonka_path(&pos_sets_new, &neg_sets_new, &all_transitions, false)
                        || has_zielonka_path(&pos_sets_new, &neg_sets_new, &all_transitions, true)
                }
                (true, true) => {
                    // set of all transitions is both accepting and non-accepting
                    // no Zielonka path possible
                    false
                }
            };
            (is_consistent, pos_sets_new, neg_sets_new)
        } else {
            // bad pair was found when running sample words on transition system
            (false, pos_sets, neg_sets)
        }
    }

    fn consistent_automaton(
        &self,
        ts: &T,
        sample: &OmegaSample,
        pos_sets: Vec<EdgeSet>,
        neg_sets: Vec<EdgeSet>,
    ) -> Self::Aut {
        // check consistency
        assert!(
            self.consistent(ts, sample, pos_sets.clone(), neg_sets.clone())
                .0
        );

        let [mut pos_sets_new, mut neg_sets_new] =
            to_infinity_sets(ts, sample).expect("ts cannot be consistent with sample");

        pos_sets_new.extend(pos_sets);
        neg_sets_new.extend(neg_sets);

        let all_transitions: EdgeSet = ts
            .state_indices()
            .cartesian_product(ts.alphabet().universe())
            .collect();
        let z_path: Vec<EdgeSet>;
        let lowest: bool;
        match (
            pos_sets_new.contains(&all_transitions),
            neg_sets_new.contains(&all_transitions),
        ) {
            (true, false) => {
                // set of all transitions is accepting
                z_path = zielonka_path(pos_sets_new, neg_sets_new, all_transitions, true).unwrap();
                lowest = true;
            }
            (false, true) => {
                // set of all transitions is non-accepting
                z_path = zielonka_path(pos_sets_new, neg_sets_new, all_transitions, false).unwrap();
                lowest = false;
            }
            (false, false) => {
                // class of set of all transitions not clear, check both options
                let res1 = zielonka_path(
                    pos_sets_new.clone(),
                    neg_sets_new.clone(),
                    all_transitions.clone(),
                    false,
                );
                let res2 = zielonka_path(pos_sets_new, neg_sets_new, all_transitions, true);
                match (res1, res2) {
                    (Some(path1), Some(path2)) => {
                        // both work, select smaller one
                        (z_path, lowest) = if path1.len() >= path2.len() {
                            (path2, true)
                        } else {
                            (path1, false)
                        };
                    }
                    (Some(path1), None) => (z_path, lowest) = (path1, false),
                    (None, Some(path2)) => (z_path, lowest) = (path2, true),
                    _ => panic!("No valid Zielonka Path exists. Not consistent."),
                }
            }
            (true, true) => {
                // this shouldn't happen, pos and neg induce same infinity set
                panic!(
                    "Set of all transitions is both accepting and non-accepting. Transition system not consistent."
                );
            }
        }
        // build dpa from Zielonka path

        let mut prio_map: HashMap<(u32, char), u8> = HashMap::new();
        let mut prio = if lowest { 0 } else { 1 };
        for i in 0..z_path.len() - 1 {
            for t in z_path[i].difference(&z_path[i + 1]) {
                prio_map.insert(*t, prio);
            }
            prio += 1;
        }
        let mut dpa = ts
            .map_edge_colors_full(move |a, b, c, d| {
                *prio_map
                    .get(&(a, *b))
                    .expect("transition missing in Zielonka path")
            })
            .erase_state_colors()
            .collect_dpa();

        // complete with sink state
        let max_prio = if prio % 2 == 0 { prio - 1 } else { prio };
        dpa.complete_with_colors(Void, max_prio);
        dpa
    }

    fn default_automaton(&self, sample: &OmegaSample) -> Self::Aut {
        let mut dpa = prefix_tree(sample.alphabet().clone(), sample.positive_words())
            .map_edge_colors(|_| 0)
            .erase_state_colors()
            .with_initial(0)
            .collect_dpa();
        dpa.complete_with_colors(Void, 1);
        dpa
    }
}

/// Check if it is possible to construct a valid zielonka path from the given classified sets.
/// `class` is the classification to use for the set of all transitions.
fn has_zielonka_path(
    pos_sets: &Vec<EdgeSet>,
    neg_sets: &Vec<EdgeSet>,
    all_transitions: &EdgeSet,
    mut class: bool,
) -> bool {
    // check if class of set with all transitions is valid
    if class {
        assert!(!neg_sets.contains(all_transitions));
    } else {
        assert!(!pos_sets.contains(all_transitions));
    }
    let mut pos_sets = pos_sets.to_owned();
    let mut neg_sets = neg_sets.to_owned();

    let mut z = all_transitions.clone();
    while !z.is_empty() {
        // set new Z to union of subsets with different classification
        let z_new: EdgeSet = if class {
            // Z accepting
            neg_sets.retain(|s| s.is_subset(&z));
            neg_sets.iter().flatten().cloned().collect()
        } else {
            // Z non-accepting
            pos_sets.retain(|s| s.is_subset(&z));
            pos_sets.iter().flatten().cloned().collect()
        };
        if z == z_new {
            return false;
        }
        z = z_new;
        class = !class;
    }
    true
}

/// For given sets compute Zielonka path consistent with given classification.
/// `class` is the classification of the set of all transitions
/// returns `None` if no consistent Zielonka path exists
fn zielonka_path(
    mut pos_sets: Vec<EdgeSet>,
    mut neg_sets: Vec<EdgeSet>,
    all_transitions: EdgeSet,
    mut class: bool,
) -> Option<Vec<EdgeSet>> {
    // check if class of set with all transitions is valid
    if class {
        assert!(!neg_sets.contains(&all_transitions));
    } else {
        assert!(!pos_sets.contains(&all_transitions));
    }

    let mut z_path = vec![all_transitions];
    let mut i = 0;
    while !z_path[i].is_empty() {
        // set new Z to union of subsets with different classification
        let z_new: EdgeSet = if class {
            // Z accepting
            neg_sets.retain(|s| s.is_subset(&z_path[i]));
            neg_sets.iter().flatten().cloned().collect()
        } else {
            // Z non-accepting
            pos_sets.retain(|s| s.is_subset(&z_path[i]));
            pos_sets.iter().flatten().cloned().collect()
        };
        if z_path[i] == z_new {
            return None;
        }
        z_path.push(z_new);
        class = !class;
        i += 1;
    }
    Some(z_path)
}

use run::InfiniteRunOutput::*;
/// Run positive and negative sample words on the given transition system.
/// If there is a pair of words escaping with the same escape string from the same state, return None.
/// Otherwise return non-escaping runs of positive and negative example words
fn to_infinity_sets<T>(ts: T, sample: &OmegaSample) -> Option<[Vec<EdgeSet>; 2]>
where
    T: TransitionSystem<Alphabet = CharAlphabet, StateIndex = u32> + Deterministic + Pointed,
    <T as TransitionSystem>::EdgeColor: Eq + std::hash::Hash,
{
    let time_start = std::time::Instant::now();
    // run transition system on sample words and
    // separate in escaping and non-escaping (successful) runs
    let (pos_successful, pos_escaping): (Vec<_>, Vec<_>) = sample
        .positive_words()
        .map(|w| (ts.omega_run::<_, run::Triggers<_>>(w), w))
        .partition_map(|(r, w)| match r {
            (Successful(v)) => Either::Left(v),
            (Failed(q, ep)) => Either::Right((q, ep.suffix())),
        });

    let mut neg_successful = Vec::default();
    for (output, w) in sample
        .negative_words()
        .map(|w| (ts.omega_run::<_, run::Triggers<_>>(w), w))
    {
        match output {
            Successful(v) => neg_successful.push(v),
            Failed(q, ep) => {
                if pos_escaping.contains(&(q, ep.suffix())) {
                    return None;
                }
            }
        }
    }
    tracing::debug!("successful_runs took {}", time_start.elapsed().as_micros());
    Some([pos_successful, neg_successful])
}

// /// Run positive and negative sample words on the given transition system.
// /// If there is a pair of words escaping with the same escape string from the same state, return None.
// /// Otherwise return vector of sets of transitions visited infinitely often
// /// during the runs of positive and negative example words.
// fn to_infinity_sets<T>(ts: T, sample: &OmegaSample) -> Option<[Vec<EdgeSet>; 2]>
// where
//     T: TransitionSystem<Alphabet = CharAlphabet, StateIndex = u32> + Deterministic + Pointed,
//     <T as TransitionSystem>::EdgeColor: Eq + std::hash::Hash,
// {
//     if let Some([pos_successful, neg_successful]) = successful_runs(ts, sample) {
//         let pos_sets = pos_successful
//             .into_iter()
//             .map(|run| {
//                 run.into_recurrent_transitions()
//                     .map(|e| {
//                         let (src, &sym, _, _) = e.into_tuple();
//                         (src, sym)
//                     })
//                     .collect()
//             })
//             .collect();
//         let neg_sets = neg_successful
//             .into_iter()
//             .map(|run| {
//                 run.into_recurrent_transitions()
//                     .map(|e| {
//                         let (src, &sym, _, _) = e.into_tuple();
//                         (src, sym)
//                     })
//                     .collect()
//             })
//             .collect();

//         Some([pos_sets, neg_sets])
//     } else {
//         None
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::passive::OmegaSample;
    use automata::automaton::{BuchiCondition, MinEvenParityCondition, WithInitial};
    use automata::core::alphabet::CharAlphabet;
    use automata::core::{Void, upw};
    use automata::{
        DTS,
        automaton::{DeterministicOmegaAutomaton, OmegaAcceptanceCondition},
    };

    // default alphabet
    fn sigma() -> CharAlphabet {
        CharAlphabet::of_size(2)
    }

    #[test]
    fn both_escaping() {
        // build transition systems
        let ts = DTS::builder()
            .with_transitions([(0, 'a', Void, 1)])
            .default_color(Void)
            .into_dts_with_initial(0);
        let ts2 = DTS::builder()
            .with_transitions([(0, 'a', Void, 0)])
            .default_color(Void)
            .into_dts_with_initial(0);

        // build samples
        let sample1 = OmegaSample::new_omega_from_pos_neg(sigma(), [upw!("a")], [upw!("b")]);
        let sample2 = OmegaSample::new_omega_from_pos_neg(sigma(), [upw!("a")], [upw!("a", "b")]);
        let sample3 = OmegaSample::new_omega_from_pos_neg(sigma(), [upw!("a", "b")], [upw!("b")]);

        // words escape from different states
        assert!(BuchiCondition.consistent(&ts, &sample1, vec![], vec![]).0);
        assert!(
            MinEvenParityCondition
                .consistent(&ts, &sample1, vec![], vec![])
                .0
        );
        // words escape from same state but with different exit strings
        assert!(BuchiCondition.consistent(&ts, &sample2, vec![], vec![]).0);
        assert!(
            MinEvenParityCondition
                .consistent(&ts, &sample2, vec![], vec![])
                .0
        );
        // words escape from same state with same exit string
        assert!(!BuchiCondition.consistent(&ts2, &sample3, vec![], vec![]).0);
        assert!(
            !MinEvenParityCondition
                .consistent(&ts2, &sample3, vec![], vec![])
                .0
        );
    }

    #[test]
    fn one_escaping() {
        // build transition system
        let ts = DTS::builder()
            .with_transitions([(0, 'a', Void, 0)])
            .default_color(Void)
            .into_dts_with_initial(0);

        // build sample
        let sample = OmegaSample::new_omega_from_pos_neg(sigma(), [upw!("a")], [upw!("b")]);

        // one word is escaping, the other is not
        assert!(BuchiCondition.consistent(&ts, &sample, vec![], vec![]).0);
        assert!(
            MinEvenParityCondition
                .consistent(&ts, &sample, vec![], vec![])
                .0
        );
    }

    #[test]
    fn buchi_consistency() {
        // build transition systems
        let ts = DTS::builder()
            .with_transitions([(0, 'b', Void, 0), (0, 'a', Void, 1), (1, 'b', Void, 1)])
            .default_color(Void)
            .into_dts_with_initial(0);
        let ts2 = DTS::builder()
            .with_transitions([(0, 'b', Void, 0), (0, 'a', Void, 1), (1, 'a', Void, 0)])
            .default_color(Void)
            .into_dts_with_initial(0);
        let ts3 = DTS::builder()
            .with_transitions([(0, 'a', Void, 0), (0, 'b', Void, 0)])
            .default_color(Void)
            .into_dts_with_initial(0);

        // build samples
        let sample1 = OmegaSample::new_omega_from_pos_neg(sigma(), [upw!("a", "b")], [upw!("b")]);
        let sample2 = OmegaSample::new_omega_from_pos_neg(sigma(), [upw!("b")], [upw!("aab")]);
        let sample3 = OmegaSample::new_omega_from_pos_neg(sigma(), [upw!("aab")], [upw!("b")]);
        let sample4 = OmegaSample::new_omega_from_pos_neg(sigma(), [upw!("a")], [upw!("b")]);

        assert!(BuchiCondition.consistent(&ts, &sample1, vec![], vec![]).0);
        assert!(!BuchiCondition.consistent(&ts2, &sample2, vec![], vec![]).0);
        assert!(BuchiCondition.consistent(&ts2, &sample3, vec![], vec![]).0);
        assert!(BuchiCondition.consistent(&ts3, &sample4, vec![], vec![]).0);
    }

    #[test]
    fn buchi_consistent_automaton() {
        // build transition system
        let ts = DTS::builder()
            .with_transitions([(0, 'b', Void, 0), (0, 'a', Void, 1), (1, 'b', Void, 1)])
            .default_color(Void)
            .into_dts_with_initial(0);

        // build sample
        let sample = OmegaSample::new_omega_from_pos_neg(sigma(), [upw!("a", "b")], [upw!("b")]);
        let sample2: OmegaSample = OmegaSample::new_for_alphabet(sigma());

        // build automaton
        let dba = DTS::builder()
            .with_transitions([
                (0, 'a', true, 1),
                (0, 'b', false, 0),
                (1, 'a', false, 2),
                (1, 'b', true, 1),
                (2, 'a', false, 2),
                (2, 'b', false, 2),
            ])
            .default_color(Void)
            .into_dba(0);

        let res = BuchiCondition.consistent_automaton(&ts, &sample, vec![], vec![]);
        let pos_sets: Vec<EdgeSet> = vec![vec![(1, 'b')].into_iter().collect()];
        let neg_sets: Vec<EdgeSet> = vec![vec![(0, 'b')].into_iter().collect()];
        let res2 = BuchiCondition.consistent_automaton(&ts, &sample2, pos_sets, neg_sets);

        assert_eq!(res, dba);
        assert_eq!(res2, dba);
    }

    #[test]
    fn parity_consistency() {
        // build transition systems
        let ts = DTS::builder()
            .with_transitions([(0, 'a', Void, 0)])
            .default_color(Void)
            .into_dts_with_initial(0);
        let ts2 = DTS::builder()
            .with_transitions([(0, 'a', Void, 0), (0, 'b', Void, 0)])
            .default_color(Void)
            .into_dts_with_initial(0);
        let ts3 = DTS::builder()
            .with_transitions([
                (0, 'a', Void, 0),
                (0, 'b', Void, 1),
                (1, 'a', Void, 0),
                (1, 'b', Void, 0),
            ])
            .default_color(Void)
            .into_dts_with_initial(0);
        let ts4 = DTS::builder()
            .with_transitions([(0, 'a', Void, 0), (0, 'b', Void, 0), (0, 'c', Void, 0)])
            .default_color(Void)
            .into_dts_with_initial(0);
        let ts5 = DTS::builder()
            .with_transitions([
                (0, 'a', Void, 0),
                (0, 'b', Void, 1),
                (1, 'a', Void, 0),
                (1, 'b', Void, 1),
            ])
            .default_color(Void)
            .into_dts_with_initial(0);

        // build samples
        let sample1 = OmegaSample::new_omega_from_pos_neg(sigma(), [upw!("a")], [upw!("b")]);
        let sample2 = OmegaSample::new_omega_from_pos_neg(sigma(), [upw!("a")], [upw!("ab")]);
        let sample3 =
            OmegaSample::new_omega_from_pos_neg(sigma(), [upw!("a"), upw!("b")], [upw!("ab")]);
        let sample4 = OmegaSample::new_omega_from_pos_neg(
            sigma(),
            [upw!("ababb")],
            [upw!("ba"), upw!("bba")],
        );
        let sample5 = OmegaSample::new_omega_from_pos_neg(
            CharAlphabet::of_size(3),
            [upw!("a"), upw!("b"), upw!("c")],
            [upw!("ab")],
        );
        let sample6 = OmegaSample::new_omega_from_pos_neg(
            CharAlphabet::of_size(3),
            [upw!("ab"), upw!("b"), upw!("c")],
            [upw!("a")],
        );
        let sample7 = OmegaSample::new_omega_from_pos_neg(
            CharAlphabet::of_size(2),
            [upw!("a"), upw!("aab")],
            [upw!("b"), upw!("abb")],
        );

        assert!(
            MinEvenParityCondition
                .consistent(&ts, &sample1, vec![], vec![])
                .0
        );
        assert!(
            MinEvenParityCondition
                .consistent(&ts2, &sample2, vec![], vec![])
                .0
        );
        assert!(
            !MinEvenParityCondition
                .consistent(&ts2, &sample3, vec![], vec![])
                .0
        );
        assert!(
            !MinEvenParityCondition
                .consistent(&ts2, &sample4, vec![], vec![])
                .0
        );
        assert!(
            !MinEvenParityCondition
                .consistent(&ts3, &sample4, vec![], vec![])
                .0
        );
        assert!(
            !MinEvenParityCondition
                .consistent(&ts4, &sample5, vec![], vec![])
                .0
        );
        assert!(
            MinEvenParityCondition
                .consistent(&ts4, &sample6, vec![], vec![])
                .0
        );
        assert!(
            MinEvenParityCondition
                .consistent(&ts5, &sample7, vec![], vec![])
                .0
        );
    }

    #[test]
    fn parity_consistent_automaton() {
        // build transition system
        let ts = DTS::builder()
            .with_transitions([
                (0, 'a', Void, 0),
                (0, 'b', Void, 1),
                (1, 'a', Void, 0),
                (1, 'b', Void, 2),
                (2, 'a', Void, 0),
                (2, 'b', Void, 2),
            ])
            .default_color(Void)
            .into_dts_with_initial(0);
        let ts2 = DTS::builder()
            .with_transitions([(0, 'a', Void, 0), (0, 'b', Void, 1), (1, 'a', Void, 0)])
            .default_color(Void)
            .into_dts_with_initial(0);

        // build sample
        let sample = OmegaSample::new_omega_from_pos_neg(
            sigma(),
            [upw!("bbba"), upw!("ababbba")],
            [upw!("b"), upw!("babbba")],
        );
        let sample2 = OmegaSample::new_omega_from_pos_neg(
            sigma(),
            [upw!("a"), upw!("aab")],
            [upw!("b"), upw!("abb")],
        );
        let sample3: OmegaSample = OmegaSample::new_for_alphabet(sigma());

        // build automaton
        let mut dpa = DTS::builder()
            .with_transitions([
                (0, 'b', 2, 1),
                (0, 'a', 0, 0),
                (1, 'b', 2, 2),
                (1, 'a', 1, 0),
                (2, 'b', 3, 2),
                (2, 'a', 2, 0),
            ])
            .default_color(Void)
            .into_dpa(0);
        let mut dpa2 = DTS::builder()
            .with_transitions([
                (0, 'a', 0, 0),
                (0, 'b', 0, 1),
                (1, 'a', 0, 0),
                (1, 'b', 1, 2),
                (2, 'a', 1, 2),
                (2, 'b', 1, 2),
            ])
            .default_color(Void)
            .into_dpa(0);

        let res = MinEvenParityCondition.consistent_automaton(&ts, &sample, vec![], vec![]);
        assert_eq!(res, dpa);
        // with completion via sink
        let res2 = MinEvenParityCondition.consistent_automaton(&ts2, &sample2, vec![], vec![]);
        assert_eq!(res2, dpa2);
        // with acceptance condition from infinity sets
        let pos_sets: Vec<EdgeSet> = vec![
            vec![(0, 'a')].into_iter().collect(),
            vec![(0, 'a'), (0, 'b'), (1, 'a')].into_iter().collect(),
        ];
        let neg_sets: Vec<EdgeSet> = vec![];
        let res3 = MinEvenParityCondition.consistent_automaton(&ts2, &sample2, pos_sets, neg_sets);
        assert_eq!(res3, dpa2);
    }

    #[test]
    fn parity_default_automaton() {
        let sample = OmegaSample::new_omega_from_pos_neg(sigma(), [upw!("abb")], [upw!("ab")]);

        let dpa = DTS::builder()
            .with_transitions([
                (0, 'a', 0, 1),
                (1, 'b', 0, 2),
                (2, 'b', 0, 0),
                (0, 'b', 1, 3),
                (1, 'a', 1, 3),
                (2, 'a', 1, 3),
                (3, 'a', 1, 3),
                (3, 'b', 1, 3),
            ])
            .default_color(Void)
            .into_dpa(0);

        assert_eq!(
            <MinEvenParityCondition as ConsistencyCheck<WithInitial<DTS>>>::default_automaton(
                &MinEvenParityCondition,
                &sample
            ),
            dpa
        );
    }
}
