use std::borrow::Cow;
use std::fmt::Debug;

use super::fwpm::FWPM;
use automata::alphabet::Alphabet;
use automata::automaton::{DFA, DPA};
use automata::representation::CollectPointed;
use automata::ts::{
    DenseId, Deterministic, DeterministicEdgesFrom, EdgeOf, EdgeRef, ForAlphabet, IdType,
    Sproutable, StateId, SymbolOf,
};
use automata::{DTS, Int, Pointed, RightCongruence, States, TransitionSystem, Void};
use itertools::Itertools;
use tracing::debug;

type Dfa<A> = DFA<DTS<A, bool, Void>>;

pub fn build_precise_dpa_for<A: Alphabet>(fwpm: FWPM<A>) -> DPA<DTS<A, Void, Int>> {
    match fwpm.complexity() {
        0 => panic!("Precise DPA construction only makes sense if at least one color exists"),
        1 => PreciseDPA::<A, 1>::from(fwpm)
            .reachable_part()
            .collect_mealy()
            .minimize()
            .collect_dpa(),
        2 => PreciseDPA::<A, 2>::from(fwpm)
            .reachable_part()
            .collect_mealy()
            .minimize()
            .collect_dpa(),
        3 => PreciseDPA::<A, 3>::from(fwpm)
            .reachable_part()
            .collect_mealy()
            .minimize()
            .collect_dpa(),
        4 => PreciseDPA::<A, 4>::from(fwpm)
            .reachable_part()
            .collect_mealy()
            .minimize()
            .collect_dpa(),
        5 => PreciseDPA::<A, 5>::from(fwpm)
            .reachable_part()
            .collect_mealy()
            .minimize()
            .collect_dpa(),
        6 => PreciseDPA::<A, 6>::from(fwpm)
            .reachable_part()
            .collect_mealy()
            .minimize()
            .collect_dpa(),
        7 => PreciseDPA::<A, 7>::from(fwpm)
            .reachable_part()
            .collect_mealy()
            .minimize()
            .collect_dpa(),
        8 => PreciseDPA::<A, 8>::from(fwpm)
            .reachable_part()
            .collect_mealy()
            .minimize()
            .collect_dpa(),
        _ => panic!("Too many priorities to construct precise DPA"),
    }
}

type ClassId = StateId;

/// We use const generics in the definition of the precise DPA. Therefore, it is necessary to bound the
/// number of colors that can be used. This constant is used as such a bound.
pub const PRECISE_DPA_COLORS: usize = 5;

/// A PState is a state in the precise DPA. It keeps track of the class in the leading
/// congruence and for each Mostowski level, it tracks the class and the state in the
/// corresponding progress DFA. We use const generics (the parameter `N`) to ensure that
/// `PState`s are [`Copy`].
#[derive(Copy, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PState<const N: usize> {
    class: StateId,
    progress_classes: [StateId; N],
    progress_states: [StateId; N],
}

impl<const N: usize> IdType for PState<N> {}

impl<const N: usize> std::fmt::Display for PState<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{} | {}]",
            self.class,
            self.progress_classes()
                .zip(self.progress_states())
                .map(|(c, q)| format!("({c} - {q})"))
                .join(", ")
        )
    }
}

impl<const N: usize> PState<N> {
    /// Returns the index of the class in the leading congruence.
    pub fn class(&self) -> ClassId {
        self.class
    }

    /// Returns an iterator over the classes of the currently active DFAs.
    pub fn progress_classes(&self) -> impl Iterator<Item = ClassId> + '_ {
        self.progress_classes.iter().cloned()
    }

    /// Returns an iterator over the states of the currently active DFAs.
    pub fn progress_states(&self) -> impl Iterator<Item = StateId> + '_ {
        self.progress_states.iter().cloned()
    }

    /// Creates a new instance of `Self` from the index of the class in the leading
    /// congruence and iterators over the classes and states of the currently active
    /// DFAs.
    pub fn from_iters<I: IntoIterator<Item = ClassId>, J: IntoIterator<Item = StateId>>(
        leading: ClassId,
        pc: I,
        pq: J,
    ) -> Self {
        let mut c = [StateId::default(); N];
        let mut q = [StateId::default(); N];
        pc.into_iter().enumerate().for_each(|(i, o)| c[i] = o);
        pq.into_iter().enumerate().for_each(|(i, o)| q[i] = o);
        Self {
            class: leading,
            progress_classes: c,
            progress_states: q,
        }
    }

    /// Creates a new instance of `Self` from the index of the class in the leading
    /// congruence and arrays of the classes and states of the currently active DFAs.
    pub fn new(leading: ClassId, pc: &[ClassId], pq: &[StateId]) -> Self {
        let c = std::array::from_fn(|i| pc[i]);
        let p = std::array::from_fn(|i| pq[i]);

        Self {
            class: leading,
            progress_classes: c,
            progress_states: p,
        }
    }
}

