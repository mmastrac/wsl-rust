use std::io::BufRead;

use wsl_api::{ExportFlags, ImportFlags, Version, Wsl2, WslErrorKind};

fn run_command(wsl: &Wsl2, distro_uuid: uuid::Uuid) -> Result<(), Box<dyn std::error::Error>> {
    println!("Running command...");
    let result = wsl.launch(
        distro_uuid,
        "/bin/echo",
        &["echo", "Hello, world!"],
        None,
        "root",
    );
    let mut process = {
        match result {
            Ok(process) => {
                println!("Successfully ran command: {process:?}");
                process
            }
            Err(e) => {
                eprintln!("Failed to run command: {:?}", e);
                return Err(e.into());
            }
        }
    };

    let stdout = process.stdout.take().unwrap();
    let stderr = process.stderr.take().unwrap();

    let stdout_thread = std::thread::spawn(move || {
        let reader = std::io::BufReader::new(stdout);
        for line in reader.lines() {
            match line {
                Ok(line) => println!("stdout: {}", line),
                Err(e) => eprintln!("Error reading stdout: {}", e),
            }
        }
    });

    let stderr_thread = std::thread::spawn(move || {
        let reader = std::io::BufReader::new(stderr);
        for line in reader.lines() {
            match line {
                Ok(line) => println!("stderr: {}", line),
                Err(e) => eprintln!("Error reading stderr: {}", e),
            }
        }
    });

    println!("Waiting for process to finish...");
    let status = process.wait()?;
    println!("Process finished with status: {status:?}");

    stdout_thread.join().unwrap();
    stderr_thread.join().unwrap();

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating WSL API instance...");

    let wsl = match Wsl2::new() {
        Ok(wsl) => {
            println!("WSL API instance created successfully");
            wsl
        }
        Err(e) if e.kind() == Some(WslErrorKind::UnsupportedOperatingSystem) => {
            eprintln!("WSL is not installed or enabled on this operating system");
            return Ok(());
        }
        Err(e) if e.kind() == Some(WslErrorKind::UnsupportedWslVersion) => {
            eprintln!("WSL version is not supported");
            return Ok(());
        }
        Err(e) => {
            eprintln!("Failed to create WSL1/2 API instance: {:?}", e);
            eprintln!("This may be due to WSL2 not being installed or enabled");
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
    let guid_copy = match result {
        Ok((guid, name)) => {
            println!("Successfully registered distribution: {:?} {}", guid, name);
            guid
        }
        Err(e) => {
            eprintln!("Failed to register distribution: {:?}", e);
            return Err(e.into());
        }
    };
    drop(r);

    println!("Setting version...");
    let result = wsl.set_version(guid_copy, Version::WSL1, std::io::stderr());
    match result {
        Ok(_) => println!("Successfully set version"),
        Err(e) => {
            eprintln!("Failed to set version: {:?}", e);
            return Err(e.into());
        }
    }

    println!("Enumerating distributions...");
    match wsl.enumerate_distributions() {
        Ok(distros) => println!("Successfully enumerated distributions: {:?}", distros),
        Err(e) => {
            eprintln!("Failed to enumerate distributions: {:?}", e);
            return Err(e.into());
        }
    }

    run_command(&wsl, default_distro)?;

    run_command(&wsl, guid_copy)?;

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
