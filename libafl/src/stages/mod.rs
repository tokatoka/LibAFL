/*!
A [`Stage`] is a technique used during fuzzing, working on one [`crate::corpus::Corpus`] entry, and potentially altering it or creating new entries.
A well-known [`Stage`], for example, is the mutational stage, running multiple [`crate::mutators::Mutator`]s against a [`crate::corpus::Testcase`], potentially storing new ones, according to [`crate::feedbacks::Feedback`].
Other stages may enrich [`crate::corpus::Testcase`]s with metadata.
*/

/// Mutational stage is the normal fuzzing stage.
pub mod mutational;
use core::marker::PhantomData;

pub use mutational::{MutationalStage, StdMutationalStage};

pub mod tmin;
pub use tmin::{
    MapEqualityFactory, MapEqualityFeedback, StdTMinMutationalStage, TMinMutationalStage,
};

pub mod push;

pub mod tracing;
pub use tracing::{ShadowTracingStage, TracingStage};

pub mod calibrate;
pub use calibrate::CalibrationStage;

pub mod power;
pub use power::{PowerMutationalStage, StdPowerMutationalStage};

pub mod generalization;
pub use generalization::GeneralizationStage;

pub mod owned;
pub use owned::StagesOwnedList;

#[cfg(feature = "std")]
pub mod concolic;
#[cfg(feature = "std")]
pub use concolic::ConcolicTracingStage;
#[cfg(feature = "std")]
pub use concolic::SimpleConcolicMutationalStage;

#[cfg(feature = "std")]
pub mod sync;

#[cfg(feature = "std")]
pub use sync::*;

use self::push::PushStage;
use crate::executors::Executor;
use crate::prelude::State;
use crate::{
    executors::HasObservers,
    inputs::Input,
    state::{HasClientPerfMonitor, HasCorpus},
    Error, Evaluator, ExecutesInput, Fuzzer,
};

/// A stage is one step in the fuzzing process.
/// Multiple stages will be scheduled one by one for each input.
pub trait Stage {
    type Input: Input;
    type State: HasClientPerfMonitor + HasCorpus<Input = Self::Input>;
    type Executor;

    /// Run the stage
    fn perform<
        EM,
        Z: Evaluator<Input = Self::Input, State = Self::State, Executor = Self::Executor>,
    >(
        &mut self,
        fuzzer: &Z,
        executor: &mut Self::Executor,
        state: &mut Self::State,
        manager: &mut EM,
        corpus_idx: usize,
    ) -> Result<(), Error>;
}

/// A tuple holding all `Stages` used for fuzzing.
pub trait StagesTuple {
    type Executor: Executor<State = Self::State>;
    type State: State;

    /// Performs all `Stages` in this tuple
    fn perform_all<EM, Z>(
        &mut self,
        fuzzer: &mut Z,
        executor: &mut Self::Executor,
        state: &mut Self::State,
        manager: &mut EM,
        corpus_idx: usize,
    ) -> Result<(), Error>;
}

impl StagesTuple for () {
    fn perform_all<EM, Z>(
        &mut self,
        _: &mut Z,
        _: &mut Self::Executor,
        _: &mut Self::State,
        _: &mut EM,
        _: usize,
    ) -> Result<(), Error> {
        Ok(())
    }
}

impl<Head, Tail> StagesTuple for (Head, Tail)
where
    Head: Stage,
    Tail: StagesTuple<Executor = Self::Executor, State = Self::State>,
{
    fn perform_all<EM, Z>(
        &mut self,
        fuzzer: &mut Z,
        executor: &mut Self::Executor,
        state: &mut Self::State,
        manager: &mut EM,
        corpus_idx: usize,
    ) -> Result<(), Error> {
        // Perform the current stage
        self.0
            .perform(fuzzer, executor, state, manager, corpus_idx)?;

        // Execute the remaining stages
        self.1
            .perform_all(fuzzer, executor, state, manager, corpus_idx)
    }
}

/// A [`Stage`] that will call a closure
#[derive(Debug)]
pub struct ClosureStage<CB, EM, Z>
where
    CB: FnMut(&mut Z, &mut <Self as Stage>::Executor, &mut <Self as Stage>::State, &mut EM),
{
    closure: CB,
    phantom: PhantomData<(EM, Z)>,
}

impl<CB, EM, Z> Stage for ClosureStage<CB, EM, Z>
where
    CB: FnMut(&mut Z, &mut Self::Executor, &mut Self::State, &mut EM, usize) -> Result<(), Error>,
    Z: Fuzzer<
        Executor = Self::Executor,
        Input = Self::Input,
        State = Self::State,
        EventManager = EM,
    >,
{
    fn perform(
        &mut self,
        fuzzer: &mut Z,
        executor: &mut Self::Executor,
        state: &mut Self::State,
        manager: &mut EM,
        corpus_idx: usize,
    ) -> Result<(), Error> {
        Ok(())
        //(self.closure)(fuzzer, executor, state, manager, corpus_idx)
    }
}

