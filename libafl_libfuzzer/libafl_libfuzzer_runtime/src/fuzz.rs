use core::ffi::c_int;
use std::net::TcpListener;

use libafl::{
    bolts::{
        core_affinity::Cores,
        launcher::Launcher,
        shmem::{ShMemProvider, StdShMemProvider},
    },
    events::{EventConfig, ProgressReporter, SimpleEventManager, SimpleRestartingEventManager},
    inputs::UsesInput,
    monitors::{tui::TuiMonitor, SimpleMonitor},
    stages::StagesTuple,
    state::{HasClientPerfMonitor, HasExecutions, HasMetadata, UsesState},
    Error, Fuzzer,
};

use crate::{fuzz_with, options::LibfuzzerOptions};

fn do_fuzz<F, ST, E, S, EM>(
    fuzzer: &mut F,
    stages: &mut ST,
    executor: &mut E,
    state: &mut S,
    mgr: &mut EM,
) -> Result<(), Error>
where
    F: Fuzzer<E, EM, ST, State = S>,
    S: HasClientPerfMonitor + HasMetadata + HasExecutions + UsesInput,
    E: UsesState<State = S>,
    EM: ProgressReporter<State = S>,
    ST: StagesTuple<E, EM, S, F>,
{
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
                let monitor = SimpleMonitor::new(|s| eprintln!("{s}"));
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
