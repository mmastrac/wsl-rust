use std::ffi::CString;
use std::process::{ChildStderr, ChildStdin, ChildStdout, ExitStatus};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};

use bitflags::bitflags;
use uuid::Uuid;
use windows::core::{IUnknown, Interface, GUID, PCSTR, PCWSTR};
use windows::Win32::Foundation::{CloseHandle, GetLastError, HANDLE};
use windows::Win32::Networking::WinSock::WSAStartup;
use windows::Win32::Storage::FileSystem::{
    GetFileType, FILE_TYPE_CHAR, FILE_TYPE_DISK, FILE_TYPE_PIPE, FILE_TYPE_REMOTE,
    FILE_TYPE_UNKNOWN,
};
use windows::Win32::System::Com::{
    CoInitializeEx, CoInitializeSecurity, CoTaskMemFree, CoUninitialize, IClientSecurity,
    COINIT_MULTITHREADED, EOAC_DYNAMIC_CLOAKING, EOAC_STATIC_CLOAKING,
    EOLE_AUTHENTICATION_CAPABILITIES, RPC_C_AUTHN_LEVEL, RPC_C_AUTHN_LEVEL_CONNECT,
    RPC_C_IMP_LEVEL, RPC_C_IMP_LEVEL_IDENTIFY, RPC_C_IMP_LEVEL_IMPERSONATE,
};
use windows::Win32::System::IO::DeviceIoControl;
use wsl_com_api_sys::{
    constants::*, get_lxss_user_session, ILxssUserSession, LxssHandleType, LXSS_ENUMERATE_INFO,
    LXSS_HANDLE, LXSS_STD_HANDLES,
};

use wsl_com_api_sys::interop::LXBUS_IPC_LX_PROCESS_WAIT_FOR_TERMINATION_PARAMETERS;

mod error;
pub use error::*;
mod interop;

// Allows this code to compile on both Windows and Unix

#[cfg(windows)]
use std::os::windows::io::AsRawHandle;
#[cfg(windows)]
fn to_handle(handle: &impl AsRawHandle) -> HANDLE {
    HANDLE(handle.as_raw_handle() as isize)
}
#[cfg(windows)]
fn from_handle<T: From<std::os::windows::io::OwnedHandle>>(handle: HANDLE) -> T {
    use std::os::windows::io::FromRawHandle;
    unsafe {
        T::from(std::os::windows::io::OwnedHandle::from_raw_handle(
            handle.0 as _,
        ))
    }
}

#[cfg(unix)]
use std::os::fd::AsRawFd as AsRawHandle;

use crate::interop::Interop;
#[cfg(unix)]
fn to_handle(_: &impl AsRawHandle) -> HANDLE {
    unreachable!("This should never be called on Unix: we only support Windows");
}
#[cfg(unix)]
fn from_handle<T>(_: HANDLE) -> T {
    unreachable!("This should never be called on Unix: we only support Windows");
}

