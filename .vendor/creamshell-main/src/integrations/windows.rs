use std::process::Command;

pub fn powershell(script: &str) -> Option<String> {
    Command::new("powershell.exe")
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            script,
        ])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

pub fn run_powershell(script: String) {
    std::thread::spawn(move || {
        let _ = powershell(&script);
    });
}
