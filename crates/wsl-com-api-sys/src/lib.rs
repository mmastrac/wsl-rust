#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use windows::{
    core::{IUnknown, Interface, Result, GUID, HRESULT, PCSTR, PCWSTR, PWSTR},
    Win32::{
        Foundation::HANDLE,
        System::Com::{CoCreateInstance, CLSCTX_LOCAL_SERVER},
    },
};

use std::mem::MaybeUninit;

pub mod constants;
pub mod error;

const CLSID_LXSSUSERSESSION: GUID = GUID::from_u128(0xa9b7a1b9_0671_405c_95f1_e0612cb4ce7e);
const IID_ILXSSUSERSESSION: GUID = GUID::from_u128(0x38541bdc_f54f_4ceb_85d0_37f0f3d2617e);

pub type LxssError = (HRESULT, LXSS_ERROR_INFO);
pub type LxssResult<T> = std::result::Result<T, LxssError>;

#[repr(C)]
pub struct LXSS_ERROR_INFO {
    pub Flags: u32,
    pub Context: u64,
    pub Message: PWSTR,
    pub Warnings: PWSTR,
    pub WarningsPipe: u32,
}

unsafe impl Send for LXSS_ERROR_INFO {}

impl std::fmt::Debug for LXSS_ERROR_INFO {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = f.debug_struct("LXSS_ERROR_INFO");
        s.field("Flags", &self.Flags)
            .field("Context", &self.Context)
            .field("WarningsPipe", &self.WarningsPipe);

        if !self.Message.is_null() {
            s.field("Message", &unsafe { self.Message.to_string() });
        }

        if !self.Warnings.is_null() {
            s.field("Warnings", &unsafe { self.Warnings.to_string() });
        }

        s.finish()
    }
}

#[repr(C)]
pub struct LXSS_ENUMERATE_INFO {
    pub DistroGuid: GUID,
    pub State: u32,
    pub Version: u32,
    pub Flags: u32,
    pub DistroName: [u16; 257],
}

#[repr(C)]
pub struct LXSS_HANDLE {
    pub Handle: u32,
    pub HandleType: LxssHandleType,
}

#[repr(u32)]
pub enum LxssHandleType {
    LxssHandleConsole = 0,
    LxssHandleInput,
    LxssHandleOutput,
}

#[repr(C)]
pub struct LXSS_STD_HANDLES {
    pub StdIn: LXSS_HANDLE,
    pub StdOut: LXSS_HANDLE,
    pub StdErr: LXSS_HANDLE,
}

#[repr(transparent)]
#[derive(Clone)]
pub struct ILxssUserSession(pub IUnknown);

unsafe impl Interface for ILxssUserSession {
    type Vtable = ILxssUserSession_Vtbl;
    const IID: GUID = IID_ILXSSUSERSESSION;
}

#[repr(C)]
pub struct ILxssUserSession_Vtbl {
    pub base__: windows::core::IUnknown_Vtbl,

    pub CreateInstance: unsafe extern "system" fn(
        this: *mut ::core::ffi::c_void,
        distro_guid: *const GUID,
        flags: u32,
        error: *mut LXSS_ERROR_INFO,
    ) -> HRESULT,

    pub RegisterDistribution: unsafe extern "system" fn(
        this: *mut ::core::ffi::c_void,
        name: PCWSTR,
        version: u32,
        file_handle: HANDLE,
        stderr_handle: HANDLE,
        target_directory: PCWSTR,
        flags: u32,
        vhd_size: u64,
        package_family_name: PCWSTR,
        installed_name: *mut PWSTR,
        error: *mut LXSS_ERROR_INFO,
        out_guid: *mut GUID,
    ) -> HRESULT,

    pub RegisterDistributionPipe: unsafe extern "system" fn(
        this: *mut ::core::ffi::c_void,
        name: PCWSTR,
        version: u32,
        pipe_handle: HANDLE,
        stderr_handle: HANDLE,
        target_directory: PCWSTR,
        flags: u32,
        vhd_size: u64,
        package_family_name: PCWSTR,
        installed_name: *mut PWSTR,
        error: *mut LXSS_ERROR_INFO,
        out_guid: *mut GUID,
    ) -> HRESULT,

