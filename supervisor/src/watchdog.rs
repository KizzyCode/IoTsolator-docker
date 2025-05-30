//! Watchdog for panics, signals and child processes

use libc::c_int;
use std::ffi::OsStr;
use std::process::Command;
use std::sync::Once;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use std::{mem, panic, ptr, thread};

/// Panic and signal watchdog
#[derive(Debug, Clone, Copy)]
pub struct Watchdog {
    _no_foreign_init: (),
}
impl Watchdog {
    /// Spin interval used in poll-loops to avoid tight looping
    const SPIN_INTERVAL: Duration = Duration::from_millis(333);

    ///  a watchdog which sets the
    pub fn scope<Scope, OnAlert>(scope: Scope, on_alert: OnAlert)
    where
        Scope: FnOnce(Self) + Send + 'static,
        OnAlert: FnOnce(),
    {
        /// Ensure the handlers are setup only once
        static INIT_ONCE: Once = Once::new();
        INIT_ONCE.call_once(|| {
            // Set signal handlers
            unsafe { Self::set_signalhandler(libc::SIGINT, Self::onsignal) };
            unsafe { Self::set_signalhandler(libc::SIGTERM, Self::onsignal) };

            // Capture panics from other threads
            let original_hook = panic::take_hook();
            panic::set_hook(Box::new(move |panic_info| {
                // Call the original panic hook and signal alert
                original_hook(panic_info);
                Self::alert_flag().store(true, Ordering::SeqCst);
            }));
        });

        // Init pseudo-handle
        thread::spawn(|| scope(Self { _no_foreign_init: () }));
        while !Self::alert_flag().load(Ordering::SeqCst) {
            // Sleep to avoid a tight spin-loop
            thread::sleep(Self::SPIN_INTERVAL);
        }

        // Call on-alert closure
        on_alert();
    }

    /// Spawns a child and watches it to ensure it exits with the expected status (an expected status of `None` means
    /// the process should never exit)
    pub fn spawn_child<Exec, Args, Arg>(&self, exec: Exec, args: Args, expected_status: Option<i32>)
    where
        Exec: AsRef<OsStr> + Send + 'static,
        Args: IntoIterator<Item = Arg> + Send + 'static,
        Arg: AsRef<OsStr> + Send + 'static,
    {
        thread::spawn(move || {
            // Spawn child process
            let exec = exec.as_ref();
            let mut child = match Command::new(exec).args(args).spawn() {
                Ok(child_process) => child_process,
                Err(e) => panic!("failed to spawn {}: {e}", exec.display()),
            };

            // Wait for child process to reap it and check the exit status
            let status = child.wait().expect("failed to wait for child process");
            match (status.code(), expected_status) {
                (Some(status), Some(expected)) if status == expected => return,
                (status, _) => panic!("{} exited with status ({status:?})", exec.display()),
            }
        });
    }

    /// The static has-panic flag
    #[inline(always)]
    const fn alert_flag() -> &'static AtomicBool {
        /// Watchdog alert flag; if true, a panic occurred somewhere
        pub static FLAG: AtomicBool = AtomicBool::new(false);
        &FLAG
    }

    /// Sets a signal handler
    unsafe fn set_signalhandler(signal: c_int, handler: extern "C" fn(c_int)) {
        // Prepare arcane struct to register the signal handler
        // SAFETY: In reality, `libc::sigaction::sa_sigaction` is in fact a union where the first field is a function
        //  pointer to the signal handler, so we have to assume that it is safe to transmute a function pointer into
        //  this `libc`-mapped type.
        let sa_sigaction: libc::sighandler_t = unsafe { mem::transmute_copy(&handler) };
        let newaction = 'os_select: {
            #[cfg(target_os = "macos")]
            break 'os_select libc::sigaction { sa_sigaction, sa_mask: 0, sa_flags: 0 };

            #[cfg(target_os = "linux")]
            break 'os_select libc::sigaction {
                sa_sigaction,
                // SAFETY: `libc::sigset_t` is an int-array, so this should be safe
                sa_mask: unsafe { std::mem::MaybeUninit::zeroed().assume_init() },
                sa_flags: 0,
                sa_restorer: None,
            };

            #[cfg(not(any(target_os = "macos", target_os = "linux")))]
            compile_error!("unsupported target platform");
        };

        // Register the signal handler
        let result = unsafe { libc::sigaction(signal, &newaction, ptr::null_mut()) };
        assert_eq!(result, 0, "failed to register signal handler");
    }

    /// Sets the alert flag so the watcher can take actions
    extern "C" fn onsignal(_signal: c_int) {
        // Atomic, re-entrant signaling via alert flag
        Self::alert_flag().store(true, Ordering::SeqCst);
    }
}
