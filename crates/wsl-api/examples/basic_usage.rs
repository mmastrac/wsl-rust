use wsl_api::Wsl;

fn main() -> windows::core::Result<()> {
    println!("Creating WSL API instance...");
    
    // Create a new WSL API instance with background COM thread
    let wsl = Wsl::new()?;
    
    println!("WSL API instance created successfully");
    
    // Example: Get default distribution
    println!("Getting default distribution...");
    wsl.get_default_distribution()?;
    
    // Example: Shutdown WSL (commented out to avoid terminating all instances)
    // println!("Shutting down WSL...");
    // wsl.shutdown()?;
    
    println!("Example completed successfully");
    Ok(())
} 