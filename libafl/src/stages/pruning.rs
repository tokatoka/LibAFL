//! Corpus pruning stage

use alloc::string::ToString;
use core::marker::PhantomData;

use libafl_bolts::{rands::Rand, Error};

#[cfg(feature = "std")]
use crate::events::EventRestarter;
use crate::{
    corpus::{Corpus, HasCurrentCorpusId}, feedbacks::Feedback, schedulers::{RemovableScheduler, Scheduler}, stages::Stage, state::{HasCorpus, HasRand, HasReset, UsesState}, Evaluator, HasFeedback, HasObjective, HasScheduler
};

#[derive(Debug)]
/// The stage to probablistically disable a corpus entry.
/// This stage should be wrapped in a if stage and run only when the fuzzer perform restarting
/// The idea comes from `https://mschloegel.me/paper/schiller2023fuzzerrestarts.pdf`
pub struct CorpusPruning<EM> {
    /// The chance of retaining this corpus
    prob: f64,
    phantom: PhantomData<EM>,
}

impl<EM> CorpusPruning<EM> {
    pub fn new(prob: f64) -> Self {
        Self {
            prob,
            phantom: PhantomData,
        }
    }
}

impl<EM> Default for CorpusPruning<EM> {
    fn default() -> Self {
        Self::new(0.1)
    }
}

impl<EM> UsesState for CorpusPruning<EM>
where
    EM: UsesState,
{
    type State = EM::State;
}

impl<E, EM, Z> Stage<E, EM, Z> for CorpusPruning<EM>
where
    EM: UsesState,
    E: UsesState<State = Self::State>,
    Z: UsesState<State = Self::State> + HasScheduler + Evaluator<E, EM>,
    <Z as HasScheduler>::Scheduler: RemovableScheduler,
    Self::State: HasCorpus + HasRand,
{
    #[allow(clippy::cast_precision_loss)]
    fn perform(
        &mut self,
        fuzzer: &mut Z,
        executor: &mut E,
        state: &mut Self::State,
        manager: &mut EM,
    ) -> Result<(), Error> {
        // Iterate over every corpus entr
        let n_all = state.corpus().count_all();
        let n_enabled = state.corpus().count();

        let Some(currently_fuzzed_idx) = state.current_corpus_id()? else {
            return Err(Error::illegal_state("Not fuzzing any testcase".to_string()));
        };

        // eprintln!("Currently fuzzing {:#?}", currently_fuzzed_idx);

        let mut disabled_to_enabled = vec![];
        let mut enabled_to_disabled = vec![];
        // do it backwards so that the index won't change even after remove
        for i in (0..n_all).rev() {
            let r = state.rand_mut().below(100) as f64;
            if self.prob * 100_f64 < r {
                let idx = state.corpus().nth_from_all(i);

                // skip the currently fuzzed id; don't remove it
                // because else after restart we can't call currrent.next() to find the next testcase
                if idx == currently_fuzzed_idx {
                    // eprintln!("skipping {:#?}", idx);
                    continue;
                }

                let removed = state.corpus_mut().remove(idx)?;
                fuzzer
                    .scheduler_mut()
                    .on_remove(state, idx, &Some(removed.clone()))?;
                // because [n_enabled, n_all) is disabled testcases
                // and [0, n_enabled) is enabled testcases
                if i >= n_enabled {
                    // we are moving disabled to enabled now
                    disabled_to_enabled.push(removed);
                } else {
                    // we are moving enabled to disabled now
                    enabled_to_disabled.push(removed);
                }
            }
        }

        // Actually move them
        for mut testcase in disabled_to_enabled {
            let input = testcase.load_input(state.corpus())?;
            fuzzer.add_input(state, executor, manager, input.clone())?;
        }

        for mut testcase in enabled_to_disabled {
            let input = testcase.load_input(state.corpus())?;
            fuzzer.add_disabled_input(state, input.clone())?;
            // fuzzer.scheduler_mut().on_add(state, new_idx)?;
        }
        log::info!(
            "There was {}, and we retained {} corpura",
            n_all,
            state.corpus().count()
        );
        Ok(())
    }

    fn should_restart(&mut self, _state: &mut Self::State) -> Result<bool, Error> {
        // Not executing the target, so restart safety is not needed
        Ok(true)
    }
    fn clear_progress(&mut self, _state: &mut Self::State) -> Result<(), Error> {
        // Not executing the target, so restart safety is not needed
        Ok(())
    }
}

/// A stage for conditional restart
#[derive(Debug, Default)]
#[cfg(feature = "std")]
pub struct RestartStage<E, EM, Z> {
    phantom: PhantomData<(E, EM, Z)>,
}

#[cfg(feature = "std")]
impl<E, EM, Z> UsesState for RestartStage<E, EM, Z>
where
    E: UsesState,
{
    type State = E::State;
}

#[cfg(feature = "std")]
impl<E, EM, Z> Stage<E, EM, Z> for RestartStage<E, EM, Z>
where
    Self::State: HasReset,
    E: UsesState,
    EM: UsesState<State = Self::State> + EventRestarter,
    Z: UsesState<State = Self::State> + HasFeedback + HasObjective,
{
    #[allow(unreachable_code)]
    fn perform(
        &mut self,
        fuzzer: &mut Z,
        _executor: &mut E,
        state: &mut Self::State,
        manager: &mut EM,
    ) -> Result<(), Error> {
        log::info!("Restarting!");
        state.reset();
        fuzzer.feedback_mut().init_state(state)?;
        fuzzer.objective_mut().init_state(state)?;

        manager.on_restart(state).unwrap();

        std::process::exit(0);
        Ok(())
    }

    fn should_restart(&mut self, _state: &mut Self::State) -> Result<bool, Error> {
        Ok(true)
    }

    fn clear_progress(&mut self, _state: &mut Self::State) -> Result<(), Error> {
        Ok(())
    }
}

#[cfg(feature = "std")]
impl<E, EM, Z> RestartStage<E, EM, Z>
where
    E: UsesState,
{
    /// Constructor for this conditionally enabled stage.
    /// If the closure returns true, the wrapped stage will be executed, else it will be skipped.
    #[must_use]
    pub fn new() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}
