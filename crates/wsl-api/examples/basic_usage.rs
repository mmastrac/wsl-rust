use wsl_api::{ExportFlags, ImportFlags, Version, Wsl};

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
    let default_distro = match wsl.get_default_distribution() {
        Ok(distro) => {
            println!("Successfully retrieved default distribution: {:?}", distro);
            distro
        }
        Err(e) => {
            eprintln!("Failed to get default distribution: {:?}", e);
            return Err(e.into());
        }
    };

    println!("Enumerating distributions...");
    match wsl.enumerate_distributions() {
        Ok(distros) => println!("Successfully enumerated distributions: {:?}", distros),
        Err(e) => {
            eprintln!("Failed to enumerate distributions: {:?}", e);
            return Err(e.into());
        }
    }

    println!("Exporting distribution...");
    let file = std::fs::File::create("distro.tar.gz").unwrap();
    let (r, w) = std::io::pipe().unwrap();
    let result = wsl.export_distribution(default_distro, file, w, ExportFlags::empty());
    // Keep the read end alive until after the export completes
    drop(r);
    match result {
        Ok(_) => println!("Successfully exported distribution"),
        Err(e) => {
            eprintln!("Failed to export distribution: {:?}", e);
            return Err(e.into());
        }
    }

    let file = std::fs::File::open("distro.tar.gz").unwrap();

    println!("Registering distribution...");
    let (r, w) = std::io::pipe().unwrap();
    let result = wsl.register_distribution("test", Version::WSL2, file, w, ImportFlags::empty());
    match result {
        Ok((guid, name)) => println!("Successfully registered distribution: {:?} {}", guid, name),
        Err(e) => {
            eprintln!("Failed to register distribution: {:?}", e);
            return Err(e.into());
        }
    };
    drop(r);

    println!("Enumerating distributions...");
    match wsl.enumerate_distributions() {
        Ok(distros) => println!("Successfully enumerated distributions: {:?}", distros),
        Err(e) => {
            eprintln!("Failed to enumerate distributions: {:?}", e);
            return Err(e.into());
        }
    }

    println!("Running command...");
    let result = wsl.launch(default_distro, "echo", &["Hello, world!"], None, "root");
    match result {
        Ok(process) => println!("Successfully ran command: {process:?}"),
        Err(e) => {
            eprintln!("Failed to run command: {:?}", e);
            return Err(e.into());
        }
    }

    println!("Shutting down WSL...");
    match wsl.shutdown(false) {
        Ok(_) => println!("Successfully shut down WSL"),
        Err(e) => {
            eprintln!("Failed to shut down WSL: {:?}", e);
            return Err(e.into());
        }
    }

    println!("Example completed successfully");
    Ok(())
}
