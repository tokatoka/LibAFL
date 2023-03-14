//! A stage that runs until the mutator returns skipped

use core::marker::PhantomData;

use crate::{
    corpus::{Corpus, CorpusId},
    executors::{Executor, HasObservers},
    fuzzer::Evaluator,
    mutators::{Mutator, MutationResult},
    stages::Stage,
    state::{HasCorpus, UsesState},
    Error,
};

/// A stage that runs until the mutator returns skipped
#[derive(Clone, Debug)]
pub struct LoopMutationalStage<E, EM, M, Z> {
    mutator: M,
    phantom: PhantomData<(E, EM, Z)>,
}

impl<E, EM, M, Z> UsesState for LoopMutationalStage<E, EM, M, Z>
where
    E: UsesState,
{
    type State = E::State;
}

impl<E, EM, M, Z> Stage<E, EM, Z> for LoopMutationalStage<E, EM, M, Z>
where
    E: Executor<EM, Z> + HasObservers,
    EM: UsesState<State = E::State>,
    E::State: HasCorpus,
    M: Mutator<E::Input, E::State>,
    Z: Evaluator<E, EM, State = E::State>,
{
    fn perform(
        &mut self,
        fuzzer: &mut Z,
        executor: &mut E,
        state: &mut E::State,
        manager: &mut EM,
        corpus_idx: CorpusId,
    ) -> Result<(), Error> {
        let input = state
            .corpus()
            .get(corpus_idx)?
            .borrow_mut()
            .load_input()?
            .clone();

        let mut ctr = 0;
        loop {
            let mut input = input.clone();
            let res = self.mutator_mut().mutate(state, &mut input, ctr)?;

            if res == MutationResult::Skipped {
                break;
            }
            let (_, corpus_idx) = fuzzer.evaluate_input(state, executor, manager, input)?;

            self.mutator_mut().post_exec(state, ctr, corpus_idx)?;
            ctr += 1;
        };

        Ok(())
    }
}

impl<E, EM, M, Z> LoopMutationalStage<E, EM, M, Z> {
    /// Constructor
    pub fn new(mutator: M) -> Self {
        Self {
            mutator,
            phantom: PhantomData,
        }
    }

    fn mutator_mut(&mut self) -> &mut M {
        &mut self.mutator
    }
}
