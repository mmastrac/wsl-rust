use bitflags::bitflags;
use std::{
    sync::mpsc::{self, Receiver, Sender},
    thread::{self, JoinHandle},
};
use uuid::Uuid;
use windows::{
    core::{IUnknown, Interface, GUID, PCWSTR},
    Win32::{
        Foundation::{GetLastError, HANDLE},
        Storage::FileSystem::{
            GetFileType, FILE_TYPE_CHAR, FILE_TYPE_DISK, FILE_TYPE_PIPE, FILE_TYPE_REMOTE,
            FILE_TYPE_UNKNOWN,
        },
        System::Com::{
            CoInitializeEx, CoInitializeSecurity, CoTaskMemFree, CoUninitialize, IClientSecurity,
            COINIT_MULTITHREADED, EOAC_DYNAMIC_CLOAKING, EOAC_STATIC_CLOAKING,
            EOLE_AUTHENTICATION_CAPABILITIES, RPC_C_AUTHN_LEVEL, RPC_C_AUTHN_LEVEL_CONNECT,
            RPC_C_IMP_LEVEL, RPC_C_IMP_LEVEL_IDENTIFY, RPC_C_IMP_LEVEL_IMPERSONATE,
        },
    },
};
use wsl_com_api_sys::{get_lxss_user_session, ILxssUserSession, LXSS_ENUMERATE_INFO};

mod error;
pub use error::*;

// Allows this code to compile on both Windows and Unix

#[cfg(windows)]
use std::os::windows::io::AsRawHandle;
#[cfg(windows)]
fn to_handle(handle: &impl AsRawHandle) -> HANDLE {
    HANDLE(handle.as_raw_handle() as isize)
}

#[cfg(unix)]
use std::os::fd::AsRawFd as AsRawHandle;
#[cfg(unix)]
fn to_handle(_: &impl AsRawHandle) -> HANDLE {
    unreachable!("This should never be called on Unix: we only support Windows");
}

/// Validates that a file handle is of the expected type
fn validate_file_handle(
    name: &str,
    handle: HANDLE,
    expected_type: windows::Win32::Storage::FileSystem::FILE_TYPE,
) -> Result<(), WslError> {
    let file_type = unsafe { GetFileType(handle) };
    if file_type == FILE_TYPE_UNKNOWN {
        return Err(windows::core::Error::new(
            wsl_com_api_sys::error::WSL_E_INVALID_USAGE,
            format!(
                "{} ({:x}) is not a valid file handle: {:?}",
                name,
                handle.0,
                unsafe { GetLastError() }
            ),
        )
        .into());
    }
    let type_to_string = |file_type: windows::Win32::Storage::FileSystem::FILE_TYPE| match file_type
    {
        FILE_TYPE_DISK => "file",
        FILE_TYPE_PIPE => "pipe",
        FILE_TYPE_CHAR => "character device",
        FILE_TYPE_REMOTE => "remote file",
        FILE_TYPE_UNKNOWN => "unknown type",
        _ => "invalid type",
    };

    if file_type != expected_type {
        let expected_type_name = type_to_string(expected_type);
        return Err(windows::core::Error::new(
            wsl_com_api_sys::error::WSL_E_INVALID_USAGE,
            format!(
                "{} ({:x}) must be a {} (got a {})",
                name,
                handle.0,
                expected_type_name,
                type_to_string(file_type)
            ),
        )
        .into());
    }
    Ok(())
}

struct CoMultithreadedInterface<T: Interface>(T);

unsafe impl<T: Interface> Send for CoMultithreadedInterface<T> {}

/// A higher-level API for interacting with WSL through COM
pub struct Wsl {
    /// Channel sender for communicating with the background COM thread
    sender: Sender<Box<dyn FnOnce(&ILxssUserSession) + Send>>,
    /// The WSL session object (thread-safe)
    session: CoMultithreadedInterface<ILxssUserSession>,
    /// Handle to the background thread
    _background_thread: JoinHandle<()>,
}

impl Wsl {
    /// Creates a new WSL API instance with a background COM thread
    pub fn new() -> Result<Self, WslError> {
        let (sender, receiver) = mpsc::channel();
        let (tx_init, rx_init) = mpsc::channel();

        let background_thread = thread::spawn(move || {
            Self::com_thread_worker(receiver, tx_init);
        });

        let session = rx_init
            .recv()
            .expect("thread died (init)?")
            .map_err(WslError::from)?;

        Ok(Wsl {
            sender,
            session,
            _background_thread: background_thread,
        })
    }

