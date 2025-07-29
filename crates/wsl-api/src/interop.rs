use std::{
    io::{BufRead, BufReader, Read},
    net::TcpStream,
    sync::mpsc::{self, Receiver, Sender},
    thread,
};
use wsl_com_api_sys::constants::*;
use wsl_com_api_sys::interop::{LX_INIT_PROCESS_EXIT_STATUS, MESSAGE_HEADER};

#[derive(Debug)]
pub struct Interop {
    term: Receiver<u32>,
    _thread: thread::JoinHandle<()>,
}

impl Interop {
    pub fn new(socket: TcpStream) -> Self {
        let (tx, rx) = mpsc::channel();

        let thread_handle = thread::spawn(move || {
            let _ = Self::process_messages(socket, tx);
        });

        Self {
            term: rx,
            _thread: thread_handle,
        }
    }

    fn process_messages(socket: TcpStream, tx: Sender<u32>) -> std::io::Result<()> {
        let mut reader = BufReader::new(&socket);
        let mut buffer = [0u8; 1024];

        loop {
            let header = unsafe { read_direct::<MESSAGE_HEADER, _>(&mut reader)? };
            match header.MessageType {
                LX_INIT_MESSAGE_EXIT_STATUS => {
                    let exit_status =
                        unsafe { read_direct::<LX_INIT_PROCESS_EXIT_STATUS, _>(&mut reader)? };
                    let _ = tx.send(exit_status.ExitCode);
                    return Ok(());
                }
                unknown => {
                    eprintln!("Unknown message type: {}", unknown);
                    // Unknown message type, try to read the rest of the message
                    let remaining_size =
                        header.MessageSize as usize - std::mem::size_of::<MESSAGE_HEADER>();
                    if remaining_size > 0 && remaining_size <= buffer.len() {
                        let _ = reader.read_exact(&mut buffer[..remaining_size]);
                    }
                }
            }
        }
    }

    pub fn recv_exit_code(&self) -> Option<u32> {
        self.term.recv().ok()
    }
}

/// Safety: the value you read must be POD.
unsafe fn read_direct<T, S: std::io::Read>(reader: &mut BufReader<S>) -> std::io::Result<T> {
    while reader.buffer().len() < std::mem::size_of::<T>() {
        reader.fill_buf()?;
    }
    let res = std::ptr::read_unaligned(reader.buffer().as_ptr() as *const T);
    reader.consume(std::mem::size_of::<T>());
    Ok(res)
}