    pub GetDistributionId: unsafe extern "system" fn(
        this: *mut ::core::ffi::c_void,
        name: PCWSTR,
        flags: u32,
        error: *mut LXSS_ERROR_INFO,
        out_guid: *mut GUID,
    ) -> HRESULT,

    pub TerminateDistribution: unsafe extern "system" fn(
        this: *mut ::core::ffi::c_void,
        distro_guid: *const GUID,
        error: *mut LXSS_ERROR_INFO,
    ) -> HRESULT,

    pub UnregisterDistribution: unsafe extern "system" fn(
        this: *mut ::core::ffi::c_void,
        distro_guid: *const GUID,
        error: *mut LXSS_ERROR_INFO,
    ) -> HRESULT,

    pub ConfigureDistribution: unsafe extern "system" fn(
        this: *mut ::core::ffi::c_void,
        distro_guid: *const GUID,
        default_uid: u32,
        flags: u32,
        error: *mut LXSS_ERROR_INFO,
    ) -> HRESULT,

    pub GetDistributionConfiguration: unsafe extern "system" fn(
        this: *mut ::core::ffi::c_void,
        distro_guid: *const GUID,
        distribution_name: *mut PWSTR,
        version: *mut u32,
        default_uid: *mut u32,
        env_count: *mut u32,
        default_environment: *mut *mut PCSTR,
        flags: *mut u32,
        error: *mut LXSS_ERROR_INFO,
    ) -> HRESULT,

    pub GetDefaultDistribution: unsafe extern "system" fn(
        this: *mut ::core::ffi::c_void,
        error: *mut LXSS_ERROR_INFO,
        out_guid: *mut GUID,
    ) -> HRESULT,

    pub ResizeDistribution: unsafe extern "system" fn(
        this: *mut ::core::ffi::c_void,
        distro_guid: *const GUID,
        output_handle: HANDLE,
        new_size: u64,
        error: *mut LXSS_ERROR_INFO,
    ) -> HRESULT,

    pub SetDefaultDistribution: unsafe extern "system" fn(
        this: *mut ::core::ffi::c_void,
        distro_guid: *const GUID,
        error: *mut LXSS_ERROR_INFO,
    ) -> HRESULT,

    pub SetSparse: unsafe extern "system" fn(
        this: *mut ::core::ffi::c_void,
        distro_guid: *const GUID,
        sparse: i32,
        allow_unsafe: i32,
        error: *mut LXSS_ERROR_INFO,
    ) -> HRESULT,

    pub EnumerateDistributions: unsafe extern "system" fn(
        this: *mut ::core::ffi::c_void,
        count: *mut u32,
        distros: *mut *const LXSS_ENUMERATE_INFO,
        error: *mut LXSS_ERROR_INFO,
    ) -> HRESULT,

    pub CreateLxProcess: unsafe extern "system" fn(
        this: *mut ::core::ffi::c_void,
        distro_guid: *const GUID,
        filename: PCSTR,
        command_line_count: u32,
        command_line: *const PCSTR,
        cwd: PCWSTR,
        nt_path: PCWSTR,
        nt_env: *mut u16,
        nt_env_len: u32,
        username: PCWSTR,
        columns: i16,
        rows: i16,
        console_handle: u32,
        std_handles: *const LXSS_STD_HANDLES,
        flags: u32,
        out_distribution_id: *mut GUID,
        out_instance_id: *mut GUID,
        process_handle: *mut HANDLE,
        server_handle: *mut HANDLE,
        stdin: *mut HANDLE,
        stdout: *mut HANDLE,
        stderr: *mut HANDLE,
        comm_channel: *mut HANDLE,
        interop_socket: *mut HANDLE,
        error: *mut LXSS_ERROR_INFO,
    ) -> HRESULT,

    pub SetVersion: unsafe extern "system" fn(
        this: *mut ::core::ffi::c_void,
        distro_guid: *const GUID,
        version: u32,
        stderr_handle: HANDLE,
        error: *mut LXSS_ERROR_INFO,
    ) -> HRESULT,

