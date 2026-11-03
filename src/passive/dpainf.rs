use std::collections::{BTreeSet, VecDeque};

use automata::{
    Class, DTS, Pointed, RightCongruence, States, TransitionSystem, ts::operations::ProductIndex,
};
use itertools::Itertools;
use tracing::{error, trace};

use crate::{
    passive::{ClassOmegaSample, FiniteSample, OmegaSample, SplitOmegaSample},
    prefixtree::prefix_tree,
};

use automata::alphabet::Alphabet;
use automata::automaton::DFA;
use automata::representation::{CollectPointed, IntoTransitions};
use automata::ts::operations::Product;
use automata::ts::{
    CompactIdMap, Deterministic, DenseId, PredecessorIterable, Sccs, Shrinkable, Sproutable,
    StateId,
};
use automata::word::FiniteWord;
use automata::Void;

/// Represents a consistency check that can be performed on a congruence. This is used in the
/// omega-sprout algorithm to ensure in each iteration, that the produced congruence relation
/// is consistent with the given constraints. The constraints can either be given by a conflict
/// relation, by a list or they could be verified against a sample (or other form of data).
pub trait ConsistencyCheck<A: Alphabet> {
    /// Verifies that `cong` is consistent with the constraint.
    fn consistent(&self, cong: &RightCongruence<DTS<A>>) -> bool;
    /// Returns an approximate threshold for the number of classes in the congruence. This is
    /// useful for algorithms to detect infinite loops.
    fn threshold(&self) -> usize;
    /// Returns a reference to the alphabet used by the constraint.
    fn alphabet(&self) -> &A;
}

impl<A: Alphabet, CC: ConsistencyCheck<A>> ConsistencyCheck<A> for &CC {
    fn alphabet(&self) -> &A {
        CC::alphabet(self)
    }
    fn consistent(&self, cong: &RightCongruence<DTS<A>>) -> bool {
        CC::consistent(self, cong)
    }
    fn threshold(&self) -> usize {
        CC::threshold(self)
    }
}

impl<A: Alphabet> ConsistencyCheck<A> for FiniteSample<A> {
    fn consistent(&self, cong: &RightCongruence<DTS<A>>) -> bool {
        let positive_indices: std::collections::BTreeSet<_> = self
            .positive_words()
            .filter_map(|w| cong.reached_state(w))
            .collect();
        let negative_indices: std::collections::BTreeSet<_> = self
            .negative_words()
            .filter_map(|w| cong.reached_state(w))
            .collect();
        positive_indices.is_disjoint(&negative_indices)
    }

    fn threshold(&self) -> usize {
        self.max_word_len() * 2
    }

    fn alphabet(&self) -> &A {
        &self.alphabet
    }
}

/// Stores two DFAs and a math::FxIndexSet of conflicts between them.
#[derive(Clone)]
pub struct ConflictRelation<A: Alphabet> {
    dfas: [RightCongruence<DTS<A>>; 2],
    conflicts: std::collections::BTreeSet<(StateId, StateId)>,
}

impl<A: Alphabet> ConsistencyCheck<A> for ConflictRelation<A> {
    fn alphabet(&self) -> &A {
        self.dfas[0].alphabet()
    }
    /// Verifies that a given congruence is consistent with the conflicts.
    fn consistent(&self, cong: &RightCongruence<DTS<A>>) -> bool {
        let left = cong.ts_product(&self.dfas[0]);
        let right = cong.ts_product(&self.dfas[1]);
        let right_reachable = right.reachable_state_indices().collect_vec();

        for ProductIndex(lcong, ldfa) in left.reachable_state_indices() {
            for ProductIndex(rcong, rdfa) in right_reachable
                .iter()
                .filter(|ProductIndex(rcong, _)| rcong == &lcong)
            {
                if lcong == *rcong && self.conflicts.contains(&(ldfa, *rdfa)) {
                    let lname = ldfa;
                    let rname = rdfa;
                    let congname = lcong;
                    trace!(
                        "\t\tConflict found, ({congname}, {lname}) and ({congname}, {rname}) reachable with ({lname}, {rname}) in conflicts"
                    );
                    return false;
                }
            }
        }
        true
    }

    /// Returns a preliminary threshold for the number of states in the product of the two DFAs.
    fn threshold(&self) -> usize {
        2 * self.dfas[0].state_count() * self.dfas[1].state_count()
    }
}

