use core::{fmt::Debug, hash::Hash, marker::PhantomData};

use libafl::{
    bolts::AsIter,
    corpus::minimizer::{CorpusMinimizer, MapCorpusMinimizer},
    corpus::CorpusId,
    executors::{Executor, HasObservers},
    observers::MapObserver,
    schedulers::{Scheduler, TestcaseScore},
    stages::Stage,
    state::{HasClientPerfMonitor, HasCorpus, HasExecutions, HasMetadata, HasRand, UsesState},
    Error, HasScheduler,
};

#[derive(Clone, Debug)]
pub struct CMinStage<EM, Z, E, O, T, TS>
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

impl<EM, Z, E, O, T, TS> UsesState for CMinStage<EM, Z, E, O, T, TS>
where
    EM: UsesState,
    E: UsesState,
    E::State: HasCorpus + HasMetadata,
    TS: TestcaseScore<E::State>,
    T: Copy + Hash + Eq,
    for<'a> O: MapObserver<Entry = T> + AsIter<'a, Item = T>,
{
    type State = EM::State;
}

impl<EM, Z, E, O, T, TS> CMinStage<EM, Z, E, O, T, TS>
where
    EM: UsesState<State = Z::State>,
    Z: UsesState,
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

impl<EM, EX, Z, E, O, T, TS> Stage<EX, EM, Z> for CMinStage<EM, Z, E, O, T, TS>
where
    EM: UsesState<State = Z::State>,
    EX: UsesState<State = Z::State>,
    EX::State: HasCorpus + HasMetadata + HasExecutions,
    Z: UsesState,
    E: UsesState,
    E::State: HasCorpus + HasMetadata,
    TS: TestcaseScore<E::State>,
    T: Copy + Hash + Eq,
    for<'a> O: MapObserver<Entry = T> + AsIter<'a, Item = T>,
{
    /// Perform the actions for this stage
    fn perform(
        &mut self,
        fuzzer: &mut Z,
        executor: &mut EX,
        state: &mut EX::State,
        manager: &mut EM,
        corpus_idx: CorpusId,
    ) -> Result<(), Error> {
        //self.minimizer.minimize(fuzzer, executor, manager, state)?;

        Ok(())
    }
}