/// The precise DPA is a construction for going from a specifically colored FORC to a deterministic
/// parity automaton. It is described <https://arxiv.org/pdf/2302.11043.pdf>[here, below Lemma 15].
#[derive(Clone)]
pub struct PreciseDPA<A: Alphabet, const N: usize = 8> {
    states: Vec<PState<N>>,
    cong: RightCongruence<DTS<A>>,
    expressions: std::collections::BTreeMap<A::Symbol, A::Expression>,
    /// Nat -> class -> DFA
    dfas: Vec<[Dfa<A>; N]>,
}

impl<A: Alphabet, const N: usize> TransitionSystem for PreciseDPA<A, N> {
    type StateIndex = PState<N>;

    type StateColor = Void;

    type EdgeColor = Int;

    type EdgesFromIter<'this>
        = DeterministicEdgesFrom<'this, Self>
    where
        Self: 'this;

    type Alphabet = A;

    fn alphabet(&self) -> &Self::Alphabet {
        self.cong.alphabet()
    }

    fn state_color(&self, _state: PState<N>) -> Option<Cow<'_, Void>> {
        Some(Cow::Owned(Void))
    }

    fn edges_from(&self, state: PState<N>) -> Option<Self::EdgesFromIter<'_>> {
        Some(DeterministicEdgesFrom::new(self, state))
    }
}

impl<A: Alphabet, const N: usize> Deterministic for PreciseDPA<A, N> {
    fn edge(&self, state: PState<N>, symbol: SymbolOf<Self>) -> Option<EdgeOf<'_, Self>> {
        let (i, p) = self.take_precise_transition(&state, symbol);
        Some(EdgeRef::owned(
            state,
            self.expressions.get(&symbol)?,
            i,
            p,
        ))
    }
}

impl<A: Alphabet, const N: usize> Pointed for PreciseDPA<A, N> {
    fn initial(&self) -> Self::StateIndex {
        *self.states.first().expect("We add this during creation")
    }
}

impl<A: Alphabet, const N: usize> PreciseDPA<A, N> {
    /// Creates a new precise DPA from the given leading congruence and sequence of sequences of DFAs.
    pub fn new(cong: RightCongruence<DTS<A>>, dfas: Vec<[Dfa<A>; N]>) -> Self {
        let e = cong.initial();
        let initial = PState::from_iters(
            e,
            [e; N],
            (0..dfas.len()).map(|i| dfas[i][e.into_usize()].initial()),
        );
        Self {
            states: vec![initial],
            expressions: cong.alphabet().expression_map(),
            cong,
            dfas,
        }
    }

    pub fn dfas_level(&self, level: usize) -> impl Iterator<Item = (usize, &'_ Dfa<A>)> + '_ {
        self.dfas.iter().map(move |dfa| &dfa[level]).enumerate()
    }

    /// Returns a reference to the leading congruence.
    pub fn cong(&self) -> &RightCongruence<DTS<A>> {
        &self.cong
    }

    /// Given a [`PState`], returns an iterator over the DFAs that are currently active.
    pub fn dfas<'a>(&'a self, q: &'a PState<N>) -> impl Iterator<Item = &'a Dfa<A>> + 'a {
        q.progress_classes()
            .enumerate()
            .map(move |(i, c)| &self.dfas[c.into_usize()][i])
    }

    /// Given a [`PState`] and a symbol, returns the index of the least accepting DFA (which is
    /// the priority of the corresponding edge) and the successor [`PState`].
    pub fn take_precise_transition(&self, q: &PState<N>, a: A::Symbol) -> (Int, PState<N>) {
        let d = self
            .cong()
            .successor(q.class(), a)
            .expect("Leading congruence must be complete");

        let progress = self
            .dfas(q)
            .zip(q.progress_classes())
            .zip(q.progress_states())
            .map(|((dfa, c), q)| {
                let p = dfa
                    .successor(q, a)
                    .expect("all dfas must be complete");
                let b = *dfa
                    .state_color(p)
                    .expect("this state must exist as it is successor");
                (c, p, b)
            })
            .collect_vec();

        let least_accepting = progress
            .iter()
            .position(|(_, _, b)| *b)
            .expect("The last DFA must be accepting!");

        let reached_pstate = PState::from_iters(
            d,
            progress
                .iter()
                .enumerate()
                .map(|(i, (c, _p, _b))| if i < least_accepting { *c } else { d }),
            progress.iter().enumerate().map(|(i, (_c, p, _b))| {
                if i < least_accepting {
                    *p
                } else {
                    self.dfas[d.into_usize()][i].initial()
                }
            }),
        );

        (
            least_accepting
                .try_into()
                .expect("Should be able to cast to u8"),
            reached_pstate,
        )
    }
}