/// A stage that takes a closure
impl<CB, EM, Z> ClosureStage<CB, EM, Z> {
    /// Create a new [`ClosureStage`]
    #[must_use]
    pub fn new(closure: CB) -> Self {
        Self {
            closure,
            phantom: PhantomData,
        }
    }
}

impl<CB, EM, Z> From<CB> for ClosureStage<CB, EM, Z>
where
    CB: FnMut(
        &mut Z,
        &mut <Self as Stage>::Executor,
        &mut <Self as Stage>::State,
        &mut EM,
        usize,
    ) -> Result<(), Error>,
{
    #[must_use]
    fn from(closure: CB) -> Self {
        Self::new(closure)
    }
}

/// Allows us to use a [`push::PushStage`] as a normal [`Stage`]
#[allow(clippy::type_complexity)]
#[derive(Debug)]
pub struct PushStageAdapter<PS>
where
    PS: PushStage,
{
    push_stage: PS,
}

impl<PS> PushStageAdapter<PS>
where
    PS: PushStage,
{
    /// Create a new [`PushStageAdapter`], wrapping the given [`PushStage`]
    /// to be used as a normal [`Stage`]
    #[must_use]
    pub fn new(push_stage: PS) -> Self {
        Self { push_stage }
    }
}

impl<PS> Stage for PushStageAdapter<PS> {
    fn perform<EM, Z>(
        &mut self,
        fuzzer: &mut Z,
        executor: &mut Self::Executor,
        state: &mut Self::State,
        event_mgr: &mut EM,
        corpus_idx: usize,
    ) -> Result<(), Error> {
        let push_stage = &mut self.push_stage;

        push_stage.set_current_corpus_idx(corpus_idx);

        push_stage.init(fuzzer, state, event_mgr, executor.observers_mut())?;

        loop {
            let input =
                match push_stage.pre_exec(fuzzer, state, event_mgr, executor.observers_mut()) {
                    Some(Ok(next_input)) => next_input,
                    Some(Err(err)) => return Err(err),
                    None => break,
                };

            let exit_kind = fuzzer.execute_input(state, executor, event_mgr, &input)?;

            push_stage.post_exec(
                fuzzer,
                state,
                event_mgr,
                executor.observers_mut(),
                input,
                exit_kind,
            )?;
        }

        self.push_stage
            .deinit(fuzzer, state, event_mgr, executor.observers_mut())
    }
}

/// The decision if the [`SkippableStage`] should be skipped
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum SkippableStageDecision {
    /// Return to indicate that this [`Stage`] should be executed
    Perform,
    /// Return to indicate that this [`Stage`] should be skipped
    Skip,
}

impl From<bool> for SkippableStageDecision {
    fn from(b: bool) -> SkippableStageDecision {
        if b {
            SkippableStageDecision::Perform
        } else {
            SkippableStageDecision::Skip
        }
    }
}

/// The [`SkippableStage`] wraps any [`Stage`] so that it can be skipped, according to a condition.
#[derive(Debug, Clone)]
pub struct SkippableStage<CD, E, EM, S, ST, Z>
where
    CD: FnMut(&mut S) -> SkippableStageDecision,
    ST: Stage<Executor = E, State = S>,
{
    wrapped_stage: ST,
    condition: CD,
    phantom: PhantomData<(E, EM, S, Z)>,
}

impl<CD, E, EM, S, ST, Z> SkippableStage<CD, E, EM, S, ST, Z>
where
    CD: FnMut(&mut S) -> SkippableStageDecision,
    ST: Stage<Executor = E, State = S>,
{
    /// Create a new [`SkippableStage`]
    pub fn new(wrapped_stage: ST, condition: CD) -> Self {
        Self {
            wrapped_stage,
            condition,
            phantom: PhantomData,
        }
    }
}

impl<CD, E, EM, S, ST, Z> Stage for SkippableStage<CD, E, EM, S, ST, Z>
where
    CD: FnMut(&mut S) -> SkippableStageDecision,
    ST: Stage<Executor = E, State = S>,
{
    /// Run the stage
    #[inline]
    fn perform(
        &mut self,
        fuzzer: &mut Z,
        executor: &mut E,
        state: &mut S,
        manager: &mut EM,
        corpus_idx: usize,
    ) -> Result<(), Error> {
        let condition = &mut self.condition;
        if condition(state) == SkippableStageDecision::Perform {
            self.wrapped_stage
                .perform(fuzzer, executor, state, manager, corpus_idx)
        } else {
            Ok(())
        }
    }
}

/// `Stage` Python bindings
#[cfg(feature = "python")]
#[allow(missing_docs)]
pub mod pybind {
    use alloc::vec::Vec;

    use pyo3::prelude::*;

