#[macro_use]
extern crate windows_service;

use std::cell::RefCell;
use std::ffi::OsString;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
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
use clap::{App, Arg};

const SERVICE_NAME: &str = "exe_proxy";

#[cfg(windows)]
fn main() -> windows_service::Result<()> {
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
            println!("run process err {:?}", e);
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

fn get_path_from_name(input:&str) ->  Result<PathBuf, std::io::Error> {
    let current = std::env::current_exe()?;
    let path = Path::new(input);
    let real_path=if path.exists() {
        path.to_path_buf()
    } else {
        current.with_file_name(path.as_os_str())
    };
    Ok(real_path)
}

fn start_child(shutdown_rx: &Receiver<()>) -> Result<(), Box<dyn std::error::Error>> {
    let app = App::new("service manager")
        .arg(Arg::with_name("exe")
            .short("e")
            .long("executor")
            .multiple(true)
            .required(true)
            .help("executor path")
            .takes_value(true))
        .arg(Arg::with_name("file")
            .short("f")
            .long("file")
            .multiple(true)
            .required(false)
            .help("config an real process config file")
            .takes_value(true))
        .get_matches();
    let err=std::io::Error::new(std::io::ErrorKind::NotFound,"no arg found");
    let exes=app.values_of("exe").ok_or(err)?;
    let mut files =app.values_of("file").unwrap_or_default();
    let mut process=vec![];
    for exe in exes {
        let exe_path=get_path_from_name(exe)?;
        let mut command = Command::new(exe_path);
        command.stdin(Stdio::piped());
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());
        if let Some(file) = files.next() {
            let config=get_path_from_name(file)?;
            let mut content = String::new();
            let mut file = File::open(config)?;
            file.read_to_string(&mut content);
            for arg in content.trim().split(' ') {
                command.arg(arg);
            }
        }
        process.push(command.spawn()?);
    }
    let dur = Duration::from_millis(500);
    while let result = shutdown_rx.recv_timeout(dur) {
        match result {
            Ok(_) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
            _ => {},
        }
    };
    for mut process in process {
        process.kill()?;
    }
    Ok(())
}

#[cfg(not(windows))]
fn main() {
    panic!("This program is only intended to run on Windows.");
}