impl<A: Alphabet, const N: usize> Debug for PreciseDPA<A, N> {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

fn padding_universal_dfa<A: Alphabet>(alphabet: &A) -> Dfa<A> {
    let mut ts = DTS::for_alphabet_size_hint(alphabet.clone(), 1);
    let e = ts.add_state(true);

    for sym in alphabet.universe() {
        ts.add_edge((e, alphabet.make_expression(sym), Void, e));
    }
    DFA::from_parts(ts, e)
}

impl<A: Alphabet, const N: usize> From<FWPM<A>> for PreciseDPA<A, N> {
    fn from(value: FWPM<A>) -> Self {
        let start = std::time::Instant::now();

        let leading = value.leading().clone();
        let padding_dfa = padding_universal_dfa(leading.alphabet());
        let mut prc_dfas = Vec::with_capacity(leading.state_count());
        for (idx, mm) in value.pms() {
            let mut dfas = mm.decompose_dfa();
            assert!(dfas.len() <= N);
            while dfas.len() < N {
                dfas.push(padding_dfa.clone());
            }
            let array_dfas = dfas
                .try_into()
                .unwrap_or_else(|_v| panic!("LENGTH DOES NOT MATCH {N}"));
            prc_dfas.insert(idx.into_usize(), array_dfas);
        }

        debug!(
            "Building precise DPA with {N} priorities took {} microseconds",
            start.elapsed().as_micros()
        );

        PreciseDPA::new(leading, prc_dfas)
    }
}

#[cfg(test)]
mod tests {
    use automata::Void;
    use automata::alphabet::CharAlphabet;
    use automata::{DTS, TransitionSystem};

    use super::PreciseDPA;

    #[test]
    fn precise_dpa() {
        let _alph = CharAlphabet::of_size(3);

        let cong = DTS::builder()
            .with_transitions([
                (0, 'a', Void, 1),
                (0, 'b', Void, 0),
                (0, 'c', Void, 0),
                (1, 'a', Void, 0),
                (1, 'b', Void, 1),
                (1, 'c', Void, 1),
            ])
            .default_color(Void)
            .into_right_congruence(0);

        let de0 = DTS::builder()
            .with_transitions([
                (0, 'a', Void, 0),
                (0, 'b', Void, 1),
                (0, 'c', Void, 0),
                (1, 'a', Void, 1),
                (1, 'b', Void, 1),
                (1, 'c', Void, 1),
            ])
            .with_state_colors([false, true])
            .into_dfa(0);
        let da0 = DTS::builder()
            .with_transitions([
                (0, 'a', Void, 0),
                (0, 'b', Void, 0),
                (0, 'c', Void, 1),
                (1, 'a', Void, 1),
                (1, 'b', Void, 1),
                (1, 'c', Void, 1),
            ])
            .with_state_colors([false, true])
            .into_dfa(0);

        let de1 = DTS::builder()
            .with_transitions([
                (0, 'a', Void, 0),
                (0, 'b', Void, 2),
                (0, 'c', Void, 1),
                (1, 'a', Void, 0),
                (1, 'b', Void, 2),
                (1, 'c', Void, 2),
                (2, 'a', Void, 2),
                (2, 'b', Void, 2),
                (2, 'c', Void, 2),
            ])
            .with_state_colors([false, false, true])
            .into_dfa(0);

        let full = DTS::builder()
            .with_transitions([
                (0, 'a', Void, 1),
                (0, 'b', Void, 1),
                (0, 'c', Void, 1),
                (1, 'a', Void, 1),
                (1, 'b', Void, 1),
                (1, 'c', Void, 1),
            ])
            .with_state_colors([false, true])
            .into_dfa(0);

        let dfas_e = [de0, de1, full.clone()];
        let dfas_a = [da0, full.clone(), full];

        let dpa = PreciseDPA::new(cong, vec![dfas_e, dfas_a]);
        assert_eq!(dpa.reachable_state_indices().count(), 8);
    }
}
