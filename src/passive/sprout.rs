use automata::{math::Set, prelude::*, transition_system::path};
use itertools::Itertools;

use std::{collections::HashSet, path::Iter};

use crate::prefixtree::prefix_tree;

use super::{consistency::ConsistencyCheck, OmegaSample};

/// gives a deterministic acc_type omega automaton that is consistent with the given sample
/// implements the sprout passive learning algorithm for omega automata from <https://arxiv.org/pdf/2108.03735.pdf>
pub fn sprout<A: ConsistencyCheck<WithInitial<DTS>>>(sample: OmegaSample, acc_type: A) -> A::Aut {
    // make ts with initial state
    let mut ts = Automaton::new_with_initial_color(sample.alphabet().clone(), Void);

    // compute threshold
    let (lb, le) = sample
        .words()
        .map(|w| (w.spoke().len(), w.cycle().len()))
        .fold((0, 0), |(a0, a1), (b0, b1)| (a0.max(b0), a1.max(b1)));
    let thresh = (lb + le.pow(2) + 1) as isize;
    println!("threshold: {}", thresh);

    // while there are positive sample words that are escaping
    let mut pos_sets = vec![];
    let mut neg_sets = vec![];
    let mut mut_sample = sample.clone();
    'outer: while let Some(escape_prefix) =
        length_lexicographical_sort(ts.escape_prefixes(mut_sample.positive_words()).collect())
            .first()
    {
        let u = escape_prefix[..escape_prefix.len() - 1].to_string();
        let a = escape_prefix.chars().last().expect("empty escape prefix");
        // check thresh
        if (u.len() as isize) - 1 > thresh {
            // compute default automaton
            return acc_type.default_automaton(&sample);
        }
        // dbg!(u.len());
        let source = ts.finite_run(&u).unwrap().reached();
        for q in ts.state_indices_vec() {
            // try adding transition
            ts.add_edge((source, a, Void, q));
            // continue if consistent
            let (is_consistent, pos_sets_new, neg_sets_new) =
                acc_type.consistent(&ts, &mut_sample, pos_sets.clone(), neg_sets.clone());
            if is_consistent {
                // update already known infinity sets
                pos_sets = pos_sets_new;
                neg_sets = neg_sets_new;
                mut_sample.remove_non_escaping(&ts);
                // dbg!(ts.size());
                // dbg!(pos_sets.len());
                // dbg!(neg_sets.len());
                // dbg!(mut_sample.words.len());
                continue 'outer;
            } else {
                ts.remove_edges_from_matching(source, a);
            }
        }
        // if none consistent add new state
        let new_state = ts.add_state(Void);
        ts.add_edge((source, a, Void, new_state));
    }
    acc_type.consistent_automaton(&ts, &mut_sample, pos_sets, neg_sets)
}

/// sort a vector of Strings length lexicographically
pub fn length_lexicographical_sort(mut words: Vec<String>) -> Vec<String> {
    words.sort_by(|a, b| {
        // Compare by length first
        let length_comparison = a.len().cmp(&b.len());

        // If lengths are equal, compare lexicographically
        if length_comparison == std::cmp::Ordering::Equal {
            a.cmp(b)
        } else {
            length_comparison
        }
    });
    words
}

impl OmegaSample {
    /// Remove all words from the sample that are not escaping in the given transition system
    pub fn remove_non_escaping<T>(&mut self, ts: &T)
    where
        T: TransitionSystem<Alphabet = CharAlphabet, StateIndex = u32> + Deterministic + Pointed,
    {
        // run transition system on sample words and
        // separate in escaping and non-escaping (successful) runs
        let successful: Vec<_> = self
            .words
            .keys()
            .filter(|w| ts.omega_run(w).is_ok())
            .cloned()
            .collect();
        for w in successful {
            self.remove(&w);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::passive::OmegaSample;
    use automata::prelude::*;

    #[test]
    fn sprout_buchi() {
        let sigma = CharAlphabet::of_size(2);

        // build sample
        let sample =
            OmegaSample::new_omega_from_pos_neg(sigma, [upw!("a"), upw!("a", "b")], [upw!("b")]);

        // build dba
        let dba = DTS::builder()
            .with_transitions([
                (0, 'a', true, 1),
                (1, 'a', true, 0),
                (1, 'b', true, 1),
                (0, 'b', false, 2),
                (2, 'a', false, 2),
                (2, 'b', false, 2),
            ])
            .default_color(Void)
            .into_dba(0);

        let res = sprout(sample, BuchiCondition);
        assert_eq!(res, dba);
    }

    #[test]
    fn sprout_buchi_thresh() {
        let sigma = CharAlphabet::of_size(2);

        // build sample
        let sample = OmegaSample::new_omega_from_pos_neg(
            sigma,
            [upw!("a"), upw!("baa")],
            [upw!("ab"), upw!("ba"), upw!("babaa"), upw!("baaba")],
        );

        // build dba
        let mut dba = DTS::builder()
            .with_transitions([
                (0, 'a', true, 1),
                (0, 'b', true, 2),
                (1, 'a', true, 1),
                (1, 'b', false, 5),
                (2, 'a', true, 3),
                (2, 'b', false, 5),
                (3, 'a', true, 4),
                (3, 'b', false, 5),
                (4, 'b', true, 2),
                (4, 'a', false, 5),
                (5, 'a', false, 5),
                (5, 'b', false, 5),
            ])
            .default_color(Void)
            .into_dba(0);

        let res = sprout(sample, BuchiCondition);
        assert_eq!(res, dba);
    }

    #[test]
    fn llex_sort() {
        assert_eq!(
            length_lexicographical_sort(vec![
                String::from("ca"),
                String::from("ac"),
                String::from("aaa"),
                String::from("b")
            ]),
            vec![
                String::from("b"),
                String::from("ac"),
                String::from("ca"),
                String::from("aaa")
            ]
        );
    }

    #[test]
    fn sprout_parity() {
        let sigma = CharAlphabet::of_size(2);

        // build sample
        let sample = OmegaSample::new_omega_from_pos_neg(
            sigma,
            [upw!("a"), upw!("aab")],
            [upw!("b"), upw!("abb")],
        );

        // build dba
        let dpa = DTS::builder()
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

        let res = sprout(sample, MinEvenParityCondition);
        assert_eq!(res, dpa);
    }

    #[test]
    fn sprout_parity_thresh() {
        let sigma = CharAlphabet::of_size(2);

        // build sample
        let sample = OmegaSample::new_omega_from_pos_neg(
            sigma,
            [upw!("a"), upw!("baaa")],
            [upw!("ba"), upw!("baa")],
        );

        // build dba
        let mut dpa = DTS::builder()
            .with_transitions([
                (0, 'a', 0, 1),
                (1, 'a', 0, 1),
                (0, 'b', 0, 2),
                (2, 'a', 0, 3),
                (3, 'a', 0, 4),
                (4, 'a', 0, 5),
                (5, 'b', 0, 2),
            ])
            .default_color(Void)
            .into_dpa(0);
        dpa.complete_with_colors(Void, 1);

        let res = sprout(sample, MinEvenParityCondition);
        assert_eq!(res, dpa);
    }
}