    pub ExportDistribution: unsafe extern "system" fn(
        this: *mut ::core::ffi::c_void,
        distro_guid: *const GUID,
        file_handle: HANDLE,
        stderr_handle: HANDLE,
        flags: u32,
        error: *mut LXSS_ERROR_INFO,
    ) -> HRESULT,

    pub ExportDistributionPipe: unsafe extern "system" fn(
        this: *mut ::core::ffi::c_void,
        distro_guid: *const GUID,
        pipe_handle: HANDLE,
        stderr_handle: HANDLE,
        flags: u32,
        error: *mut LXSS_ERROR_INFO,
    ) -> HRESULT,

    pub AttachDisk: unsafe extern "system" fn(
        this: *mut ::core::ffi::c_void,
        disk: PCWSTR,
        flags: u32,
        error: *mut LXSS_ERROR_INFO,
    ) -> HRESULT,

    pub DetachDisk: unsafe extern "system" fn(
        this: *mut ::core::ffi::c_void,
        disk: PCWSTR,
        result: *mut i32,
        step: *mut i32,
        error: *mut LXSS_ERROR_INFO,
    ) -> HRESULT,

    pub MountDisk: unsafe extern "system" fn(
        this: *mut ::core::ffi::c_void,
        disk: PCWSTR,
        flags: u32,
        partition_index: u32,
        name: PCWSTR,
        ttype: PCWSTR,
        options: PCWSTR,
        result: *mut i32,
        step: *mut i32,
        mount_name: *mut PWSTR,
        error: *mut LXSS_ERROR_INFO,
    ) -> HRESULT,

    pub Shutdown: unsafe extern "system" fn(this: *mut ::core::ffi::c_void, force: i32) -> HRESULT,

    pub ImportDistributionInplace: unsafe extern "system" fn(
        this: *mut ::core::ffi::c_void,
        name: PCWSTR,
        vhd_path: PCWSTR,
        error: *mut LXSS_ERROR_INFO,
        out_guid: *mut GUID,
    ) -> HRESULT,

    pub MoveDistribution: unsafe extern "system" fn(
        this: *mut ::core::ffi::c_void,
        distro_guid: *const GUID,
        name: PCWSTR,
        error: *mut LXSS_ERROR_INFO,
    ) -> HRESULT,
}

pub fn get_lxss_user_session() -> windows::core::Result<ILxssUserSession> {
    let session: ILxssUserSession =
        unsafe { CoCreateInstance(&CLSID_LXSSUSERSESSION, None, CLSCTX_LOCAL_SERVER)? };

    Ok(session)
}

impl ILxssUserSession {
    pub fn CreateInstance(&self, distro_guid: GUID, flags: u32) -> LxssResult<()> {
        unsafe {
            let vtable = self.0.vtable() as *const _ as *const ILxssUserSession_Vtbl;
            let mut error_info = std::mem::zeroed();
            let result = ((*vtable).CreateInstance)(
                self.0.as_raw(),
                std::ptr::from_ref(&distro_guid),
                flags,
                std::ptr::from_mut(&mut error_info),
            );
            if result.is_ok() {
                Ok(())
            } else {
                Err((result, error_info))
            }
        }
    }

    pub fn GetDefaultDistribution(&self) -> LxssResult<GUID> {
        unsafe {
            let vtable = self.vtable();
            let mut error_info: LXSS_ERROR_INFO = std::mem::zeroed();
            let mut guid = MaybeUninit::uninit();
            let result = ((*vtable).GetDefaultDistribution)(
                self.0.as_raw(),
                std::ptr::from_mut(&mut error_info),
                guid.as_mut_ptr(),
            );
            if result.is_ok() {
                Ok(guid.assume_init())
            } else {
                Err((result, error_info))
            }
        }
    }