impl<A: Alphabet> std::fmt::Debug for ConflictRelation<A> {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl<A: Alphabet> ConflictRelation<A> {
    /// Returns a reference to the underlying alphabet of one of the DFAs. We assure that both DFAs use the same
    /// alphabet.
    pub fn alphabet(&self) -> &A {
        self.dfas[0].alphabet()
    }
}

/// Drops the colors of the DFA and returns the remaining congruence together with the map from
/// the old to the new state ids.
fn to_congruence<A: Alphabet>(
    dfa: &DFA<DTS<A, bool, Void>>,
) -> (RightCongruence<DTS<A>>, CompactIdMap<StateId>) {
    dfa.erase_colors()
        .with_initial(dfa.initial())
        .collect_with_map()
}

/// Computes a conflict relation encoding iteration consistency. For more details on the construction,
/// see Lemma 29 in [this paper](https://arxiv.org/pdf/2302.11043.pdf).
pub fn iteration_consistency_conflicts<A: Alphabet>(
    samples: &SplitOmegaSample<'_, A>,
    class: Class<A::Symbol>,
) -> ConflictRelation<A> {
    let idx = samples.cong().reached_state(&class).unwrap();
    let Some(sample) = samples.get(idx) else {
        panic!("Sample for class {:?} does not exist!", class)
    };
    let periodic_sample = sample.to_periodic_sample();

    trace!(
        "Positive periodic words: {}\nNegative periodic words: {}",
        periodic_sample
            .positive()
            .map(|w| format!("{:?}", w))
            .join(","),
        periodic_sample
            .negative()
            .map(|w| format!("{:?}", w))
            .join(",")
    );

    let looping_words = samples.cong().looping_words(idx);

    let left_pta = DFA::from(
        prefix_tree(sample.alphabet.clone(), periodic_sample.positive()).map_state_colors(|mr| {
            !mr.is_empty() && periodic_sample.classify(mr.omega_power()) == Some(true)
        }),
    )
    .intersection(&looping_words)
    .reachable_part()
    .collect_dfa();

    let right_pta = DFA::from(
        prefix_tree(sample.alphabet.clone(), periodic_sample.negative()).map_state_colors(|mr| {
            !mr.is_empty() && periodic_sample.classify(mr.omega_power()) == Some(false)
        }),
    )
    .intersection(&looping_words)
    .reachable_part()
    .collect_dfa();

    let mut conflicts = std::collections::BTreeSet::default();
    let mut queue = VecDeque::from_iter(
        left_pta
            .accepting_states()
            .cartesian_product(right_pta.accepting_states()),
    );

    let mut left_cache: std::collections::HashMap<_, _> = std::collections::HashMap::default();
    let mut right_cache: std::collections::HashMap<_, _> = std::collections::HashMap::default();

    while let Some((left, right)) = queue.pop_front() {
        if !conflicts.insert((left, right)) {
            continue;
        }
        let left_pred = left_cache
            .entry(left)
            .or_insert_with(|| left_pta.predecessors(left).unwrap().collect_vec());
        let right_pred = right_cache
            .entry(right)
            .or_insert_with(|| right_pta.predecessors(right).unwrap().collect_vec());

        for l in left_pred {
            for r in right_pred
                .iter()
                .filter(|o| o.expression() == l.expression())
            {
                queue.push_back((l.source(), r.source()));
            }
        }
    }

    let (left, left_map) = to_congruence(&left_pta);
    let (right, right_map) = to_congruence(&right_pta);

    let renumber = |map: &CompactIdMap<StateId>, q| StateId::from_usize(map.id(&q).unwrap());
    let conflicts: BTreeSet<_> = conflicts
        .into_iter()
        .map(|(l, r)| (renumber(&left_map, l), renumber(&right_map, r)))
        .collect();

    ConflictRelation {
        dfas: [left, right],
        conflicts,
    }
}

/// Computes a conflict relation encoding prefix consistency. For more details on how this works, see
/// Lemma 28 in [this paper](https://arxiv.org/pdf/2302.11043.pdf).
pub fn prefix_consistency_conflicts<A: Alphabet>(sample: &OmegaSample<A>) -> ConflictRelation<A> {
    let left_pta = prefix_tree(sample.alphabet.clone(), sample.positive_words());
    let right_pta = prefix_tree(sample.alphabet.clone(), sample.negative_words());

    let dfa = (&left_pta).ts_product(&right_pta).reachable_part();

    trace!("built prefix tree product");
    let sccs = Sccs::on(&dfa).compute();
    trace!("computed scc decomposition");

    let states_with_infinite_run: Vec<(StateId, StateId)> = sccs
        .sccs_iter()
        .filter(|scc| !scc.is_transient())
        .flat_map(|scc| scc.clone().into_iter().map(Into::into))
        .collect();

    let mut conflicts = std::collections::BTreeSet::default();
    for ProductIndex(l, r) in dfa.states() {
        let reachable = dfa
            .reachable_state_indices_from(ProductIndex(l, r))
            .collect_vec();
        if reachable
            .iter()
            .any(|ProductIndex(p, q)| states_with_infinite_run.contains(&(*p, *q)))
        {
            conflicts.insert((l, r));
        }
    }

    ConflictRelation {
        dfas: [
            (&left_pta).erase_state_colors().collect_right_congruence(),
            (&right_pta).erase_state_colors().collect_right_congruence(),
        ],
        conflicts,
    }
}

impl<A: Alphabet> ConsistencyCheck<A> for () {
    fn consistent(&self, _cong: &RightCongruence<DTS<A>>) -> bool {
        true
    }

