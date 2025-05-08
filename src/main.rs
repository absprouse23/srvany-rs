use std::os::windows::process::CommandExt;
use std::process::Child;
use std::thread;
use std::{
    ffi::OsString,
    process::Command,
    sync::mpsc,
    time::Duration,
};
use windows::Win32::System::Threading::CREATE_NO_WINDOW;
use windows_registry::LOCAL_MACHINE;
use windows_service::{
    define_windows_service,
    service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
        ServiceType,
    },
    service_control_handler::{self, ServiceControlHandlerResult, ServiceStatusHandle},
    service_dispatcher,
};

const SERVICE_NAME: &str = "srvany-rs";
const SERVICE_TYPE: ServiceType = ServiceType::OWN_PROCESS;

struct RegValues {
    app_path: OsString,
    app_dir: Option<OsString>,
    app_params: Option<OsString>,
    app_env: Option<Vec<(OsString, OsString)>>,
    restart: bool,
}

fn set_service_status(
    status_handle: &ServiceStatusHandle,
    current_state: ServiceState,
    exit_code: ServiceExitCode,
) {
    let _ = status_handle.set_service_status(ServiceStatus {
        service_type: SERVICE_TYPE,
        current_state,
        controls_accepted: ServiceControlAccept::STOP,
        exit_code,
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    });
}

fn create_process(app_data: &RegValues) -> Result<Child, Box<dyn std::error::Error>> {
    let mut proc_handle = Command::new(app_data.app_path.clone());

    if let Some(dir) = &app_data.app_dir {
        proc_handle.current_dir(dir);
    }
    if let Some(params) = &app_data.app_params {
        proc_handle.raw_arg(params);
    }
    if let Some(envs) = &app_data.app_env {
        proc_handle.env_clear().envs(envs.iter().cloned());
    }
    proc_handle.creation_flags(CREATE_NO_WINDOW.0);

    proc_handle.spawn().map_err(|e| e.into())
}

fn service_main(arguments: Vec<OsString>) {
    let (shutdown_tx, shutdown_rx) = mpsc::channel();

    let event_handler = move |control_event| -> ServiceControlHandlerResult {
        match control_event {
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,

            ServiceControl::Stop => {
                shutdown_tx.send(()).unwrap();
                ServiceControlHandlerResult::NoError
            }

            _ => ServiceControlHandlerResult::NotImplemented,
        }
    };

    let status_handle = service_control_handler::register(SERVICE_NAME, event_handler).unwrap();

    let service_key = match LOCAL_MACHINE.open(format!(
        "SYSTEM\\CurrentControlSet\\Services\\{}\\Parameters\\",
        arguments[0].to_string_lossy() // first arg is the service name
    )) {
        Ok(key) => key,
        Err(_err) => {
            set_service_status(
                &status_handle,
                ServiceState::Stopped,
                ServiceExitCode::Win32(0),
            );
            return;
        }
    };

    let app_data = RegValues {
        app_path: match service_key.get_string("Application") {
            Ok(string) => OsString::from(string),
            Err(_) => {
                set_service_status(
                    &status_handle,
                    ServiceState::Stopped,
                    ServiceExitCode::Win32(0),
                );
                return;
            }
        },
        app_dir: match service_key.get_string("AppDirectory") {
            Ok(string) => Some(OsString::from(string)),
            Err(_) => None,
        },
        app_params: match service_key.get_string("AppParameters") {
            Ok(string) => Some(OsString::from(string)),
            Err(_) => None,
        },

        // srvany-ng uses REG_MULTI_SZ with the format <KEY>=<VALUE>, this converts to a vector of tuples with (<KEY>,<VALUE>)
        app_env: match service_key.get_multi_string("AppEnvironment") {
            Ok(strings) => Some(
                strings
                    .into_iter()
                    .filter_map(|s| {
                        let mut parts = s.splitn(2, '=');
                        if let (Some(key), Some(value)) = (parts.next(), parts.next()) {
                            Some((OsString::from(key), OsString::from(value)))
                        } else {
                            None
                        }
                    })
                    .collect(),
            ),
            Err(_) => None,
        },
        restart: matches!(service_key.get_u32("RestartOnExit"), Ok(1)),
    };

    let mut child = match create_process(&app_data) {
        Ok(child) => {
            set_service_status(
                &status_handle,
                ServiceState::Running,
                ServiceExitCode::Win32(0),
            );
            child
        }
        Err(_) => {
            set_service_status(
                &status_handle,
                ServiceState::Stopped,
                ServiceExitCode::Win32(0),
            );
            return;
        }
    };

    loop {
        match shutdown_rx.recv_timeout(Duration::from_secs(1)) {
            Ok(_) => {
                let _ = child.kill();
                break;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if let Some(_status) = child.try_wait().unwrap() {
                    if app_data.restart {
                        thread::sleep(Duration::from_secs(1));
                        child = match create_process(&app_data) {
                            Ok(new_child) => new_child,
                            Err(_) => break,
                        };
                    } else {
                        break;
                    }
                }
            }
            Err(_) => break,
        }
    }

    set_service_status(
        &status_handle,
        ServiceState::Stopped,
        ServiceExitCode::Win32(0),
    );
}

define_windows_service!(ffi_service_main, service_main);

fn main() -> Result<(), windows_service::Error> {
    // Register generated `ffi_service_main` with the system and start the service, blocking
    // this thread until the service is stopped.
    service_dispatcher::start("myservice", ffi_service_main)?;
    Ok(())
}
