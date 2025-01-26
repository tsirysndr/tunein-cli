use std::{path::Path, process::Command};

use anyhow::Error;

const SERVICE_TEMPLATE: &str = include_str!("./systemd/tunein.service");

pub fn install() -> Result<(), Error> {
    if cfg!(not(target_os = "linux")) {
        println!("This command is only supported on Linux");
        std::process::exit(1);
    }

    let home = std::env::var("HOME")?;
    let service_path: &str = &format!("{}/.config/systemd/user/tunein.service", home);
    std::fs::create_dir_all(format!("{}/.config/systemd/user", home))
        .expect("Failed to create systemd user directory");

    if Path::new(service_path).exists() {
        println!("Service file already exists. Nothing to install.");
        return Ok(());
    }

    let tunein_path = std::env::current_exe()?;

    let service_template: &str = &SERVICE_TEMPLATE.replace(
        "ExecStart=/usr/bin/tunein",
        &format!("ExecStart={}", tunein_path.display()),
    );

    std::fs::write(service_path, service_template).expect("Failed to write service file");

    Command::new("systemctl")
        .arg("--user")
        .arg("daemon-reload")
        .status()?;

    Command::new("systemctl")
        .arg("--user")
        .arg("enable")
        .arg("tunein")
        .status()?;

    Command::new("systemctl")
        .arg("--user")
        .arg("start")
        .arg("tunein")
        .status()?;

    println!("✅ Tunein service installed successfully!");

    Ok(())
}

pub fn uninstall() -> Result<(), Error> {
    if cfg!(not(target_os = "linux")) {
        println!("This command is only supported on Linux");
        std::process::exit(1);
    }

    let home = std::env::var("HOME")?;
    let service_path: &str = &format!("{}/.config/systemd/user/tunein.service", home);

    if Path::new(service_path).exists() {
        Command::new("systemctl")
            .arg("--user")
            .arg("stop")
            .arg("tunein")
            .status()?;

        Command::new("systemctl")
            .arg("--user")
            .arg("disable")
            .arg("tunein")
            .status()?;

        std::fs::remove_file(service_path).expect("Failed to remove service file");

        Command::new("systemctl")
            .arg("--user")
            .arg("daemon-reload")
            .status()?;

        println!("✅ Tunein service uninstalled successfully!");
    } else {
        println!("Service file does not exist. Nothing to uninstall.");
    }

    Ok(())
}

pub fn status() -> Result<(), Error> {
    if cfg!(not(target_os = "linux")) {
        println!("This command is only supported on Linux");
        std::process::exit(1);
    }

    let home = std::env::var("HOME")?;
    let service_path: &str = &format!("{}/.config/systemd/user/tunein.service", home);

    if Path::new(service_path).exists() {
        Command::new("systemctl")
            .arg("--user")
            .arg("status")
            .arg("tunein")
            .status()?;
    } else {
        println!("Service file does not exist. Tunein service is not installed.");
    }

    Ok(())
}
