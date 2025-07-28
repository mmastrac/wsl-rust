use std::{
    sync::mpsc::{self, Receiver, Sender},
    thread,
    thread::JoinHandle,
};
use windows::{
    core::Result,
    Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED},
};
use wsl_com_api::{get_lxss_user_session, ILxssUserSession};

/// A higher-level API for interacting with WSL through COM
pub struct Wsl {
    /// Channel sender for communicating with the background COM thread
    sender: Sender<Box<dyn FnOnce(&ILxssUserSession) -> Result<()> + Send>>,
    /// Handle to the background thread
    _background_thread: JoinHandle<()>,
}

impl Wsl {
    /// Creates a new WSL API instance with a background COM thread
    pub fn new() -> Result<Self> {
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
    fn com_thread_worker(receiver: Receiver<Box<dyn FnOnce(&ILxssUserSession) -> Result<()> + Send>>) {
        // Initialize COM with apartment threading
        unsafe {
            let result = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            if result.is_err() {
                eprintln!("Failed to initialize COM: {:?}", result);
                return;
            }
        }
        
        // Get the WSL user session
        let session = match get_lxss_user_session() {
            Ok(session) => session,
            Err(e) => {
                eprintln!("Failed to get WSL user session: {:?}", e);
                unsafe { CoUninitialize(); }
                return;
            }
        };
        
        // Process requests from the main thread
        for request in receiver {
            if let Err(e) = request(&session) {
                eprintln!("Error executing WSL request: {:?}", e);
            }
        }
        
        // Cleanup COM
        unsafe { CoUninitialize(); }
    }
    
    /// Executes a function on the COM thread
    fn execute<F>(&self, f: F) -> Result<()>
    where
        F: FnOnce(&ILxssUserSession) -> Result<()> + Send + 'static,
    {
        self.sender.send(Box::new(f))
            .map_err(|_| windows::core::Error::from(windows::core::HRESULT::from_win32(1)))?;
        Ok(())
    }
    
    /// Example API method: Shutdown WSL
    pub fn shutdown(&self) -> Result<()> {
        self.execute(|session| {
            session.Shutdown(0)?;
            Ok(())
        })
    }
    
    /// Example API method: Get default distribution
    pub fn get_default_distribution(&self) -> Result<()> {
        self.execute(|_session| {
            // This would need to be implemented in wsl-com-api
            // For now, just a placeholder
            println!("Getting default distribution...");
            Ok(())
        })
    }
}

impl Drop for Wsl {
    fn drop(&mut self) {
        // The background thread will be joined when the JoinHandle is dropped
        // COM will be cleaned up in the background thread
    }
}