    pub fn UnregisterDistribution(&self, distro_guid: GUID) -> LxssResult<()> {
        unsafe {
            let vtable = self.0.vtable() as *const _ as *const ILxssUserSession_Vtbl;
            let mut error_info = std::mem::zeroed();
            let result = ((*vtable).UnregisterDistribution)(
                self.0.as_raw(),
                std::ptr::from_ref(&distro_guid),
                std::ptr::from_mut(&mut error_info),
            );
            if result.is_ok() {
                Ok(())
            } else {
                Err((result, error_info))
            }
        }
    }

    pub fn TerminateDistribution(&self, distro_guid: GUID) -> LxssResult<()> {
        unsafe {
            let vtable = self.0.vtable() as *const _ as *const ILxssUserSession_Vtbl;
            let mut error_info = std::mem::zeroed();
            let result = ((*vtable).TerminateDistribution)(
                self.0.as_raw(),
                std::ptr::from_ref(&distro_guid),
                std::ptr::from_mut(&mut error_info),
            );
            if result.is_ok() {
                Ok(())
            } else {
                Err((result, error_info))
            }
        }
    }

    pub fn ConfigureDistribution(
        &self,
        distro_guid: GUID,
        default_uid: u32,
        flags: u32,
    ) -> LxssResult<()> {
        unsafe {
            let vtable = self.0.vtable() as *const _ as *const ILxssUserSession_Vtbl;
            let mut error_info = std::mem::zeroed();
            let result = ((*vtable).ConfigureDistribution)(
                self.0.as_raw(),
                std::ptr::from_ref(&distro_guid),
                default_uid,
                flags,
                std::ptr::from_mut(&mut error_info),
            );
            if result.is_ok() {
                Ok(())
            } else {
                Err((result, error_info))
            }
        }
    }

    pub unsafe fn EnumerateDistributions(&self) -> LxssResult<(u32, *const LXSS_ENUMERATE_INFO)> {
        unsafe {
            let vtable = self.0.vtable() as *const _ as *const ILxssUserSession_Vtbl;
            let mut error_info = std::mem::zeroed();
            let mut distros = std::ptr::null();
            let mut count = 0;
            let result = ((*vtable).EnumerateDistributions)(
                self.0.as_raw(),
                std::ptr::from_mut(&mut count),
                std::ptr::from_mut(&mut distros),
                std::ptr::from_mut(&mut error_info),
            );
            if result.is_ok() {
                Ok((count, distros))
            } else {
                Err((result, error_info))
            }
        }
    }

    pub fn SetVersion(
        &self,
        distro_guid: GUID,
        version: u32,
        stderr_handle: HANDLE,
    ) -> LxssResult<()> {
        unsafe {
            let vtable = self.0.vtable() as *const _ as *const ILxssUserSession_Vtbl;
            let mut error_info = std::mem::zeroed();
            let result = ((*vtable).SetVersion)(
                self.0.as_raw(),
                std::ptr::from_ref(&distro_guid),
                version,
                stderr_handle,
                std::ptr::from_mut(&mut error_info),
            );
            if result.is_ok() {
                Ok(())
            } else {
                Err((result, error_info))
            }
        }
    }

    pub fn RegisterDistribution(
        &self,
        name: PCWSTR,
        version: u32,
        file_handle: HANDLE,
        stderr_handle: HANDLE,
        target_directory: PCWSTR,
        flags: u32,
        vhd_size: u64, // zero = default size
        package_family_name: PCWSTR,
    ) -> LxssResult<RegisterDistributionResult> {
        unsafe {
            let vtable = self.0.vtable() as *const _ as *const ILxssUserSession_Vtbl;
            let mut error_info = std::mem::zeroed();
            let mut installed_name = PWSTR::null();
            let mut guid = MaybeUninit::uninit();
            let result = ((*vtable).RegisterDistribution)(
                self.0.as_raw(),
                name,
                version,
                file_handle,
                stderr_handle,
                target_directory,
                flags,
                vhd_size,
                package_family_name,
                std::ptr::from_mut(&mut installed_name),
                std::ptr::from_mut(&mut error_info),
                guid.as_mut_ptr(),
            );
            if result.is_ok() {
                Ok(RegisterDistributionResult {
                    Guid: guid.assume_init(),
                    InstalledName: installed_name,
                })
            } else {
                Err((result, error_info))
            }
        }
    }

