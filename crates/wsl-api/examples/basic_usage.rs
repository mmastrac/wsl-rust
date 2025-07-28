use wsl_api::Wsl;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating WSL API instance...");

    let wsl = match Wsl::new() {
        Ok(wsl) => {
            println!("WSL API instance created successfully");
            wsl
        }
        Err(e) => {
            eprintln!("Failed to create WSL API instance: {:?}", e);
            eprintln!("This may be due to:");
            eprintln!("  - WSL not being installed or enabled");
            eprintln!("  - Insufficient permissions (COM operations require admin privileges)");
            eprintln!("  - Running in a CI environment without WSL support");
            return Err(e.into());
        }
    };

    println!("Getting default distribution...");
    match wsl.get_default_distribution() {
        Ok(distro) => println!("Successfully retrieved default distribution: {:?}", distro),
        Err(e) => {
            eprintln!("Failed to get default distribution: {:?}", e);
        }
    }

    println!("Enumerating distributions...");
    match wsl.enumerate_distributions() {
        Ok(distros) => println!("Successfully enumerated distributions: {:?}", distros),
        Err(e) => {
            eprintln!("Failed to enumerate distributions: {:?}", e);
        }
    }

    println!("Shutting down WSL...");
    match wsl.shutdown(false) {
        Ok(_) => println!("Successfully shut down WSL"),
        Err(e) => eprintln!("Failed to shut down WSL: {:?}", e),
    }

    println!("Example completed successfully");
    Ok(())
}
