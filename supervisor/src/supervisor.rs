//! A supervised process

use libc::c_int;
use std::ffi::OsStr;
use std::process::{Child, Command, ExitStatus};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, Once};
use std::time::Duration;
use std::{mem, panic, ptr, thread};

/// Panic and signal watchdog
#[derive(Debug)]
pub struct PanicWatchdog {
    _no_foreign_init: (),
}
impl PanicWatchdog {
    ///  a watchdog which sets the
    pub fn start() -> Self {
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
        Self { _no_foreign_init: () }
    }

    /// If the watchdog has an alert
    #[must_use]
    pub fn has_alert(&self) -> bool {
        Self::alert_flag().load(Ordering::SeqCst)
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

/// A supervised process
#[derive(Debug, Clone)]
pub struct Process {
    /// Whether the process was intentionally killed or not
    killed: Arc<AtomicBool>,
    /// The child command
    child: Arc<Mutex<Child>>,
}
impl Process {
    /// Spin interval to avoid a tight loop
    const SPIN_INTERVAL: Duration = Duration::from_millis(333);
    /// Gracetime for the child to handle a term signal
    const TERM_GRACETIME: Duration = Duration::from_secs(2);

    /// Spawns a supervised process
    pub fn spawn<Exec, Args, Arg>(executable: Exec, args: Args) -> Self
    where
        Exec: AsRef<OsStr>,
        Args: IntoIterator<Item = Arg>,
        Arg: AsRef<OsStr>,
    {
        // Spawn child and init self
        let child = Command::new(executable).args(args).spawn().expect("failed to spawn child process");
        Self { killed: Arc::default(), child: Arc::new(Mutex::new(child)) }
    }

    /// Gets the exit status of the child process, or `None` if it is still running
    pub fn status(&self) -> Option<ExitStatus> {
        let mut child = self.child.lock().expect("failed to lock mutex");
        child.try_wait().expect("failed to query child process status")
    }

    /// Ensures that the child process does never exit unless it has been explicitly killed
    pub fn expect_never(&self) {
        loop {
            // Query child status
            match self.status() {
                None => thread::sleep(Self::SPIN_INTERVAL),
                Some(_) if self.killed.load(Ordering::SeqCst) => return,
                Some(e) => panic!("child process exited: {}", e),
            }
        }
    }

    /// Ensures that the child process exits with status `0`
    pub fn expect_zero(&self) {
        loop {
            // Query child status
            match self.status() {
                None => thread::sleep(Self::SPIN_INTERVAL),
                Some(e) if e.code() == Some(0) => return,
                Some(_) if self.killed.load(Ordering::SeqCst) => return,
                Some(e) => panic!("child process exited with non-zero status code: {}", e),
            }
        }
    }

    /// Best-effort to kill the child process
    pub fn kill(&self) {
        // Send sigterm to the child process
        self.killed.store(true, Ordering::SeqCst);
        let pid = self.child.lock().expect("failed to lock mutex").id();
        let _ = unsafe { libc::kill(pid as _, libc::SIGTERM) };

        // Give the process some time to exit after sigterm
        let poll_times = Self::TERM_GRACETIME.as_secs_f64() / Self::SPIN_INTERVAL.as_secs_f64();
        for _ in 0..poll_times.ceil() as u64 {
            // Exit if the process is dead
            if let Some(_) = self.status() {
                // Process exited after sigterm
                return;
            };
        }

        // Send a sigkill to the child
        let _ = unsafe { libc::kill(pid as _, libc::SIGKILL) };
    }
}