    fn threshold(&self) -> usize {
        0
    }

    fn alphabet(&self) -> &A {
        unimplemented!("This does not make sense, you should not call this function directly")
    }
}

/// This constraint ensures that the learned automaton separates idempotents.
#[derive(Clone)]
pub struct SeparatesIdempotents<'a, A: Alphabet> {
    // placeholder: the constraint is not implemented yet
    #[allow(dead_code)]
    sample: &'a ClassOmegaSample<'a, A>,
}

impl<'a, A: Alphabet> SeparatesIdempotents<'a, A> {
    /// Creates a new instance of the constraint.
    pub fn new(sample: &'a ClassOmegaSample<'a, A>) -> Self {
        Self { sample }
    }
}

impl<A: Alphabet> ConsistencyCheck<A> for SeparatesIdempotents<'_, A> {
    fn consistent(&self, _cong: &RightCongruence<DTS<A>>) -> bool {
        true
    }

    fn threshold(&self) -> usize {
        todo!()
    }

    fn alphabet(&self) -> &A {
        todo!()
    }
}

#[derive(Debug)]
pub enum DpaInfError<A: Alphabet> {
    /// The threshold has been exceeded, returns constructed right congruence and threshold value
    Threshold(RightCongruence<DTS<A>>, usize),
    /// The given timeout has been exceeded, returns right congruence that has been constructed thus far
    Timeout(RightCongruence<DTS<A>>),
}

/// Runs the omega-sprout algorithm on a given conflict relation.
#[allow(clippy::result_large_err)]
pub fn dpainf<A, C>(
    conflicts: C,
    additional_constraints: Vec<Box<dyn ConsistencyCheck<A>>>,
    allow_transitions_into_epsilon: bool,
    timeout_seconds: Option<u64>,
) -> Result<RightCongruence<DTS<A>>, DpaInfError<A>>
where
    A: Alphabet,
    C: ConsistencyCheck<A>,
{
    let mut cong: RightCongruence<DTS<A>> =
        RightCongruence::new_with_initial_color(conflicts.alphabet().clone(), Void);
    let initial = cong.initial();
    let threshold = conflicts.threshold();

    // We maintain a math::FxIndexSet of missing transitions and go through them in order of creation for the states and in order
    // give by alphabet for the symbols for one state (this amouts to BFS).
    let mut queue: VecDeque<_> = conflicts
        .alphabet()
        .universe()
        .map(|sym| (initial, sym))
        .collect();

    let time_start = std::time::Instant::now();
    let timeout_seconds = timeout_seconds.unwrap_or(u64::MAX);
    'outer: while let Some((source, sym)) = queue.pop_front() {
        if time_start.elapsed().as_secs() >= timeout_seconds {
            error!("exceeded timeout, returning right congruence built so far");
            return Err(DpaInfError::Timeout(cong));
        }

        for target in 0..cong.state_count() {
            let target = StateId::from_usize(target);
            if !allow_transitions_into_epsilon && target == initial {
                continue;
            }
            cong.add_edge((source, cong.alphabet().make_expression(sym), Void, target));

            if conflicts.consistent(&cong)
                && additional_constraints.iter().all(|c| c.consistent(&cong))
            {
                trace!(
                    "\tTransition {source}--{}-->{target} is consistent",
                    sym,
                );
                continue 'outer;
            } else {
                trace!(
                    "\tTransition {source}--{}-->{target} is not consistent",
                    sym,
                );
                cong.remove_edges_between_matching(source, target, sym);
            }
        }

        if cong.state_count() > threshold {
            error!("exceeded threshold on number of states {threshold}");
            return Err(DpaInfError::Threshold(cong, threshold));
        }

        trace!(
            "No consistent transition found, adding new state {}",
            cong.state_count()
        );

        let new_state = cong.add_state(Void);
        cong.add_edge((source, cong.alphabet().make_expression(sym), Void, new_state));
        queue.extend(std::iter::repeat(new_state).zip(conflicts.alphabet().universe()))
    }

    Ok(cong)
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::passive::{SetSample, dpainf::ConflictRelation, sample::OmegaSample};
    use automata::alphabet::CharAlphabet;
    use automata::upw;
    use automata::ts::q;
    use automata::{Class, States};

