use windows::core::HRESULT;

#[derive(Debug)]
enum UnderlyingError {
    Lxss(wsl_com_api_sys::LxssError),
    Windows(windows::core::Error),
}

#[derive(Debug)]
pub struct WslError {
    underlying: UnderlyingError,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum WslErrorKind {
    UnsupportedOperatingSystem,
    UnsupportedWslVersion,
}

impl WslError {
    pub fn hresult(&self) -> HRESULT {
        match &self.underlying {
            UnderlyingError::Lxss(e) => e.0,
            UnderlyingError::Windows(e) => e.code(),
        }
    }

    pub fn kind(&self) -> Option<WslErrorKind> {
        #[cfg(not(windows))]
        return Some(WslErrorKind::UnsupportedOperatingSystem);

        #[cfg(windows)]
        match self.hresult() {
            windows::Win32::Foundation::REGDB_E_CLASSNOTREG => Some(WslErrorKind::UnsupportedWslVersion),
            _ => None,
        }
    }
}

impl std::fmt::Display for WslError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let known_error = known_error(self.hresult());
        if known_error.is_empty() {
            write!(
                f,
                "Unknown WSL error 0x{:08x}: {}",
                self.hresult().0,
                self.hresult().message()
            )
        } else {
            write!(f, "WSL error: {}", known_error)
        }
    }
}

impl std::error::Error for WslError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.underlying {
            UnderlyingError::Lxss(_) => None,
            UnderlyingError::Windows(e) => Some(e),
        }
    }
}

impl From<wsl_com_api_sys::LxssError> for WslError {
    fn from(value: wsl_com_api_sys::LxssError) -> Self {
        WslError {
            underlying: UnderlyingError::Lxss(value),
        }
    }
}

impl From<windows::core::Error> for WslError {
    fn from(value: windows::core::Error) -> Self {
        WslError {
            underlying: UnderlyingError::Windows(value),
        }
    }
}

fn known_error(error: HRESULT) -> &'static str {
    use wsl_com_api_sys::error::*;
    match error {
        WSL_E_DEFAULT_DISTRO_NOT_FOUND => "Default distribution not found",
        WSL_E_DISTRO_NOT_FOUND => "Distribution not found",
        WSL_E_WSL1_NOT_SUPPORTED => "WSL 1 not supported",
        WSL_E_VM_MODE_NOT_SUPPORTED => "VM mode not supported",
        WSL_E_TOO_MANY_DISKS_ATTACHED => "Too many disks attached",
        WSL_E_CONSOLE => "Console",
        WSL_E_CUSTOM_KERNEL_NOT_FOUND => "Custom kernel not found",
        WSL_E_USER_NOT_FOUND => "User not found",
        WSL_E_INVALID_USAGE => "Invalid usage",
        WSL_E_EXPORT_FAILED => "Export failed",
        WSL_E_IMPORT_FAILED => "Import failed",
        WSL_E_DISTRO_NOT_STOPPED => "Distribution not stopped",
        WSL_E_TTY_LIMIT => "TTY limit",
        WSL_E_CUSTOM_SYSTEM_DISTRO_ERROR => "Custom system distro error",
        WSL_E_LOWER_INTEGRITY => "Lower integrity",
        WSL_E_HIGHER_INTEGRITY => "Higher integrity",
        WSL_E_FS_UPGRADE_NEEDED => "FS upgrade needed",
        WSL_E_USER_VHD_ALREADY_ATTACHED => "User VHD already attached",
        WSL_E_VM_MODE_INVALID_STATE => "VM mode invalid state",
        WSL_E_VM_MODE_MOUNT_NAME_ALREADY_EXISTS => "VM mode mount name already exists",
        WSL_E_ELEVATION_NEEDED_TO_MOUNT_DISK => "Elevation needed to mount disk",
        WSL_E_DISK_ALREADY_ATTACHED => "Disk already attached",
        WSL_E_DISK_ALREADY_MOUNTED => "Disk already mounted",
        WSL_E_DISK_MOUNT_FAILED => "Disk mount failed",
        WSL_E_DISK_UNMOUNT_FAILED => "Disk unmount failed",
        WSL_E_WSL2_NEEDED => "WSL 2 needed",
        WSL_E_VM_MODE_INVALID_MOUNT_NAME => "VM mode invalid mount name",
        WSL_E_GUI_APPLICATIONS_DISABLED => "GUI applications disabled",
        WSL_E_DISTRO_ONLY_AVAILABLE_FROM_STORE => "Distribution only available from store",
        WSL_E_WSL_MOUNT_NOT_SUPPORTED => "WSL mount not supported",
        WSL_E_WSL_OPTIONAL_COMPONENT_REQUIRED => "WSL optional component required",
        WSL_E_VMSWITCH_NOT_FOUND => "VMSwitch not found",
        WSL_E_VMSWITCH_NOT_SET => "VMSwitch not set",
        WSL_E_NOT_A_LINUX_DISTRO => "Not a Linux distro",
        WSL_E_OS_NOT_SUPPORTED => "OS not supported",
        WSL_E_INSTALL_PROCESS_FAILED => "Install process failed",
        WSL_E_INSTALL_COMPONENT_FAILED => "Install component failed",
        WSL_E_DISK_MOUNT_DISABLED => "Disk mount disabled",
        WSL_E_WSL1_DISABLED => "WSL 1 disabled",
        WSL_E_VIRTUAL_MACHINE_PLATFORM_REQUIRED => "Virtual machine platform required",
        WSL_E_LOCAL_SYSTEM_NOT_SUPPORTED => "Local system not supported",
        WSL_E_DISK_CORRUPTED => "Disk corrupted",
        WSL_E_DISTRIBUTION_NAME_NEEDED => "Distribution name needed",
        WSL_E_INVALID_JSON => "Invalid JSON",
        WSL_E_VM_CRASHED => "VM crashed",
        _ => "",
    }
}