    /// Background thread worker that initializes COM and handles requests
    fn com_thread_worker(
        receiver: Receiver<Box<dyn FnOnce(&ILxssUserSession) + Send>>,
        initialized: Sender<windows::core::Result<CoMultithreadedInterface<ILxssUserSession>>>,
    ) {
        // Initialize COM with apartment threading
        unsafe {
            let result = CoInitializeEx(None, COINIT_MULTITHREADED);
            if result.is_err() {
                initialized
                    .send(Err(result.into()))
                    .expect("thread died (init tx)?");
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
            if let Err(e) = result {
                CoUninitialize();
                initialized.send(Err(e)).expect("thread died (init tx)?");
                return;
            }
        }

        // Get the WSL user session
        let session = match get_lxss_user_session() {
            Ok(session) => session,
            Err(e) => {
                unsafe {
                    CoUninitialize();
                }
                initialized
                    .send(Err(e.into()))
                    .expect("thread died (init tx)?");
                return;
            }
        };

        let result = Self::set_session_blanket(&session);
        if let Err(e) = result {
            unsafe {
                CoUninitialize();
            }
            initialized.send(Err(e)).expect("thread died (init tx)?");
            return;
        }

        initialized
            .send(Ok(CoMultithreadedInterface(session.clone())))
            .expect("thread died (init tx)?");

        // Process requests from the main thread
        for request in receiver {
            request(&session);
        }

        // Cleanup COM
        unsafe {
            CoUninitialize();
        }
    }

    fn set_session_blanket(session: &ILxssUserSession) -> windows::core::Result<()> {
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
    fn execute_thread<F, T>(&self, f: F) -> Result<T, WslError>
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

    /// Executes a function on the current thread
    fn execute<F, T>(&self, f: F) -> Result<T, WslError>
    where
        F: FnOnce(&ILxssUserSession) -> Result<T, WslError> + Send + 'static,
        T: Send + 'static,
    {
        f(&self.session.0)
    }

    /// Shuts down WSL and closes this handle.
    pub fn shutdown(self, force: bool) -> Result<(), WslError> {
        self.execute_thread(move |session| {
            session.Shutdown(force as i32)?;
            Ok(())
        })
    }

    /// Gets the default distribution.
    pub fn get_default_distribution(&self) -> Result<Uuid, WslError> {
        self.execute(|session| {
            Ok(session
                .GetDefaultDistribution()
                .map(|guid| Uuid::from_u128(guid.to_u128()))?)
        })
    }

    /// Enumerates the distributions.
    pub fn enumerate_distributions(&self) -> Result<Vec<Distribution>, WslError> {
        self.execute(|session| {
            let (count, distros) = unsafe { session.EnumerateDistributions()? };
            let distros_copy = {
                let slice = unsafe { std::slice::from_raw_parts(distros, count as usize) };
                slice
                    .iter()
                    .map(|distro| Distribution::from(distro))
                    .collect()
            };
            unsafe {
                CoTaskMemFree(Some(distros as _));
            }
            Ok(distros_copy)
        })
    }

    /// Exports a distribution.
    pub fn export_distribution(
        &self,
        distro_guid: Uuid,
        file: impl AsRawHandle,
        stderr: impl AsRawHandle,
        flags: ExportFlags,
    ) -> Result<(), WslError> {
        let file_handle = to_handle(&file);
        let stderr_handle = to_handle(&stderr);

        let res = self.execute(move |session| {
            // Validate handles in the COM thread to ensure they're still valid
            validate_file_handle("stderr_handle", stderr_handle, FILE_TYPE_PIPE)?;
            validate_file_handle("file_handle", file_handle, FILE_TYPE_DISK)?;

            session.ExportDistribution(
                GUID::from_u128(distro_guid.as_u128()),
                file_handle,
                stderr_handle,
                flags.bits(),
            )?;
            Ok(())
        });

        drop(file);
        drop(stderr);
        res
    }

    /// Registers a WSL distribution in the default location. Note that the
    /// distribution name must be unique.
    pub fn register_distribution(
        &self,
        name: &str,
        version: Version,
        file: impl AsRawHandle,
        stderr: impl AsRawHandle,
        flags: ImportFlags,
    ) -> Result<(Uuid, String), WslError> {
        let file_handle = to_handle(&file);
        let stderr_handle = to_handle(&stderr);
        let wide_name = widestring::U16CString::from_str_truncate(name);

        let res = self.execute(move |session| {
            // Validate handles in the COM thread to ensure they're still valid
            validate_file_handle("stderr_handle", stderr_handle, FILE_TYPE_PIPE)?;
            validate_file_handle("file_handle", file_handle, FILE_TYPE_DISK)?;

            let (guid, installed_name) = session.RegisterDistribution(
                PCWSTR::from_raw(wide_name.as_ptr()),
                version.into(),
                file_handle,
                stderr_handle,
                PCWSTR::null(),
                flags.bits(),
                0,
                PCWSTR::null(),
            )?;
            let name = unsafe { installed_name.to_string().unwrap_or_default() };
            unsafe {
                CoTaskMemFree(Some(installed_name.0 as _));
            }
            Ok((Uuid::from_u128(guid.to_u128()), name))
        });

        drop(file);
        drop(stderr);
        res
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

impl Into<u32> for Version {
    fn into(self) -> u32 {
        match self {
            Version::WSL1 => 1,
            Version::WSL2 => 2,
            Version::Unknown(v) => v,
        }
    }
}

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct ExportFlags: u32 {
        const VHD = 0x1;
        const GZIP = 0x2;
        const XZIP = 0x4;
        const VERBOSE = 0x8;
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct ImportFlags: u32 {
        const VHD = 0x1;
        const CREATE_SHORTCUT = 0x2;
        /// Disable "out of box experience" script (OOBE)
        const NO_OOBE = 0x4;
        const FIXED_VHD = 0x8;
    }
}