/// WSL-specific process waiting function that uses LXBUS IOCTL
unsafe fn wait_for_wsl_process(process_handle: HANDLE, timeout_ms: u32) -> Result<u32, WslError> {
    let mut parameters = LXBUS_IPC_LX_PROCESS_WAIT_FOR_TERMINATION_PARAMETERS {
        Input: wsl_com_api_sys::interop::LXBUS_IPC_LX_PROCESS_WAIT_FOR_TERMINATION_INPUT {
            TimeoutMs: timeout_ms,
        },
    };

    DeviceIoControl(
        process_handle,
        LXBUS_IPC_LX_PROCESS_IOCTL_WAIT_FOR_TERMINATION,
        Some(&parameters.Input as *const _ as *const _),
        std::mem::size_of::<wsl_com_api_sys::interop::LXBUS_IPC_LX_PROCESS_WAIT_FOR_TERMINATION_INPUT>(
        ) as u32,
        Some(&mut parameters.Output as *mut _ as *mut _),
        std::mem::size_of::<
            wsl_com_api_sys::interop::LXBUS_IPC_LX_PROCESS_WAIT_FOR_TERMINATION_OUTPUT,
        >() as u32,
        None,
        None,
    )?;

    // The exit status is stored shifted, with status in the lower 8 bits
    Ok((parameters.Output.ExitStatus as u32) >> 8)
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

/// A higher-level API for interacting with WSL2 through COM
pub struct Wsl2 {
    /// Channel sender for communicating with the background COM thread
    sender: Sender<Box<dyn FnOnce(&ILxssUserSession) + Send>>,
    /// The WSL session object (thread-safe)
    session: CoMultithreadedInterface<ILxssUserSession>,
    /// Handle to the background thread
    _background_thread: JoinHandle<()>,
}

impl Wsl2 {
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

        Ok(Wsl2 {
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
        unsafe {
            // Initialize Winsock: this is required (unsure what requires it,
            // but we get 8007276D otherwise)
            // "Either the application has not called WSAStartup, or WSAStartup failed"
            let mut wsa_data = std::mem::zeroed();
            let result = WSAStartup(0x0202, &mut wsa_data);
            if result != 0 {
                initialized
                    .send(Err(windows::core::Error::new(
                        wsl_com_api_sys::error::WSL_E_INVALID_USAGE,
                        format!("WSAStartup failed: 0x{:x}", result),
                    )
                    .into()))
                    .expect("thread died (init tx)?");
                return;
            }

            // Initialize COM with apartment threading
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
        let session = match unsafe { get_lxss_user_session() } {
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

    /// Enables the WSL session to impersonate the calling user's identity when
    /// making requests to the WSL service. This allows the session to access
    /// user-specific resources and run processes with the correct permissions
    /// without requiring elevated privileges for the entire session.
    ///
    /// See
    /// https://learn.microsoft.com/en-us/windows/win32/api/objidl/nf-objidl-iclientsecurity-setblanket
    /// for details on COM security blankets.
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
        self.execute_thread(move |session| unsafe {
            session.Shutdown(force as i32)?;
            Ok(())
        })
    }

    /// Gets the default distribution.
    pub fn get_default_distribution(&self) -> Result<Uuid, WslError> {
        self.execute(|session| unsafe {
            Ok(session
                .GetDefaultDistribution()
                .map(|guid| Uuid::from_u128(guid.to_u128()))?)
        })
    }

    /// Launches a Linux process in the specified WSL distribution. The process
    /// runs under the specified username and returns handles to
    /// stdin/stdout/stderr for communication.
    pub fn launch(
        &self,
        distro_guid: Uuid,
        command: &str,
        args: &[&str],
        cwd: Option<&str>,
        username: &str,
    ) -> Result<WslProcess, WslError> {
        let username = widestring::U16CString::from_str_truncate(username);
        let command = CString::new(command).unwrap();
        let cwd = cwd.map(|cwd| widestring::U16CString::from_str_truncate(cwd));
        let nt_path = widestring::U16CString::from_str_truncate(
            std::env::current_dir()
                .unwrap_or_default()
                .to_str()
                .unwrap(),
        );
        let args = args
            .iter()
            .map(|arg| CString::new(*arg).unwrap())
            .collect::<Vec<_>>();

        let (stdin_r, stdin_w) = std::io::pipe().unwrap();
        let (stdout_r, stdout_w) = std::io::pipe().unwrap();
        let (stderr_r, stderr_w) = std::io::pipe().unwrap();

        let pipe = (to_handle(&stdin_r), to_handle(&stdout_w), to_handle(&stderr_w));

        let handles = LXSS_STD_HANDLES {
            StdIn: LXSS_HANDLE {
                Handle: pipe.0.0 as _,
                HandleType: LxssHandleType::LxssHandleInput,
            },
            StdOut: LXSS_HANDLE {
                Handle: pipe.1.0 as _,
                HandleType: LxssHandleType::LxssHandleOutput,
            },
            StdErr: LXSS_HANDLE {
                Handle: pipe.2.0 as _,
                HandleType: LxssHandleType::LxssHandleOutput,
            },
        };

        std::mem::forget(stderr_w);
        std::mem::forget(stdout_w);
        std::mem::forget(stdin_r);

        self.execute(move |session| unsafe {
            let arg_ptrs = args
                .iter()
                .map(|arg| arg.to_bytes_with_nul().as_ptr())
                .collect::<Vec<_>>();
            let result = session.CreateLxProcess(
                GUID::from_u128(distro_guid.as_u128()),
                PCSTR::from_raw(command.as_ptr() as *const u8),
                args.len() as u32,
                arg_ptrs.as_ptr() as *const PCSTR,
                PCWSTR::from_raw(cwd.map(|cwd| cwd.as_ptr()).unwrap_or(std::ptr::null())),
                PCWSTR::from_raw(nt_path.as_ptr()),
                std::ptr::null_mut(), // todo
                0,                    // todo
                PCWSTR::from_raw(username.as_ptr()),
                80,
                25,
                0,
                std::ptr::from_ref(&handles),
                CreateInstanceFlags::empty().bits(),
            )?;

            eprintln!("result: {result:?}");

            #[allow(unreachable_code)]
            let process = if result.ProcessHandle.is_invalid() {
                // This is harder to mock on unix, so just bail
                #[cfg(unix)]
                let tcp = { unreachable!("Unsupported platform") };

                #[cfg(windows)]
                let tcp = {
                    use std::net::TcpStream;
                    use std::os::windows::io::FromRawSocket;
                    TcpStream::from_raw_socket(result.InteropSocket.0 as _)
                };

                WslProcess {
                    stdin: Some(from_handle(result.StandardIn)),
                    stdout: Some(from_handle(result.StandardOut)),
                    stderr: Some(from_handle(result.StandardErr)),
                    pipe,
                    handle: WslProcessInner::WSL2(Interop::new(tcp), result.CommunicationChannel),
                }
            } else {
                let process = WslProcess {
                    stdin: Some(from_handle(to_handle(&stdin_w))),
                    stdout: Some(from_handle(to_handle(&stdout_r))),
                    stderr: Some(from_handle(to_handle(&stderr_r))),
                    pipe,
                    handle: WslProcessInner::WSL1(result.ProcessHandle),
                };

                // Close the server handle
                _ = CloseHandle(result.ServerHandle);

                std::mem::forget(stdin_w);
                std::mem::forget(stdout_r);
                std::mem::forget(stderr_r);

                process
            };

            Ok(process)
        })
    }

    /// Enumerates the distributions.
    pub fn enumerate_distributions(&self) -> Result<Vec<Distribution>, WslError> {
        self.execute(|session| unsafe {
            let (count, distros) = session.EnumerateDistributions()?;
            let distros_copy = {
                let slice = std::slice::from_raw_parts(distros, count as usize);
                slice
                    .iter()
                    .map(|distro| Distribution::from(distro))
                    .collect()
            };
            CoTaskMemFree(Some(distros as _));
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

        let res = self.execute(move |session| unsafe {
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

        let res = self.execute(move |session| unsafe {
            // Validate handles in the COM thread to ensure they're still valid
            validate_file_handle("stderr_handle", stderr_handle, FILE_TYPE_PIPE)?;
            validate_file_handle("file_handle", file_handle, FILE_TYPE_DISK)?;

            let result = session.RegisterDistribution(
                PCWSTR::from_raw(wide_name.as_ptr()),
                version.into(),
                file_handle,
                stderr_handle,
                PCWSTR::null(),
                flags.bits(),
                0,
                PCWSTR::null(),
            )?;
            let name = result.InstalledName.to_string().unwrap_or_default();
            CoTaskMemFree(Some(result.InstalledName.0 as _));
            Ok((Uuid::from_u128(result.Guid.to_u128()), name))
        });

        drop(file);
        drop(stderr);
        res
    }

    pub fn set_version(
        &self,
        distribution: Uuid,
        version: Version,
        stderr: impl AsRawHandle,
    ) -> Result<(), WslError> {
        let handle = to_handle(&stderr);
        let res = self.execute(move |session| unsafe {
            session.SetVersion(
                GUID::from_u128(distribution.as_u128()),
                version.into(),
                handle,
            )?;
            Ok(())
        });

        drop(stderr);
        res
    }
}

impl Drop for Wsl2 {
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
    Legacy,
    WSL1,
    WSL2,
    Unknown(u32),
}

impl Into<u32> for Version {
    fn into(self) -> u32 {
        match self {
            Version::Legacy => 0,
            Version::WSL1 => 1,
            Version::WSL2 => 2,
            Version::Unknown(v) => v,
        }
    }
}

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct ExportFlags: u32 {
        const VHD = LXSS_EXPORT_DISTRO_FLAGS_VHD;
        const GZIP = LXSS_EXPORT_DISTRO_FLAGS_GZIP;
        const XZIP = LXSS_EXPORT_DISTRO_FLAGS_XZIP;
        const VERBOSE = LXSS_EXPORT_DISTRO_FLAGS_VERBOSE;
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct ImportFlags: u32 {
        const VHD = LXSS_IMPORT_DISTRO_FLAGS_VHD;
        const CREATE_SHORTCUT = LXSS_IMPORT_DISTRO_FLAGS_CREATE_SHORTCUT;
        /// Disable "out of box experience" script (OOBE)
        const NO_OOBE = LXSS_IMPORT_DISTRO_FLAGS_NO_OOBE;
        const FIXED_VHD = LXSS_IMPORT_DISTRO_FLAGS_FIXED_VHD;
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct CreateInstanceFlags: u32 {
        const ALLOW_FS_UPGRADE = LXSS_CREATE_INSTANCE_FLAGS_ALLOW_FS_UPGRADE;
        const OPEN_EXISTING = LXSS_CREATE_INSTANCE_FLAGS_OPEN_EXISTING;
        const IGNORE_CLIENT = LXSS_CREATE_INSTANCE_FLAGS_IGNORE_CLIENT;
        const USE_SYSTEM_DISTRO = LXSS_CREATE_INSTANCE_FLAGS_USE_SYSTEM_DISTRO;
        const SHELL_LOGIN = LXSS_CREATE_INSTANCE_FLAGS_SHELL_LOGIN;
    }
}

#[derive(Debug)]
pub struct WslProcess {
    pub stdin: Option<ChildStdin>,
    pub stdout: Option<ChildStdout>,
    pub stderr: Option<ChildStderr>,
    pipe: (HANDLE, HANDLE, HANDLE),
    handle: WslProcessInner,
}

fn u32_to_exit_status(exit_code: u32) -> ExitStatus {
    // Allow this to compile on both Unix and Windows
    #[cfg(unix)]
    use std::os::unix::process::ExitStatusExt;
    #[cfg(windows)]
    use std::os::windows::process::ExitStatusExt;

    ExitStatusExt::from_raw(exit_code as _)
}

impl WslProcess {
    pub fn wait(self) -> Result<ExitStatus, WslError> {
        match &self.handle {
            WslProcessInner::WSL1(handle) => {
                // Use WSL-specific waiting mechanism instead of WaitForSingleObject
                let exit_code = unsafe { wait_for_wsl_process(*handle, u32::MAX)? };
                Ok(u32_to_exit_status(exit_code))
            }
            WslProcessInner::WSL2(interop, _) => {
                let exit = interop.recv_exit_code();
                Ok(exit.map(u32_to_exit_status).unwrap_or_default())
            }
        }
    }
}

impl Drop for WslProcess {
    fn drop(&mut self) {
        match self.handle {
            WslProcessInner::WSL2(_, handle) => unsafe { _ = CloseHandle(handle) },
            WslProcessInner::WSL1(handle) => unsafe {
                _ = CloseHandle(handle);
            },
        }

        unsafe {
            _ = CloseHandle(self.pipe.0);
            _ = CloseHandle(self.pipe.1);
            _ = CloseHandle(self.pipe.2);
        }
    }
}

#[derive(Debug)]
enum WslProcessInner {
    WSL1(HANDLE),
    WSL2(Interop, HANDLE),
}
