use std::fmt::Debug;


use automata::alphabet::{Alphabet, CharAlphabet};
use automata::automaton::{MealyMachine, MooreMachine};
use automata::dag::Dag;
use automata::representation::IntoTransitions;
use automata::ts::operations::DefaultIfMissing;
use automata::ts::Sccs;
use automata::word::FiniteWord;
use automata::{Color, DTS, Int, Pointed, States, Void};
use automata::{RightCongruence, TransitionSystem};

/// A priority mapping is essentially a [`MealyMachine`], i.e. it reads
/// finite words and ouptuts a priority (which in this case is a `usize`).
pub type WeakPriorityMapping<A = CharAlphabet> = MealyMachine<DTS<A, Void, Int>>;

/// Stores information on classes/states of a [`RightCongruence`]. This may be
/// extended in the futuer, but for now it simply stores whether a class c is
/// idempoten (meaning cc ~ c) and whether it is good (in the sense that the
/// omega iteration of c is in the language L_c).
#[derive(Clone, Copy, Ord, PartialEq, PartialOrd, Eq, Hash, Default)]
pub struct Annotation {
    pub(super) idempotent: bool,
    pub(super) good: Option<bool>,
}

impl std::fmt::Display for Annotation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(if !self.idempotent {
            "#"
        } else {
            match self.good {
                Some(true) => "+",
                Some(false) => "-",
                None => "o",
            }
        })
    }
}

impl std::fmt::Debug for Annotation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(b) = self.good {
            write!(f, "{}", if b { '+' } else { '-' })?;
        }
        Ok(())
    }
}

impl Annotation {
    /// Creates a new annotation.
    pub fn new(idempotent: bool, good: Option<bool>) -> Self {
        Self { idempotent, good }
    }
}

/// This a simple newtype wrapper around a congruence, which has no edge colors and uses
/// [`Annotation`]s as state colors.
#[derive(Clone)]
pub struct AnnotatedCongruence<A: Alphabet = CharAlphabet>(
    RightCongruence<DTS<A, Annotation, Void>>,
);

impl<A: Alphabet> AnnotatedCongruence<A> {
    pub fn new(rc: RightCongruence<DTS<A, Annotation, Void>>) -> Self {
        Self(rc)
    }
}

pub trait ClassifiesIdempotents<A: Alphabet> {
    fn classify(&self, class: impl FiniteWord<Symbol = A::Symbol>) -> Option<bool>;
}

impl<A: Alphabet, CI: ClassifiesIdempotents<A>> ClassifiesIdempotents<A> for &CI {
    fn classify(&self, class: impl FiniteWord<Symbol = <A as Alphabet>::Symbol>) -> Option<bool> {
        CI::classify(self, class)
    }
}

impl<A: Alphabet> Debug for AnnotatedCongruence<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl<A: Alphabet> AnnotatedCongruence<A> {
    /// Computes the canonic coloring on a given annotated congruence. This makes use
    /// of the dag of strongly connected components of the congruence. For more information
    /// on how the computation is done exactly, see [Section 5, Step 2](https://arxiv.org/pdf/2302.11043.pdf).
    pub fn canonic_coloring(&self) -> MooreMachine<DTS<A, Int, Void>> {
        // we first need to decompose into sccs and mark them with the color of the
        // idempotent that it contains.
        let tjdag = Sccs::on(&self.0).compute();
        let mut dag: Dag<Result<usize, Option<bool>>> =
            tjdag.fold_state_colors(Err(None), |acc, x| match (acc, x.good) {
                (Err(None), None) => Err(None),
                (Err(Some(x)), Some(y)) => Err(Some(x || y)),
                (Err(Some(x)), None) => Err(Some(x)),
                (Err(None), Some(x)) => Err(Some(x)),
                (Ok(_), _) => unreachable!(),
            });

        let mut seen = std::collections::BTreeSet::default();
        'outer: loop {
            if dag.masked_is_empty(&seen) {
                break 'outer;
            }
            let t = dag.masked_terminal_nodes(&seen).next().unwrap();
            assert!(seen.insert(t), "This must not have been seen before!");
            let i = dag
                .immediate(t)
                .map(|i| {
                    dag.color(i)
                        .expect("This node must exist")
                        .expect("The color of this node must already be known")
                })
                .max()
                .unwrap_or(0);
            let parity = i % 2 == 0;

            let offset = match dag.color(t).expect("We know this node exists") {
                Err(Some(true)) => !parity as usize,
                Err(Some(false)) => parity as usize,
                Err(None) => 0,
                Ok(_) => unreachable!(),
            };
            *dag.color_mut(t).expect("This node exists") = Ok(i + offset);
        }

        let state_coloring: automata::math::FxIndexMap<_, _> = self
            .0
            .states()
            .map(|i| {
                let scc = tjdag.scc_index_of(i).expect("Must be in an SCC");
                let info = dag.color(*scc).expect("Must have worked on that SCC");
                (
                    i,
                    info.expect("Every SCC must have a color")
                        .try_into()
                        .unwrap(),
                )
            })
            .collect();

        (&self.0)
            .erase_edge_colors()
            .with_state_color(DefaultIfMissing::new(state_coloring, 0))
            .with_initial(self.0.initial())
            .collect()
    }

    /// Takes a reference to a right congruence and a function that classifies idempotents
    /// of the congruence to construct an annotated congruence.
    pub fn build<Q, C, F>(rc: &RightCongruence<DTS<A, Q, C>>, f: F) -> Self
    where
        Q: Color,
        C: Color,
        F: ClassifiesIdempotents<A>,
    {
        let annotations: automata::math::FxIndexMap<_, _> = rc
            .states()
            .map(|i| {
                let cls = rc.state_to_mr(i).unwrap();
                let out = if !cls.is_empty() && rc.is_idempotent(i) == Some(true) {
                    let b = f.classify(cls);
                    Annotation::new(true, b)
                } else {
                    Annotation::new(false, None)
                };
                (i, out)
            })
            .collect();

        Self::new(
            rc.erase_edge_colors()
                .with_state_color(DefaultIfMissing::new(annotations, Annotation::default()))
                .with_initial(rc.initial())
                .collect(),
        )
    }
}
