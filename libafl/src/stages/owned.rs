//! A dynamic collection of owned Stages

use alloc::{boxed::Box, vec::Vec};

use crate::{
    bolts::anymap::AsAny,
    prelude::State,
    stages::{Stage, StagesTuple},
    Error,
};

/// Combine `Stage` and `AsAny`
pub trait AnyStage: Stage + AsAny {}

/// An owned list of `Observer` trait objects
#[derive(Default)]
#[allow(missing_debug_implementations)]
pub struct StagesOwnedList {
    /// The named trait objects map
    pub list: Vec<
        Box<
            dyn AnyStage<
                Input = <<Self as StagesTuple>::State as State>::Input,
                State = <Self as StagesTuple>::State,
                Executor = <Self as StagesTuple>::Executor,
            >,
        >,
    >,
}

impl StagesTuple for StagesOwnedList {
    fn perform_all<EM, Z>(
        &mut self,
        fuzzer: &mut Z,
        executor: &mut Self::Executor,
        state: &mut Self::State,
        manager: &mut EM,
        corpus_idx: usize,
    ) -> Result<(), Error> {
        for s in &mut self.list {
            s.perform(fuzzer, executor, state, manager, corpus_idx)?;
        }
        Ok(())
    }
}

impl StagesOwnedList {
    /// Create a new instance
    #[must_use]
    pub fn new<EM>(
        list: Vec<
            Box<
                dyn AnyStage<
                    Input = <<Self as StagesTuple>::State as State>::Input,
                    State = <Self as StagesTuple>::State,
                    Executor = <Self as StagesTuple>::Executor,
                >,
            >,
        >,
    ) -> Self {
        Self { list }
    }
}
