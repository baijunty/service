#[macro_use]
extern crate windows_service;

use std::ffi::OsString;
use std::time::Duration;
use windows_service::{
    service::{
        ServiceAccess, ServiceAction, ServiceActionType, ServiceErrorControl,ServiceState,
        ServiceFailureActions, ServiceFailureResetPeriod, ServiceInfo, ServiceStartType,
        ServiceType,
    },
    service_manager::{ServiceManager, ServiceManagerAccess},
};
use std::thread;

const SERVICE_NAME: &str = "PhoneProxy";

#[cfg(windows)]
fn main() -> windows_service::Result<()> {
    let mut env =std::env::args();
    let command= &*env.find(|s|{
        (&*s).eq_ignore_ascii_case("i")||(&*s).eq_ignore_ascii_case("u")
    }).expect("使用 i安装 或者 u卸载服务");
    println!("{}",command);
    let manager_access = ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE;
    let service_manager = ServiceManager::local_computer(None::<&str>, manager_access)?;
    match command {
        "i" =>{
            create_service(&service_manager)?;
        },
        "u" =>{
            del_service(&service_manager)?;
        },
        _=>unreachable!("バカな")
    }
    Ok(())
}

fn del_service(service_manager:&ServiceManager) ->windows_service::Result<()> {
    let service_access = ServiceAccess::QUERY_STATUS | ServiceAccess::STOP | ServiceAccess::DELETE;
    let service = service_manager.open_service(SERVICE_NAME, service_access)?;
    if let Ok(state) = service.query_status() {
        println!("now state {:?}",state.current_state);
        if state.current_state!= ServiceState::Stopped {
            service.stop()?;
            // Wait for service to stop
            thread::sleep(Duration::from_secs(5));
        }
    }
    service.delete()?;
    Ok(())
}

fn create_service(service_manager:&ServiceManager) ->windows_service::Result<()> {
    let service_binary_path = ::std::env::current_exe()
        .unwrap()
        .with_file_name("proxy.exe");

    let service_info = ServiceInfo {
        name: OsString::from(SERVICE_NAME),
        display_name: OsString::from("友家App热更新支持服务"),
        service_type: ServiceType::OWN_PROCESS,
        start_type: ServiceStartType::AutoStart,
        error_control: ServiceErrorControl::Normal,
        executable_path: service_binary_path,
        launch_arguments: vec![],
        dependencies: vec![],
        account_name: None, // run as System
        account_password: None,
    };
    let service_access = ServiceAccess::QUERY_CONFIG
        | ServiceAccess::STOP
        | ServiceAccess::CHANGE_CONFIG
        | ServiceAccess::START
        | ServiceAccess::DELETE;
    let service = service_manager
        .create_service(&service_info, service_access)
        .or(service_manager.open_service(SERVICE_NAME, service_access))?;
    let actions = vec![
        ServiceAction {
            action_type: ServiceActionType::Restart,
            delay: Duration::from_secs(5),
        },
        ServiceAction {
            action_type: ServiceActionType::None,
            delay: Duration::default(),
        },
    ];

    println!("Update failure actions");
    let failure_actions = ServiceFailureActions {
        reset_period: ServiceFailureResetPeriod::After(Duration::from_secs(86400 * 2)),
        reboot_msg: None,
        command: None,
        actions: Some(actions),
    };
    service.update_failure_actions(failure_actions)?;

    println!("Query failure actions");
    let updated_failure_actions = service.get_failure_actions()?;

    println!("Enable failure actions on non-crash failures");
    service.set_failure_actions_on_non_crash_failures(true)?;

    println!("Query failure actions on non-crash failures enabled");
    let failure_actions_flag = service.get_failure_actions_on_non_crash_failures()?;
    println!(
        "Failure actions on non-crash failures enabled: {}",
        failure_actions_flag
    );
    Ok(())
}

#[cfg(not(windows))]
fn main() {
    panic!("This program is only intended to run on Windows.");
}
