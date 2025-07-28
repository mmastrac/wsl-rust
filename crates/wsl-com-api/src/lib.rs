use windows::{
    core::{GUID, Interface, HRESULT, IUnknown, Result},
    Win32::Foundation::HANDLE,
    Win32::System::Com::{CoCreateInstance, CLSCTX_LOCAL_SERVER},
};

const CLSID_LXSSUSERSESSION: GUID = GUID::from_u128(0xa9b7a1b9_0671_405c_95f1_e0612cb4ce7e);
const IID_ILXSSUSERSESSION: GUID = GUID::from_u128(0x38541bdc_f54f_4ceb_85d0_37f0f3d2617e);

#[repr(C)] pub struct LXSS_ERROR_INFO {
    pub Flags: u32,
    pub Context: u64,
    pub Message: *mut u16,
    pub Warnings: *mut u16,
    pub WarningsPipe: u32,
}

#[repr(C)] pub struct LXSS_ENUMERATE_INFO {
    pub DistroGuid: GUID,
    pub State: u32,
    pub Version: u32,
    pub Flags: u32,
    pub DistroName: [u16; 257],
}

#[repr(C)] pub struct LXSS_HANDLE {
    pub Handle: u32,
    pub HandleType: u32,
}

#[repr(C)] pub struct LXSS_STD_HANDLES {
    pub StdIn: LXSS_HANDLE,
    pub StdOut: LXSS_HANDLE,
    pub StdErr: LXSS_HANDLE,
}

#[repr(transparent)]
#[derive(Clone)]
pub struct ILxssUserSession(IUnknown);

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
        name: *const u16,
        version: u32,
        file_handle: HANDLE,
        stderr_handle: HANDLE,
        target_directory: *const u16,
        flags: u32,
        vhd_size: u64,
        package_family_name: *const u16,
        installed_name: *mut *mut u16,
        error: *mut LXSS_ERROR_INFO,
        out_guid: *mut GUID,
    ) -> HRESULT,

    pub RegisterDistributionPipe: unsafe extern "system" fn(
        this: *mut ::core::ffi::c_void,
        name: *const u16,
        version: u32,
        pipe_handle: HANDLE,
        stderr_handle: HANDLE,
        target_directory: *const u16,
        flags: u32,
        vhd_size: u64,
        package_family_name: *const u16,
        installed_name: *mut *mut u16,
        error: *mut LXSS_ERROR_INFO,
        out_guid: *mut GUID,
    ) -> HRESULT,

    pub GetDistributionId: unsafe extern "system" fn(
        this: *mut ::core::ffi::c_void,
        name: *const u16,
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
        distribution_name: *mut *mut u16,
        version: *mut u32,
        default_uid: *mut u32,
        env_count: *mut u32,
        default_environment: *mut *mut *mut i8,
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
        distros: *mut *mut LXSS_ENUMERATE_INFO,
        error: *mut LXSS_ERROR_INFO,
    ) -> HRESULT,

    pub CreateLxProcess: unsafe extern "system" fn(
        this: *mut ::core::ffi::c_void,
        distro_guid: *const GUID,
        filename: *const i8,
        command_line_count: u32,
        command_line: *const *const i8,
        cwd: *const u16,
        nt_path: *const u16,
        nt_env: *mut u16,
        nt_env_len: u32,
        username: *const u16,
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
        disk: *const u16,
        flags: u32,
        error: *mut LXSS_ERROR_INFO,
    ) -> HRESULT,

    pub DetachDisk: unsafe extern "system" fn(
        this: *mut ::core::ffi::c_void,
        disk: *const u16,
        result: *mut i32,
        step: *mut i32,
        error: *mut LXSS_ERROR_INFO,
    ) -> HRESULT,

    pub MountDisk: unsafe extern "system" fn(
        this: *mut ::core::ffi::c_void,
        disk: *const u16,
        flags: u32,
        partition_index: u32,
        name: *const u16,
        ttype: *const u16,
        options: *const u16,
        result: *mut i32,
        step: *mut i32,
        mount_name: *mut *mut u16,
        error: *mut LXSS_ERROR_INFO,
    ) -> HRESULT,

    pub Shutdown: unsafe extern "system" fn(
        this: *mut ::core::ffi::c_void,
        force: i32,
    ) -> HRESULT,

    pub ImportDistributionInplace: unsafe extern "system" fn(
        this: *mut ::core::ffi::c_void,
        name: *const u16,
        vhd_path: *const u16,
        error: *mut LXSS_ERROR_INFO,
        out_guid: *mut GUID,
    ) -> HRESULT,

    pub MoveDistribution: unsafe extern "system" fn(
        this: *mut ::core::ffi::c_void,
        distro_guid: *const GUID,
        name: *const u16,
        error: *mut LXSS_ERROR_INFO,
    ) -> HRESULT,
}

pub fn get_lxss_user_session() -> Result<ILxssUserSession> {
    unsafe { CoCreateInstance(&CLSID_LXSSUSERSESSION, None, CLSCTX_LOCAL_SERVER) }
}

impl ILxssUserSession {
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
