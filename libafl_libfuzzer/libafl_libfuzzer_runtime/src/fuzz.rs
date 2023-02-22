use core::ffi::c_int;
use std::{
    net::TcpListener,
    time::{SystemTime, UNIX_EPOCH},
};

use libafl::{
    bolts::{
        core_affinity::Cores,
        launcher::Launcher,
        shmem::{ShMemProvider, StdShMemProvider},
    },
    corpus::Corpus,
    events::{EventConfig, ProgressReporter, SimpleEventManager, SimpleRestartingEventManager},
    executors::ExitKind,
    inputs::UsesInput,
    monitors::{tui::TuiMonitor, MultiMonitor, SimpleMonitor},
    stages::StagesTuple,
    state::{HasClientPerfMonitor, HasExecutions, HasMetadata, HasSolutions, UsesState},
    Error, Fuzzer,
};

use crate::{feedbacks::LibfuzzerCrashCauseMetadata, fuzz_with, options::LibfuzzerOptions};

fn do_fuzz<F, ST, E, S, EM>(
    options: &LibfuzzerOptions,
    fuzzer: &mut F,
    stages: &mut ST,
    executor: &mut E,
    state: &mut S,
    mgr: &mut EM,
) -> Result<(), Error>
where
    F: Fuzzer<E, EM, ST, State = S>,
    S: HasClientPerfMonitor + HasMetadata + HasExecutions + UsesInput + HasSolutions,
    E: UsesState<State = S>,
    EM: ProgressReporter<State = S>,
    ST: StagesTuple<E, EM, S, F>,
{
    if let Some(solution) = state.solutions().last() {
        let kind = state
            .solutions()
            .get(solution)
            .expect("Last solution was not available")
            .borrow()
            .metadata()
            .get::<LibfuzzerCrashCauseMetadata>()
            .expect("Crash cause not attached to solution")
            .kind();
        let mut halt = false;
        match kind {
            ExitKind::Oom if !options.ignore_ooms() => halt = true,
            ExitKind::Crash if !options.ignore_crashes() => halt = true,
            ExitKind::Timeout if !options.ignore_timeouts() => halt = true,
            _ => {
                log::info!("Ignoring {kind:?} according to requested ignore rules.");
            }
        }
        if halt {
            log::info!("Halting; the error on the next line is actually okay. :)");
            return Err(Error::shutting_down());
        }
    }
    fuzzer.fuzz_loop(stages, executor, state, mgr)?;
    Ok(())
}

pub fn fuzz(
    options: LibfuzzerOptions,
    harness: &extern "C" fn(*const u8, usize) -> c_int,
) -> Result<(), Error> {
    if let Some(forks) = options.forks() {
        let mut shmem_provider = StdShMemProvider::new().expect("Failed to init shared memory");
        if forks == 1 {
            fuzz_with!(options, harness, do_fuzz, |fuzz_single| {
                let monitor = MultiMonitor::with_time(
                    |s| eprintln!("{s}"),
                    SystemTime::now().duration_since(UNIX_EPOCH).unwrap(),
                );
                let (state, mgr): (
                    Option<StdState<_, _, _, _>>,
                    SimpleRestartingEventManager<_, StdState<_, _, _, _>, _>,
                ) = match SimpleRestartingEventManager::launch(monitor, &mut shmem_provider) {
                    // The restarting state will spawn the same process again as child, then restarted it each time it crashes.
                    Ok(res) => res,
                    Err(err) => match err {
                        Error::ShuttingDown => {
                            return Ok(());
                        }
                        _ => {
                            panic!("Failed to setup the restarter: {err}");
                        }
                    },
                };
                crate::start_fuzzing_single(fuzz_single, state, mgr)
            })
        } else {
            fuzz_with!(options, harness, do_fuzz, |mut run_client| {
                let cores = Cores::from((0..forks).collect::<Vec<_>>());
                let broker_port = TcpListener::bind("0.0.0.0:0")?.local_addr().unwrap().port();

                let monitor = TuiMonitor::new(options.fuzzer_name().to_string(), true);

                match Launcher::builder()
                    .shmem_provider(shmem_provider)
                    .configuration(EventConfig::from_name(options.fuzzer_name()))
                    .monitor(monitor)
                    .run_client(&mut run_client)
                    .cores(&cores)
                    .broker_port(broker_port)
                    // TODO .remote_broker_addr(opt.remote_broker_addr)
                    .stdout_file(Some("/dev/null"))
                    .build()
                    .launch()
                {
                    Ok(()) => (),
                    Err(Error::ShuttingDown) => println!("Fuzzing stopped by user. Good bye."),
                    res @ Err(_) => return res,
                }
                Ok(())
            })
        }
    } else {
        fuzz_with!(options, harness, do_fuzz, |fuzz_single| {
            let mgr = SimpleEventManager::new(SimpleMonitor::new(|s| eprintln!("{s}")));
            crate::start_fuzzing_single(fuzz_single, None, mgr)
        })
    }
}
