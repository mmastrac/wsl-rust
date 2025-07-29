use windows::core::GUID;

// Interop message structures
#[repr(C)]
pub struct MESSAGE_HEADER {
    pub MessageType: u32,
    pub MessageSize: u32,
}

#[repr(C)]
pub struct LX_INIT_CREATE_NT_PROCESS_COMMON {
    pub FilenameOffset: u32,
    pub CommandLineOffset: u32,
    pub CurrentWorkingDirectoryOffset: u32,
    pub EnvironmentOffset: u32,
    pub CommandLineCount: u16,
    pub Rows: u32,
    pub Columns: u32,
    pub CreatePseudoconsole: u32,
}

#[repr(C)]
pub struct LX_INIT_CREATE_NT_PROCESS {
    pub Common: LX_INIT_CREATE_NT_PROCESS_COMMON,
    pub StdFdIds: [u32; 3],
}

#[repr(C)]
pub struct LX_INIT_CREATE_PROCESS_RESPONSE {
    pub Result: i32,
    pub Flags: u32,
    pub SignalPipeId: u32,
}

#[repr(C)]
pub struct LX_INIT_PROCESS_EXIT_STATUS {
    pub ExitCode: u32,
}

#[repr(C)]
pub struct LX_INIT_WINDOW_SIZE_CHANGED {
    pub Rows: u32,
    pub Columns: u32,
}

#[repr(C)]
pub struct LX_INIT_CREATE_NT_PROCESS_UTILITY_VM {
    pub Common: LX_INIT_CREATE_NT_PROCESS_COMMON,
    pub VmId: GUID,
    pub Port: u32,
}

#[repr(C)]
pub struct LXBUS_IPC_MESSAGE_MARSHAL_HANDLE_DATA {
    pub Handle: u32,
    pub HandleType: u32,
}

#[repr(C)]
pub union LXBUS_IPC_LX_PROCESS_WAIT_FOR_TERMINATION_PARAMETERS {
    pub Input: LXBUS_IPC_LX_PROCESS_WAIT_FOR_TERMINATION_INPUT,
    pub Output: LXBUS_IPC_LX_PROCESS_WAIT_FOR_TERMINATION_OUTPUT,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct LXBUS_IPC_LX_PROCESS_WAIT_FOR_TERMINATION_INPUT {
    pub TimeoutMs: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct LXBUS_IPC_LX_PROCESS_WAIT_FOR_TERMINATION_OUTPUT {
    pub ExitStatus: i32,
}
