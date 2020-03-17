#[macro_use]
extern crate windows_service;

use std::cell::RefCell;
use std::ffi::OsString;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::process::{Child, Command, Stdio};
use std::rc::Rc;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::time::Duration;

use windows_service::{
    define_windows_service,
    service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
        ServiceType,
    },
    service_control_handler::{self, ServiceControlHandlerResult},
    service_dispatcher,
};

const SERVICE_NAME: &str = "exe_proxy";

#[cfg(windows)]
fn main() -> windows_service::Result<()> {
    if std::env::args().len() < 2 {
        panic!("args is empty")
    }
    define_windows_service!(ffi_service_main, run);
    service_dispatcher::start(SERVICE_NAME, ffi_service_main)?;
    Ok(())
//    let (shutdown_tx, shutdown_rx) = mpsc::channel();
//    match start_proxy(&shutdown_rx) {
//        Ok(_) => {
//            println!("everything is ok")
//        },
//        Err(e) => println!("{:?}",e.to_string()),
//    };
//    Ok(())
}

fn run(args: Vec<OsString>) {
    if let Err(e) = run_service() {
        panic!("error");
    }
}

fn run_service() -> Result<(), Box<dyn std::error::Error>> {
    // Create a channel to be able to poll a stop event from the service worker loop.

    let (shutdown_tx, shutdown_rx) = mpsc::channel();
    // Define system service event handler that will be receiving service events.
    let event_handler = move |control_event| -> ServiceControlHandlerResult {
        match control_event {
            // Notifies a service to report its current status information to the service
            // control manager. Always return NoError even if not implemented.
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,

            // Handle stop
            ServiceControl::Stop => {
                shutdown_tx.send(()).unwrap();
                ServiceControlHandlerResult::NoError
            }
            _ => ServiceControlHandlerResult::NoError,
        }
    };

    // Register system service event handler.
    // The returned status handle should be used to report service status changes to the system.
    let status_handle = service_control_handler::register(SERVICE_NAME, event_handler)?;

    const SERVICE_TYPE: ServiceType = ServiceType::OWN_PROCESS;
    // Tell the system that service is running
    status_handle.set_service_status(ServiceStatus {
        service_type: SERVICE_TYPE,
        current_state: ServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
    })?;
    match start_child(&shutdown_rx) {
        Ok(_) => {},
        Err(e) => {
            println!("run process err {:?}",e);
        }
    };
    // Tell the system that service has stopped.
    status_handle.set_service_status(ServiceStatus {
        service_type: SERVICE_TYPE,
        current_state: ServiceState::Stopped,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
    })?;

    Ok(())
}

fn start_child(shutdown_rx: &Receiver<()>) -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let current=std::env::current_exe()?;
    let process =current .with_file_name(OsString::from(args.next().unwrap()));
    let mut command = Command::new(process);
    let mut out=File::create(current.with_file_name("log.log"))?;
    let mut err=File::create(current.with_file_name("err.log"))?;
    command.stdin(Stdio::piped());
    command.stdout(out);
    command.stderr(err);
    match args.next() {
        Some(config) => {
            let config = std::env::current_exe()?.with_file_name(OsString::from(config));
            let mut content = String::new();
            let mut file = File::open(config)?;
            file.read_to_string(&mut content);
            for arg in content.split(' ') {
                command.arg(arg);
            }
        },
        None => {}
    }
    let mut child = command.spawn()?;
    let dur = Duration::from_millis(500);
    while let result = shutdown_rx.recv_timeout(dur) {
        match result {
            Ok(_) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
            _ => {},
        }
    };
    child.kill()?;
    Ok(())
}

#[cfg(not(windows))]
fn main() {
    panic!("This program is only intended to run on Windows.");
}