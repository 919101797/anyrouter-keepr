#[cfg(target_os = "macos")]
use std::process::{Child, Command, Stdio};

#[cfg(windows)]
use std::sync::mpsc;
#[cfg(windows)]
use std::thread;
#[cfg(windows)]
use std::time::Duration;

#[cfg(windows)]
const ES_CONTINUOUS: u32 = 0x8000_0000;
#[cfg(windows)]
const ES_SYSTEM_REQUIRED: u32 = 0x0000_0001;

#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn SetThreadExecutionState(es_flags: u32) -> u32;
}

pub struct PowerGuard {
    #[cfg(target_os = "macos")]
    child: Child,
    #[cfg(windows)]
    stop_tx: Option<mpsc::Sender<()>>,
    #[cfg(windows)]
    handle: Option<thread::JoinHandle<()>>,
}

impl PowerGuard {
    pub fn acquire() -> Result<Self, String> {
        acquire_platform_guard()
    }
}

#[cfg(target_os = "macos")]
fn acquire_platform_guard() -> Result<PowerGuard, String> {
    let child = Command::new("caffeinate")
        .args(["-i", "-s"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|err| format!("failed to start caffeinate: {err}"))?;

    Ok(PowerGuard { child })
}

#[cfg(windows)]
fn acquire_platform_guard() -> Result<PowerGuard, String> {
    let (ready_tx, ready_rx) = mpsc::channel::<Result<(), String>>();
    let (stop_tx, stop_rx) = mpsc::channel::<()>();

    let handle = thread::Builder::new()
        .name("anyrouter-keeper-power".to_string())
        .spawn(move || {
            let state = ES_CONTINUOUS | ES_SYSTEM_REQUIRED;
            let previous = unsafe { SetThreadExecutionState(state) };
            if previous == 0 {
                let _ = ready_tx.send(Err(format!(
                    "SetThreadExecutionState failed: {}",
                    std::io::Error::last_os_error()
                )));
                return;
            }

            let _ = ready_tx.send(Ok(()));
            let _ = stop_rx.recv();
            unsafe {
                SetThreadExecutionState(ES_CONTINUOUS);
            }
        })
        .map_err(|err| format!("failed to start power guard thread: {err}"))?;

    match ready_rx.recv_timeout(Duration::from_secs(2)) {
        Ok(Ok(())) => Ok(PowerGuard {
            stop_tx: Some(stop_tx),
            handle: Some(handle),
        }),
        Ok(Err(error)) => {
            let _ = stop_tx.send(());
            let _ = handle.join();
            Err(error)
        }
        Err(error) => {
            let _ = stop_tx.send(());
            let _ = handle.join();
            Err(format!("power guard thread did not initialize: {error}"))
        }
    }
}

#[cfg(not(any(target_os = "macos", windows)))]
fn acquire_platform_guard() -> Result<PowerGuard, String> {
    Ok(PowerGuard {})
}

impl Drop for PowerGuard {
    fn drop(&mut self) {
        #[cfg(target_os = "macos")]
        {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }

        #[cfg(windows)]
        {
            if let Some(tx) = self.stop_tx.take() {
                let _ = tx.send(());
            }
            if let Some(handle) = self.handle.take() {
                let _ = handle.join();
            }
        }
    }
}