//     pub fn inf_aba_sample() -> (CharAlphabet, OmegaSample<CharAlphabet>) {
//         let Ok(sample) = OmegaSample::try_from_str(
//             r#"omega
//             alphabet: a,b
//             positive:
//             abab
//             aba
//             ababb
//             abaa
//             abaab
//             negative:
//             bab
//             abba
//             bbab
//             aaabb
//             a
//             abb
//             aabb
//             babb
//             b
//             abbba
//             abbb"#,
//         ) else {
//             panic!("Cannot parse sample")
//         };
//         (sample.alphabet.clone(), sample)
//     }

    pub fn testing_larger_forc_sample() -> (CharAlphabet, OmegaSample<CharAlphabet>) {
        let Ok(sample) = OmegaSample::try_from_str(
            r#"omega
        alphabet: a,b
        positive:
        bbabab
        ab
        baa
        abbab
        babab
        babba
        bbaba
        babab
        babba
        aba
        aab
        abaabb
        ababb
        a
        abab
        baba
        ba
        bbaba
        abbab
        babbba
        abbab
        abbaab
        babbbba
        negative:
        bba
        abba
        baab
        bbba
        abb
        abbba
        bab
        bba
        babb
        bbab
        b
        bb
        abba
        bbaab
        abbb
        bbaa
        abbaa
        babbab
        bbabba
        babbb
        bbabb
        "#,
        ) else {
            panic!("Cannot parse sample");
        };
        (sample.alphabet.clone(), sample)
    }

    fn testing_smaller_forc_smaple() -> (CharAlphabet, OmegaSample<CharAlphabet>) {
        let alphabet = CharAlphabet::of_size(3);
        (
            alphabet.clone(),
            SetSample::new_omega_from_pos_neg(
                alphabet,
                [
                    upw!("a"),
                    upw!("baa"),
                    upw!("aca"),
                    upw!("caab"),
                    upw!("abca"),
                ],
                [upw!("b"), upw!("c"), upw!("ab"), upw!("ac"), upw!("abc")],
            ),
        )
    }

    #[test]
    fn learn_small_forc() {
        let (_alphabet, sample) = testing_smaller_forc_smaple();
        let cong = sample.infer_prefix_congruence().unwrap();
        assert_eq!(cong.state_count(), 1);

        let split_sample = sample.split(&cong);
        let eps = Class::epsilon();
        let _eps_sample = split_sample.get(q(0)).unwrap();

        let conflicts: ConflictRelation<CharAlphabet> =
            super::iteration_consistency_conflicts(&split_sample, eps);

        let prc_eps = super::dpainf(conflicts, vec![], false, None).unwrap();
        assert_eq!(prc_eps.state_count(), 6);
    }

    #[test_log::test]
    fn learn_larger_forc() {
        let (_alphabet, sample) = testing_larger_forc_sample();
        let cong = sample.infer_prefix_congruence().unwrap();
        tracing::debug!("Got prefix congruence");
        let split = sample.split(&cong);
        let forc = split.infer_forc();
        let prc_eps = forc[q(0)].clone();
        assert_eq!(prc_eps.state_count(), 13);
    }
}
