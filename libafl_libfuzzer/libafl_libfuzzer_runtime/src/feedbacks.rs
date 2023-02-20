use alloc::rc::Rc;
use core::{cell::RefCell, fmt::Debug};

use libafl::{
    alloc,
    bolts::tuples::Named,
    corpus::Testcase,
    events::EventFirer,
    executors::ExitKind,
    feedbacks::Feedback,
    impl_serdeany,
    inputs::UsesInput,
    observers::ObserversTuple,
    state::{HasClientPerfMonitor, HasMetadata},
    Error,
};
use libafl_targets::OOMFeedback;
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct LibfuzzerKeepFeedback {
    keep: Rc<RefCell<bool>>,
}

impl LibfuzzerKeepFeedback {
    pub fn new() -> Self {
        Self {
            keep: Rc::new(RefCell::new(false)),
        }
    }

    pub fn keep(&self) -> Rc<RefCell<bool>> {
        self.keep.clone()
    }
}

impl Named for LibfuzzerKeepFeedback {
    fn name(&self) -> &str {
        "libfuzzer-keep"
    }
}

impl<S> Feedback<S> for LibfuzzerKeepFeedback
where
    S: UsesInput + HasClientPerfMonitor,
{
    fn is_interesting<EM, OT>(
        &mut self,
        _state: &mut S,
        _manager: &mut EM,
        _input: &S::Input,
        _observers: &OT,
        _exit_kind: &ExitKind,
    ) -> Result<bool, Error>
    where
        EM: EventFirer<State = S>,
        OT: ObserversTuple<S>,
    {
        Ok(*self.keep.borrow())
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct LibfuzzerCrashCauseMetadata {
    kind: ExitKind,
}

impl_serdeany!(LibfuzzerCrashCauseMetadata);

impl LibfuzzerCrashCauseMetadata {
    pub fn kind(&self) -> ExitKind {
        self.kind
    }
}

#[derive(Debug)]
pub struct LibfuzzerCrashCauseFeedback {
    exit_kind: ExitKind,
}

impl LibfuzzerCrashCauseFeedback {
    pub fn new() -> Self {
        Self {
            exit_kind: ExitKind::Ok,
        }
    }
}

impl Named for LibfuzzerCrashCauseFeedback {
    fn name(&self) -> &str {
        "crash-cause"
    }
}

impl<S> Feedback<S> for LibfuzzerCrashCauseFeedback
where
    S: UsesInput + HasClientPerfMonitor,
{
    fn is_interesting<EM, OT>(
        &mut self,
        _state: &mut S,
        _manager: &mut EM,
        _input: &S::Input,
        _observers: &OT,
        exit_kind: &ExitKind,
    ) -> Result<bool, Error>
    where
        EM: EventFirer<State = S>,
        OT: ObserversTuple<S>,
    {
        self.exit_kind = *exit_kind;
        Ok(false)
    }

    fn append_metadata<OT>(
        &mut self,
        _state: &mut S,
        _observers: &OT,
        testcase: &mut Testcase<S::Input>,
    ) -> Result<(), Error>
    where
        OT: ObserversTuple<S>,
    {
        match self.exit_kind {
            ExitKind::Crash | ExitKind::Oom if OOMFeedback::oomed() => {
                if let Some(filename) = testcase.filename_mut() {
                    *filename = format!("oom-{}", filename);
                }
                testcase.metadata_mut().insert(LibfuzzerCrashCauseMetadata {
                    kind: ExitKind::Oom,
                });
            }
            ExitKind::Crash | ExitKind::Oom => {
                if let Some(filename) = testcase.filename_mut() {
                    *filename = format!("crash-{}", filename);
                }
                testcase.metadata_mut().insert(LibfuzzerCrashCauseMetadata {
                    kind: ExitKind::Crash,
                });
            }
            ExitKind::Timeout => {
                if let Some(filename) = testcase.filename_mut() {
                    *filename = format!("timeout-{}", filename);
                }
                testcase.metadata_mut().insert(LibfuzzerCrashCauseMetadata {
                    kind: ExitKind::Timeout,
                });
            }
            _ => {
                testcase.metadata_mut().insert(LibfuzzerCrashCauseMetadata {
                    kind: self.exit_kind,
                });
            }
        }
        Ok(())
    }
}