    use crate::{
        events::pybind::PythonEventManager,
        executors::pybind::PythonExecutor,
        fuzzer::pybind::{PythonStdFuzzer, PythonStdFuzzerWrapper},
        stages::{mutational::pybind::PythonStdMutationalStage, Stage, StagesTuple},
        state::pybind::{PythonStdState, PythonStdStateWrapper},
        Error,
    };

    #[derive(Clone, Debug)]
    pub struct PyObjectStage {
        inner: PyObject,
    }

    impl PyObjectStage {
        #[must_use]
        pub fn new(obj: PyObject) -> Self {
            PyObjectStage { inner: obj }
        }
    }

    impl Stage<PythonExecutor, PythonEventManager, PythonStdState, PythonStdFuzzer> for PyObjectStage {
        #[inline]
        fn perform(
            &mut self,
            fuzzer: &mut PythonStdFuzzer,
            executor: &mut PythonExecutor,
            state: &mut PythonStdState,
            manager: &mut PythonEventManager,
            corpus_idx: usize,
        ) -> Result<(), Error> {
            Python::with_gil(|py| -> PyResult<()> {
                self.inner.call_method1(
                    py,
                    "perform",
                    (
                        PythonStdFuzzerWrapper::wrap(fuzzer),
                        executor.clone(),
                        PythonStdStateWrapper::wrap(state),
                        manager.clone(),
                        corpus_idx,
                    ),
                )?;
                Ok(())
            })?;
            Ok(())
        }
    }

    #[derive(Clone, Debug)]
    pub enum PythonStageWrapper {
        StdMutational(Py<PythonStdMutationalStage>),
        Python(PyObjectStage),
    }

    /// Stage Trait binding
    #[pyclass(unsendable, name = "Stage")]
    #[derive(Clone, Debug)]
    pub struct PythonStage {
        wrapper: PythonStageWrapper,
    }

    macro_rules! unwrap_me_mut {
        ($wrapper:expr, $name:ident, $body:block) => {
            crate::unwrap_me_mut_body!($wrapper, $name, $body, PythonStageWrapper,
                { StdMutational },
                {
                    Python(py_wrapper) => {
                        let $name = py_wrapper;
                        $body
                    }
                }
            )
        };
    }

    #[pymethods]
    impl PythonStage {
        #[staticmethod]
        #[must_use]
        pub fn new_std_mutational(
            py_std_havoc_mutations_stage: Py<PythonStdMutationalStage>,
        ) -> Self {
            Self {
                wrapper: PythonStageWrapper::StdMutational(py_std_havoc_mutations_stage),
            }
        }

        #[staticmethod]
        #[must_use]
        pub fn new_py(obj: PyObject) -> Self {
            Self {
                wrapper: PythonStageWrapper::Python(PyObjectStage::new(obj)),
            }
        }

        #[must_use]
        pub fn unwrap_py(&self) -> Option<PyObject> {
            match &self.wrapper {
                PythonStageWrapper::Python(pyo) => Some(pyo.inner.clone()),
                PythonStageWrapper::StdMutational(_) => None,
            }
        }
    }

    impl Stage<PythonExecutor, PythonEventManager, PythonStdState, PythonStdFuzzer> for PythonStage {
        #[inline]
        #[allow(clippy::let_and_return)]
        fn perform(
            &mut self,
            fuzzer: &mut PythonStdFuzzer,
            executor: &mut PythonExecutor,
            state: &mut PythonStdState,
            manager: &mut PythonEventManager,
            corpus_idx: usize,
        ) -> Result<(), Error> {
            unwrap_me_mut!(self.wrapper, s, {
                s.perform(fuzzer, executor, state, manager, corpus_idx)
            })
        }
    }

    #[derive(Clone, Debug)]
    #[pyclass(unsendable, name = "StagesTuple")]
    pub struct PythonStagesTuple {
        list: Vec<PythonStage>,
    }

    #[pymethods]
    impl PythonStagesTuple {
        #[new]
        fn new(list: Vec<PythonStage>) -> Self {
            Self { list }
        }

        fn len(&self) -> usize {
            self.list.len()
        }

        fn __getitem__(&self, idx: usize) -> PythonStage {
            self.list[idx].clone()
        }
    }

    impl StagesTuple<PythonExecutor, PythonEventManager, PythonStdState, PythonStdFuzzer>
        for PythonStagesTuple
    {
        fn perform_all(
            &mut self,
            fuzzer: &mut PythonStdFuzzer,
            executor: &mut PythonExecutor,
            state: &mut PythonStdState,
            manager: &mut PythonEventManager,
            corpus_idx: usize,
        ) -> Result<(), Error> {
            for s in &mut self.list {
                s.perform(fuzzer, executor, state, manager, corpus_idx)?;
            }
            Ok(())
        }
    }

    /// Register the classes to the python module
    pub fn register(_py: Python, m: &PyModule) -> PyResult<()> {
        m.add_class::<PythonStage>()?;
        m.add_class::<PythonStagesTuple>()?;
        Ok(())
    }
}
