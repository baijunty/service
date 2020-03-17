#[macro_use]
extern crate windows_service;

use std::ffi::OsString;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::process::{Command, Stdio, Child};
use std::sync::mpsc;
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
use std::sync::mpsc::Receiver;
use std::rc::Rc;
use std::cell::RefCell;

const SERVICE_NAME: &str = "PhoneProxy";

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
        println!("{:?}", e);
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
    start_proxy(&shutdown_rx)?;
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

fn start_proxy(shutdown_rx:&Receiver<()>) -> Result<(), Box<dyn std::error::Error>> {
    let current = std::env::current_exe()?;
//    let config_file = current.with_file_name("config.json");
    let mut child = Command::new(current.with_file_name("client.exe"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .args(vec!["-r", "104.198.93.253:7777", "-l", ":8802", "-mode", "fast2", "-key", "kasiwa120bai", "-crypt", "aes-192", "-sockbuf", "16777217", "-dscp", "46"]).spawn().expect("start failed");
    let mut child2 = Command::new(current.with_file_name("client.exe"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .args(vec!["-r", "3.113.27.184:7777", "-l", ":8801", "-mode", "fast2", "-key", "kasiwa120bai", "-crypt", "aes-192", "-sockbuf", "16777217", "-dscp", "46"]).spawn().expect("start failed");
    let mut child3 = Command::new(current.with_file_name("hotfix.exe"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn().expect("start failed");
    let dur=Duration::from_millis(500);
    while let result =  shutdown_rx.recv_timeout(dur){
        match result {
            Ok(_) | Err(mpsc::RecvTimeoutError::Disconnected)=>break,
            _=>(),
        }
    };
    child.kill()?;
    child2.kill()?;
    child3.kill()?;
    Ok(())
}

#[test]
fn test_process() {
    let (shutdown_tx, shutdown_rx) = mpsc::channel();
    match start_proxy(&shutdown_rx) {
        Ok(_) => {},
        Err(e) => println!("{:?}", e.to_string())
    }
}

#[cfg(not(windows))]
fn main() {
    panic!("This program is only intended to run on Windows.");
}