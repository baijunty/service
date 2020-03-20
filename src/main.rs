extern crate clap;
#[macro_use]
extern crate windows_service;

use std::ffi::OsString;
use std::thread;
use std::time::Duration;

use clap::{App, Arg, SubCommand};
use windows_service::{
    service::{
        ServiceAccess, ServiceAction, ServiceActionType, ServiceErrorControl, ServiceFailureActions,
        ServiceFailureResetPeriod, ServiceInfo, ServiceStartType, ServiceState,
        ServiceType,
    },
    service_manager::{ServiceManager, ServiceManagerAccess},
};

#[cfg(windows)]
fn main() -> windows_service::Result<()> {
    let app = App::new("service manager")
        .version("0.0.1")
        .about("Windows 服务管理")
        .author("baijunty@163.com")
        .name("service")
        .arg(Arg::with_name("config")
            .short("c")
            .long("config")
            .required(true)
            .default_value("install")
            .help("install or uninstall a service")
            .takes_value(true))
        .arg(Arg::with_name("name")
            .short("n")
            .long("name")
            .required(true)
            .help("service name")
            .takes_value(true))
        .arg(Arg::with_name("desc")
            .short("d")
            .long("description")
            .default_value("暂无描述")
            .required(false)
            .help("service description")
            .takes_value(true))
        .subcommand(SubCommand::with_name("config")
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
        )
        .get_matches();
    let manager_access = ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE;
    let service_manager = ServiceManager::local_computer(None::<&str>, manager_access)?;
    match (app.value_of("config"), app.value_of("name")) {
        (Some("install"), Some(name)) => {
            let service_binary_path = ::std::env::current_exe()
                .unwrap()
                .with_file_name("proxy.exe");
            let desc = app.value_of("desc").expect("输入错误");
            let sub =  app.subcommand_matches("config").expect(app.usage()) ;
            let mut args = vec![OsString::from("-e")];
            args.extend(sub.values_of("exe").expect(app.usage()).map(|f|OsString::from(f)));
            if let Some(fs) = sub.values_of("file") {
                args.push(OsString::from("-f"));
                args.extend(fs.map(|f|OsString::from(f)));
            }
            println!("debug {:?}",args);
            let service_info = ServiceInfo {
                name: OsString::from(name),
                display_name: OsString::from(desc),
                service_type: ServiceType::OWN_PROCESS,
                start_type: ServiceStartType::AutoStart,
                error_control: ServiceErrorControl::Normal,
                executable_path: service_binary_path,
                launch_arguments: args,
                dependencies: vec![],
                account_name: None, // run as System
                account_password: None,
            };
            create_service(&service_manager, &service_info)?;
        },
        (Some("uninstall"), Some(name)) => {
            del_service(&service_manager, name)?;
        },
        _ => panic!(app.usage().to_string())
    }
    Ok(())
}

fn del_service(service_manager: &ServiceManager, name: &str) -> windows_service::Result<()> {
    let service_access = ServiceAccess::QUERY_STATUS | ServiceAccess::STOP | ServiceAccess::DELETE;
    let service = service_manager.open_service(name, service_access)?;
    if let Ok(state) = service.query_status() {
        println!("now state {:?}", state.current_state);
        if state.current_state != ServiceState::Stopped {
            service.stop()?;
            // Wait for service to stop
            thread::sleep(Duration::from_secs(5));
        }
    }
    service.delete()?;
    Ok(())
}

fn create_service(service_manager: &ServiceManager, service_info: &ServiceInfo) -> windows_service::Result<()> {
    let service_access = ServiceAccess::QUERY_CONFIG
        | ServiceAccess::STOP
        | ServiceAccess::CHANGE_CONFIG
        | ServiceAccess::START
        | ServiceAccess::DELETE;
    let service = service_manager
        .create_service(service_info, service_access)
        .or(service_manager.open_service(&service_info.name, service_access))?;
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
    let failure_actions = ServiceFailureActions {
        reset_period: ServiceFailureResetPeriod::After(Duration::from_secs(86400 * 2)),
        reboot_msg: None,
        command: None,
        actions: Some(actions),
    };
    service.update_failure_actions(failure_actions)?;
    let updated_failure_actions = service.get_failure_actions()?;
    service.set_failure_actions_on_non_crash_failures(true)?;
    let failure_actions_flag = service.get_failure_actions_on_non_crash_failures()?;
    Ok(())
}

#[cfg(not(windows))]
fn main() {
    panic!("This program is only intended to run on Windows.");
}
