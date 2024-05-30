use automata::{math::Set, prelude::*, transition_system::path};

use std::{collections::HashSet, path::Iter};

use crate::prefixtree::prefix_tree;

use super::{consistency::ConsistencyCheck, OmegaSample};

/// gives a deterministic acc_type omega automaton that is consistent with the given sample
/// implements the sprout passive learning algorithm for omega automata from <https://arxiv.org/pdf/2108.03735.pdf>
pub fn sprout<A: ConsistencyCheck<WithInitial<DTS>>>(sample: OmegaSample, acc_type: A) -> A::Aut {
    // make ts with initial state
    // let mut temp_ts = DTS::for_alphabet(sample.alphabet().clone())
    // temp_ts.add_state(Void);
    // let mut ts = temp_ts.with_initial(0);

    let mut ts = Automaton::new_with_initial_color(sample.alphabet().clone(), Void);

    // compute threshold
    let (lb, le) = sample
        .words()
        .map(|w| (w.spoke().len(), w.cycle().len()))
        .fold((0, 0), |(a0, a1), (b0, b1)| (a0.max(b0), a1.max(b1)));
    let thresh = (lb + le ^ 2 + 1) as isize;

    // while there are positive sample words that are escaping
    'outer: while let Some(escape_prefix) =
        length_lexicographical_sort(ts.escape_prefixes(sample.positive_words()).collect()).first()
    {
        let u = escape_prefix[..escape_prefix.len() - 1].to_string();
        let a = escape_prefix.chars().last().expect("empty escape prefix");
        // check thresh
        if (u.len() as isize) - 1 > thresh {
            // compute default automaton
            return acc_type.default_automaton(&sample);
        }
        let source = ts.finite_run(u).unwrap().reached();

        for q in ts.state_indices_vec() {
            // try adding transition
            ts.add_edge((source, a, Void, q));
            // continue if consistent
            if acc_type.consistent(&ts, &sample) {
                continue 'outer;
            } else {
                ts.remove_edges_from_matching(source, a);
            }
        }
        // if none consistent add new state
        let new_state = ts.add_state(Void);
        ts.add_edge((source, a, Void, new_state));
    }
    acc_type.consistent_automaton(&ts, &sample)
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

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::passive::OmegaSample;
    use automata::prelude::*;

    #[test]
    fn sprout_buchi() {
        let sigma = alphabet!(simple 'a', 'b');

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
        assert_eq!(format!("{:?}", res), format!("{:?}", dba));
    }

    #[test]
    fn sprout_buchi_thresh() {
        let sigma = alphabet!(simple 'a', 'b');

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
        assert_eq!(format!("{:?}", res), format!("{:?}", dba));
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
}
