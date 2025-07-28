use std::{
    sync::mpsc::{self, Receiver, Sender},
    thread::{self, JoinHandle},
};
use uuid::Uuid;
use windows::{
    core::{IUnknown, Interface, PCWSTR},
    Win32::System::Com::{
        CoInitializeEx, CoInitializeSecurity, CoUninitialize, IClientSecurity,
        COINIT_MULTITHREADED, EOAC_DYNAMIC_CLOAKING, EOAC_STATIC_CLOAKING,
        EOLE_AUTHENTICATION_CAPABILITIES, RPC_C_AUTHN_LEVEL, RPC_C_AUTHN_LEVEL_CONNECT,
        RPC_C_IMP_LEVEL, RPC_C_IMP_LEVEL_IDENTIFY, RPC_C_IMP_LEVEL_IMPERSONATE,
    },
};
use wsl_com_api_sys::{get_lxss_user_session, ILxssUserSession, LXSS_ENUMERATE_INFO};

mod error;
pub use error::*;

/// A higher-level API for interacting with WSL through COM
pub struct Wsl {
    /// Channel sender for communicating with the background COM thread
    sender: Sender<Box<dyn FnOnce(&ILxssUserSession) + Send>>,
    /// Handle to the background thread
    _background_thread: JoinHandle<()>,
}

impl Wsl {
    /// Creates a new WSL API instance with a background COM thread
    pub fn new() -> Result<Self, WslError> {
        let (sender, receiver) = mpsc::channel();

        let background_thread = thread::spawn(move || {
            Self::com_thread_worker(receiver);
        });

        Ok(Wsl {
            sender,
            _background_thread: background_thread,
        })
    }

    /// Background thread worker that initializes COM and handles requests
    fn com_thread_worker(receiver: Receiver<Box<dyn FnOnce(&ILxssUserSession) + Send>>) {
        // Initialize COM with apartment threading
        unsafe {
            let result = CoInitializeEx(None, COINIT_MULTITHREADED);
            if result.is_err() {
                eprintln!("Failed to initialize COM: {:?}", result);
                eprintln!(
                    "This may be due to insufficient permissions or COM already being initialized"
                );
                return;
            }

            let result = CoInitializeSecurity(
                None,
                -1,
                None,
                None,
                RPC_C_AUTHN_LEVEL_CONNECT,
                RPC_C_IMP_LEVEL_IDENTIFY,
                None,
                EOAC_STATIC_CLOAKING,
                None,
            );
            if result.is_err() {
                eprintln!("Failed to initialize COM security: {:?}", result);
                return;
            }
        }

        // Get the WSL user session
        let session = match get_lxss_user_session() {
            Ok(session) => session,
            Err(e) => {
                eprintln!("Failed to get WSL user session: {:?}", e);
                eprintln!("This may be due to:");
                eprintln!("  - WSL not being installed or enabled");
                eprintln!("  - Insufficient permissions (COM operations require admin privileges)");
                eprintln!("  - Running in a CI environment without WSL support");
                unsafe {
                    CoUninitialize();
                }
                return;
            }
        };

        let result = Self::set_session_blanket(&session);
        if result.is_err() {
            eprintln!("Failed to set session blanket: {:?}", result);
            return;
        }

        // Process requests from the main thread
        for request in receiver {
            request(&session);
        }

        // Cleanup COM
        unsafe {
            CoUninitialize();
        }
    }

    fn set_session_blanket(session: &ILxssUserSession) -> Result<(), WslError> {
        let client_security: IClientSecurity = session.cast()?;

        let mut authn_svc = 0;
        let mut authz_svc = 0;
        let mut authn_lvl: RPC_C_AUTHN_LEVEL = RPC_C_AUTHN_LEVEL(0);
        let mut imp_lvl: RPC_C_IMP_LEVEL = RPC_C_IMP_LEVEL(0);
        let mut capabilities: EOLE_AUTHENTICATION_CAPABILITIES =
            EOLE_AUTHENTICATION_CAPABILITIES(0);
        unsafe {
            client_security.QueryBlanket::<&IUnknown>(
                &session.0,
                std::ptr::from_mut(&mut authn_svc),
                Some(std::ptr::from_mut(&mut authz_svc)),
                std::ptr::null_mut(),
                Some(std::ptr::from_mut(&mut authn_lvl)),
                Some(std::ptr::from_mut(&mut imp_lvl)),
                std::ptr::null_mut(),
                Some(std::ptr::from_mut(&mut capabilities.0) as _),
            )?;
        }

        capabilities.0 &= !EOAC_STATIC_CLOAKING.0;
        capabilities.0 |= EOAC_DYNAMIC_CLOAKING.0;

        unsafe {
            client_security.SetBlanket::<&IUnknown, PCWSTR>(
                &session.0,
                authn_svc,
                authz_svc,
                PCWSTR::null(),
                authn_lvl,
                RPC_C_IMP_LEVEL_IMPERSONATE,
                None,
                capabilities,
            )?;
        }

        Ok(())
    }

    /// Executes a function on the COM thread
    fn execute<F, T>(&self, f: F) -> Result<T, WslError>
    where
        F: FnOnce(&ILxssUserSession) -> Result<T, WslError> + Send + 'static,
        T: Send + 'static,
    {
        let (tx, rx) = mpsc::channel();
        self.sender
            .send(Box::new(move |session| {
                let result = f(session);
                _ = tx.send(result);
            }))
            .expect("thread died (tx)?");
        rx.recv().expect("thread died (rx)?")
    }

    /// Example API method: Shutdown WSL
    pub fn shutdown(&self, force: bool) -> Result<(), WslError> {
        self.execute(move |session| {
            session.Shutdown(force as i32)?;
            Ok(())
        })
    }

    /// Example API method: Get default distribution
    pub fn get_default_distribution(&self) -> Result<Uuid, WslError> {
        self.execute(|session| {
            Ok(session
                .GetDefaultDistribution()
                .map(|guid| Uuid::from_u128(guid.to_u128()))?)
        })
    }

    pub fn enumerate_distributions(&self) -> Result<Vec<Distribution>, WslError> {
        self.execute(|session| {
            let (count, distros) = unsafe { session.EnumerateDistributions()? };
            let distros = unsafe { std::slice::from_raw_parts(distros, count as usize) };
            let distros = distros
                .iter()
                .map(|distro| Distribution::from(distro))
                .collect();
            Ok(distros)
        })
    }
}

impl Drop for Wsl {
    fn drop(&mut self) {
        // The background thread will be joined when the JoinHandle is dropped
        // COM will be cleaned up in the background thread
    }
}

#[derive(Debug)]
pub struct Distribution {
    pub name: String,
    pub uuid: Uuid,
    pub version: Version,
}

impl From<&LXSS_ENUMERATE_INFO> for Distribution {
    fn from(info: &LXSS_ENUMERATE_INFO) -> Self {
        let name = unsafe {
            PCWSTR::from_raw(info.DistroName.as_ptr())
                .to_string()
                .unwrap()
        };
        Self {
            name: name.to_string(),
            uuid: Uuid::from_u128(info.DistroGuid.to_u128()),
            version: match info.Version {
                1 => Version::WSL1,
                2 => Version::WSL2,
                _ => Version::Unknown(info.Version),
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Version {
    WSL1,
    WSL2,
    Unknown(u32),
}
