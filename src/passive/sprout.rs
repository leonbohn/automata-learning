/// Module defining ConsistencyCheck and implementing it for various omega acceptance conditions
mod consistency;
pub use consistency::*;

use automata::{automaton::WithoutCondition, random, ts::path, Pointed, TransitionSystem, DTS};
use itertools::Itertools;
use tracing::{error, info, trace, warn};

use crate::prefixtree::prefix_tree;
use automata::automaton::{Automaton, WithInitial};
use automata::core::alphabet::CharAlphabet;
use automata::core::word::OmegaWord;
use automata::core::Void;
use automata::ts::{run, Deterministic, Shrinkable, Sproutable};
use std::{collections::HashSet, fmt::Debug, path::Iter};

use super::OmegaSample;

#[derive(thiserror::Error)]
pub enum SproutError<A: ConsistencyCheck<WithInitial<DTS>>> {
    #[error("timeout was exceeded, bailing with ts of size {}", aut.size())]
    Timeout {
        aut: Automaton<CharAlphabet, WithoutCondition, Void, Void>,
    },
    #[error("escape prefix threshold `{0}` exceeded, bailing with ts of size {}", aut.size())]
    Threshold { thres: usize, aut: A::Aut },
}

impl<A: ConsistencyCheck<WithInitial<DTS>>> Debug for SproutError<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use SproutError::*;
        write!(
            f,
            "{}",
            match self {
                Timeout { aut } => "reached timeout".to_string(),
                Threshold { thres, aut } => format!(
                    "exceeded threshold {thres} with automaton of size {}",
                    aut.size()
                ),
            }
        )
    }
}

#[allow(type_alias_bounds)]
pub type SproutResult<A: ConsistencyCheck<WithInitial<DTS>>> = Result<A::Aut, SproutError<A>>;

/// gives a deterministic acc_type omega automaton that is consistent with the given sample
/// implements the sprout passive learning algorithm for omega automata from <https://arxiv.org/pdf/2108.03735.pdf>
pub fn sprout<A: ConsistencyCheck<WithInitial<DTS>>>(
    sample: OmegaSample,
    acc_type: A,
) -> SproutResult<A> {
    let time_start = std::time::Instant::now();

    // make ts with initial state
    let mut ts = Automaton::new_with_initial_color(sample.alphabet().clone(), Void);

    // compute threshold
    let (lb, le) = sample
        .words()
        .map(|w| (w.spoke().len(), w.cycle().len()))
        .fold((0, 0), |(a0, a1), (b0, b1)| (a0.max(b0), a1.max(b1)));
    let thresh = (lb + le.pow(2) + 1) as isize;
    info!("starting sprout with threshold {thresh}");

    // while there are positive sample words that are escaping
    let mut pos_sets = vec![];
    let mut neg_sets = vec![];
    let mut mut_sample = sample.clone();

    'outer: loop {
        trace!("attempting to find an escape prefix with sets\npos {pos_sets:?}, neg {neg_sets:?}\n{mut_sample:?}\n{ts:?}");
        let Some(escape_prefix) = ts.omega_escape_prefixes(mut_sample.positive_words()).min()
        else {
            break;
        };

        trace!("found escape prefix {escape_prefix:?}");
        // WARN TODO should find a way to either pass or globally set timeout
        if time_start.elapsed() >= std::time::Duration::from_secs(60 * 30) {
            error!(
                "task exceeded timeout, aborting with automaton of size {}",
                ts.size()
            );
            return Err(SproutError::Timeout { aut: ts });
        }
        // check thresh
        if (escape_prefix.len() as isize) - 2 > thresh {
            error!(
                "task exceeded threshold with an automaton of size {}",
                ts.size()
            );
            // compute default automaton
            return Err(SproutError::Threshold {
                thres: thresh as usize,
                aut: acc_type.default_automaton(&sample),
            });
        }

        let source = ts.reached_state_index(&escape_prefix).unwrap();
        let sym = escape_prefix.escape_symbol();

        for q in ts.state_indices_vec() {
            // try adding transition
            ts.add_edge((source, sym, q));
            // continue if consistent
            let (is_consistent, pos_sets_new, neg_sets_new) =
                acc_type.consistent(&ts, &mut_sample, pos_sets.clone(), neg_sets.clone());
            if is_consistent {
                // update already known infinity sets
                pos_sets = pos_sets_new;
                neg_sets = neg_sets_new;
                mut_sample.remove_non_escaping(&ts);
                continue 'outer;
            } else {
                ts.remove_edges_from_matching(source, sym);
            }
        }
        // if none consistent add new state
        let new_state = ts.add_state(Void);
        ts.add_edge((source, sym, new_state));
    }

    info!(
        "completed sprout algorithm after {} microseconds",
        time_start.elapsed().as_micros()
    );
    Ok(acc_type.consistent_automaton(&ts, &mut_sample, pos_sets, neg_sets))
}

impl OmegaSample {
    /// Remove all words from the sample that are not escaping in the given transition system
    pub fn remove_non_escaping<T>(&mut self, ts: &T)
    where
        T: TransitionSystem<Alphabet = CharAlphabet, StateIndex = u32> + Deterministic + Pointed,
        // <T as TransitionSystem>::EdgeColor: Eq + std::hash::Hash,
    {
        // run transition system on sample words and
        // separate in escaping and non-escaping (successful) runs
        self.positive
            .retain(|w| ts.omega_run::<_, run::NoObserver>(w).is_escaping());
        self.negative
            .retain(|w| ts.omega_run::<_, run::NoObserver>(w).is_escaping());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::passive::OmegaSample;
    use automata::automaton::{BuchiCondition, MinEvenParityCondition};
    use automata::core::alphabet::CharAlphabet;
    use automata::core::{upw, Void};
    use automata::ts::Sproutable;
    use automata::DTS;
    use std::collections::HashSet;

    #[test]
    fn sprout_buchi_successful() {
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

        let res = sprout(sample.clone(), BuchiCondition).unwrap();
        for pos in sample.positive_words() {
            assert!(dba.accepts(pos));
            assert!(res.accepts(pos));
        }
        for neg in sample.negative_words() {
            assert!(!dba.accepts(neg));
            assert!(!res.accepts(neg));
        }

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

        let Err(SproutError::Threshold { thres: t, aut: res }) = sprout(sample, BuchiCondition)
        else {
            panic!("expected to hit threshold");
        };
        assert_eq!(res, dba);
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

        let res = sprout(sample, MinEvenParityCondition).unwrap();
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

        let Err(SproutError::Threshold { thres: t, aut: res }) =
            sprout(sample, MinEvenParityCondition)
        else {
            panic!("expected threshold to be exceeded");
        };
        assert_eq!(res, dpa);
    }
}
