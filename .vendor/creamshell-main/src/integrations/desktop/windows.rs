use super::DesktopIntegration;
use crate::integrations::windows::powershell;
use crate::platform::OpenWindow;
use creamui_render::WindowHandle;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub struct WindowsDesktop {
    windows: Arc<Mutex<Vec<OpenWindow>>>,
}

impl WindowsDesktop {
    pub fn detect() -> Option<Self> {
        powershell("$PSVersionTable.PSVersion.Major").map(|_| Self::new())
    }

    fn new() -> Self {
        let windows = Arc::new(Mutex::new(Vec::new()));
        let cache = windows.clone();
        thread::spawn(move || loop {
            if let Ok(mut current) = cache.lock() {
                *current = query_windows();
            }
            thread::sleep(Duration::from_millis(700));
        });
        Self { windows }
    }
}

impl DesktopIntegration for WindowsDesktop {
    fn prepare_window(&self, window: &WindowHandle) {
        window.set_always_on_top(true);
    }

    fn windows(&self) -> Vec<OpenWindow> {
        self.windows
            .lock()
            .map(|windows| windows.clone())
            .unwrap_or_default()
    }

    fn activate_window(&self, id: &str) {
        let id = id.to_owned();
        thread::spawn(move || {
            let _ = powershell(&format!(
                "$shell = New-Object -ComObject WScript.Shell; [void]$shell.AppActivate({id})"
            ));
        });
    }
}

fn query_windows() -> Vec<OpenWindow> {
    let script = "Get-Process | Where-Object { $_.MainWindowHandle -ne 0 } | ForEach-Object { \"$($_.Id)`t$($_.ProcessName)`t$($_.MainWindowTitle)\" }";
    powershell(script)
        .map(|output| {
            output
                .lines()
                .filter_map(|line| {
                    let mut fields = line.splitn(3, '\t');
                    let id = fields.next()?.trim();
                    let app_name = fields.next()?.trim();
                    let title = fields.next()?.trim();
                    (!title.is_empty() && !title.starts_with("CreamShell")).then(|| OpenWindow {
                        id: id.to_owned(),
                        app_name: app_name.to_owned(),
                        title: title.to_owned(),
                        icon_path: None,
                        active: false,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}