    pub fn ExportDistribution(
        &self,
        distro_guid: GUID,
        file_handle: HANDLE,
        stderr_handle: HANDLE,
        flags: u32,
    ) -> LxssResult<()> {
        unsafe {
            let vtable = self.0.vtable() as *const _ as *const ILxssUserSession_Vtbl;
            let mut error_info = std::mem::zeroed();
            let result = ((*vtable).ExportDistribution)(
                self.0.as_raw(),
                std::ptr::from_ref(&distro_guid),
                file_handle,
                stderr_handle,
                flags,
                std::ptr::from_mut(&mut error_info),
            );
            if result.is_ok() {
                Ok(())
            } else {
                Err((result, error_info))
            }
        }
    }

    pub fn CreateLxProcess(
        &self,
        distro_guid: GUID,
        filename: PCSTR,
        command_line_count: u32,
        command_line: *const PCSTR,
        cwd: PCWSTR,
        nt_path: PCWSTR,
        nt_env: *mut u16,
        nt_env_len: u32,
        username: PCWSTR,
        columns: i16,
        rows: i16,
        console_handle: u32,
        std_handles: *const LXSS_STD_HANDLES,
        flags: u32,
    ) -> LxssResult<CreateLxProcessResult> {
        unsafe {
            let vtable = self.0.vtable() as *const _ as *const ILxssUserSession_Vtbl;
            let mut error_info = std::mem::zeroed();
            let mut lx_process_result = CreateLxProcessResult {
                DistributionId: GUID::default(),
                InstanceId: GUID::default(),
                ProcessHandle: HANDLE::default(),
                ServerHandle: HANDLE::default(),
                StandardIn: HANDLE::default(),
                StandardOut: HANDLE::default(),
                StandardErr: HANDLE::default(),
                CommunicationChannel: HANDLE::default(),
                InteropSocket: HANDLE::default(),
            };
            let result = ((*vtable).CreateLxProcess)(
                self.0.as_raw(),
                std::ptr::from_ref(&distro_guid),
                filename,
                command_line_count,
                command_line,
                cwd,
                nt_path,
                nt_env,
                nt_env_len,
                username,
                columns,
                rows,
                console_handle,
                std_handles,
                flags,
                std::ptr::from_mut(&mut lx_process_result.DistributionId),
                std::ptr::from_mut(&mut lx_process_result.InstanceId),
                std::ptr::from_mut(&mut lx_process_result.ProcessHandle),
                std::ptr::from_mut(&mut lx_process_result.ServerHandle),
                std::ptr::from_mut(&mut lx_process_result.StandardIn),
                std::ptr::from_mut(&mut lx_process_result.StandardOut),
                std::ptr::from_mut(&mut lx_process_result.StandardErr),
                std::ptr::from_mut(&mut lx_process_result.CommunicationChannel),
                std::ptr::from_mut(&mut lx_process_result.InteropSocket),
                std::ptr::from_mut(&mut error_info),
            );
            if result.is_ok() {
                Ok(lx_process_result)
            } else {
                Err((result, error_info))
            }
        }
    }

    pub fn Shutdown(&self, force: i32) -> Result<()> {
        unsafe {
            let vtable = self.0.vtable() as *const _ as *const ILxssUserSession_Vtbl;
            let result = ((*vtable).Shutdown)(self.0.as_raw(), force);
            if result.is_ok() {
                Ok(())
            } else {
                Err(result.into())
            }
        }
    }
}

pub struct RegisterDistributionResult {
    pub Guid: GUID,
    pub InstalledName: PWSTR,
}

pub struct CreateLxProcessResult {
    pub DistributionId: GUID,
    pub InstanceId: GUID,
    pub ProcessHandle: HANDLE,
    pub ServerHandle: HANDLE,
    pub StandardIn: HANDLE,
    pub StandardOut: HANDLE,
    pub StandardErr: HANDLE,
    pub CommunicationChannel: HANDLE,
    pub InteropSocket: HANDLE,
}
