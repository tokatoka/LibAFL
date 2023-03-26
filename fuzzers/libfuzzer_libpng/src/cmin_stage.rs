use core::{fmt::Debug, hash::Hash, marker::PhantomData};

use libafl::{
    bolts::AsIter,
    corpus::{
        minimizer::{CorpusMinimizer, MapCorpusMinimizer},
        CorpusId,
    },
    executors::{Executor, HasObservers},
    observers::MapObserver,
    schedulers::{RemovableScheduler, TestcaseScore},
    stages::Stage,
    state::{HasCorpus, HasMetadata, UsesState},
    Error, HasScheduler,
};

#[derive(Debug)]
pub struct CMinStage<E, EM, O, T, TS, Z>
where
    E: UsesState,
    E::State: HasCorpus + HasMetadata,
    TS: TestcaseScore<E::State>,
    T: Copy + Hash + Eq,
    for<'a> O: MapObserver<Entry = T> + AsIter<'a, Item = T>,
{
    minimizer: MapCorpusMinimizer<E, O, T, TS>,

    /// This marker is used to indicate that this struct owns data of referened types
    phantom: PhantomData<(EM, Z, E, O, T, TS)>,
}

impl<E, EM, O, T, TS, Z> UsesState for CMinStage<E, EM, O, T, TS, Z>
where
    E: UsesState,
    E::State: HasCorpus + HasMetadata,
    TS: TestcaseScore<E::State>,
    T: Copy + Hash + Eq,
    for<'a> O: MapObserver<Entry = T> + AsIter<'a, Item = T>,
{
    type State = E::State;
}

impl<E, EM, O, T, TS, Z> CMinStage<E, EM, O, T, TS, Z>
where
    E: UsesState,
    E::State: HasCorpus + HasMetadata,
    TS: TestcaseScore<E::State>,
    T: Copy + Hash + Eq,
    for<'a> O: MapObserver<Entry = T> + AsIter<'a, Item = T>,
{
    /// Create a new `CMinStage`
    pub fn new(minimizer: MapCorpusMinimizer<E, O, T, TS>) -> Result<Self, Error> {
        Ok(Self {
            minimizer,
            phantom: PhantomData,
        })
    }
}

impl<E, EM, O, T, TS, Z> Stage<E, EM, Z> for CMinStage<E, EM, O, T, TS, Z>
where
    E: UsesState,
    E::State: HasCorpus + HasMetadata,
    E: Executor<EM, Z> + HasObservers,
    EM: UsesState<State = E::State>,
    Z: UsesState<State = E::State> + HasScheduler,
    Z::Scheduler: RemovableScheduler,
    TS: TestcaseScore<E::State>,
    T: Copy + Hash + Eq,
    for<'a> O: MapObserver<Entry = T> + AsIter<'a, Item = T>,
{
    /// Perform the actions for this stage
    fn perform(
        &mut self,
        fuzzer: &mut Z,
        executor: &mut E,
        state: &mut E::State,
        manager: &mut EM,
        corpus_idx: CorpusId,
    ) -> Result<(), Error> {
        self.minimizer.minimize(fuzzer, executor, manager, state)?;

        Ok(())
    }
}
