//! FlameWM session launch and supervision policy.
//!
//! This crate is intentionally independent. It plans FlameWM-owned processes only; process
//! spawning, signals, and parent-death handling remain adapter responsibilities.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionPaths {
    pub bindir: PathBuf,
    pub config_home: PathBuf,
    pub data_root: PathBuf,
}

impl SessionPaths {
    #[must_use]
    pub fn flamewm_binary(&self) -> PathBuf {
        self.bindir.join("flamewm")
    }

    #[must_use]
    pub fn shell_binary(&self) -> PathBuf {
        self.bindir.join("flamewm-shell")
    }

    #[must_use]
    pub fn config_dir(&self) -> PathBuf {
        self.config_home.join("flamewm")
    }

    #[must_use]
    pub fn cursor_path(&self) -> PathBuf {
        self.data_root.join("flamewm/cursors")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionRole {
    WindowManager,
    Shell,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessPlan {
    pub role: SessionRole,
    pub program: PathBuf,
    pub arguments: Vec<String>,
    pub environment: BTreeMap<String, String>,
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionLaunchPlan {
    pub processes: Vec<ProcessPlan>,
}

#[must_use]
pub fn build_launch_plan(
    paths: &SessionPaths,
    wm_passthrough: &[String],
    existing_env: &BTreeMap<String, String>,
) -> SessionLaunchPlan {
    build_launch_plan_with_cursor_library_path(paths, wm_passthrough, existing_env, None)
}

#[must_use]
pub fn build_launch_plan_with_cursor_library_path(
    paths: &SessionPaths,
    wm_passthrough: &[String],
    existing_env: &BTreeMap<String, String>,
    cursor_library_path: Option<&str>,
) -> SessionLaunchPlan {
    let environment = flame_environment(paths, existing_env, cursor_library_path);
    let wm_arguments = wm_passthrough
        .iter()
        .filter(|argument| !argument.starts_with("--config-dir="))
        .cloned()
        .collect::<Vec<_>>();

    SessionLaunchPlan {
        processes: vec![
            ProcessPlan {
                role: SessionRole::WindowManager,
                program: paths.flamewm_binary(),
                arguments: wm_arguments,
                environment: environment.clone(),
                required: true,
            },
            ProcessPlan {
                role: SessionRole::Shell,
                program: paths.shell_binary(),
                arguments: Vec::new(),
                environment,
                required: true,
            },
        ],
    }
}

fn flame_environment(
    paths: &SessionPaths,
    existing_env: &BTreeMap<String, String>,
    cursor_library_path: Option<&str>,
) -> BTreeMap<String, String> {
    let mut environment = BTreeMap::new();
    environment.insert(
        "XCURSOR_THEME".to_owned(),
        existing_env
            .get("XCURSOR_THEME")
            .cloned()
            .unwrap_or_else(|| "Breeze".to_owned()),
    );
    environment.insert(
        "XCURSOR_SIZE".to_owned(),
        existing_env
            .get("XCURSOR_SIZE")
            .cloned()
            .unwrap_or_else(|| "24".to_owned()),
    );
    environment.insert(
        "XCURSOR_THEME_CORE".to_owned(),
        existing_env
            .get("XCURSOR_THEME_CORE")
            .cloned()
            .unwrap_or_else(|| "1".to_owned()),
    );
    environment.insert(
        "FLAMEWM_CONFIG_DIR".to_owned(),
        paths.config_dir().display().to_string(),
    );

    let cursor_path = paths.cursor_path().display().to_string();
    let inherited_path = existing_env
        .get("XCURSOR_PATH")
        .filter(|value| !value.is_empty())
        .map(String::as_str)
        .or(cursor_library_path.filter(|value| !value.is_empty()));
    let combined_cursor_path = inherited_path.map_or(cursor_path.clone(), |existing| {
        format!("{cursor_path}:{existing}")
    });
    environment.insert("XCURSOR_PATH".to_owned(), combined_cursor_path);
    environment
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SupervisorPolicy {
    pub maximum_restarts: u8,
    pub initial_backoff_ms: u64,
    pub maximum_backoff_ms: u64,
    pub graceful_stop_ms: u64,
}

impl Default for SupervisorPolicy {
    fn default() -> Self {
        Self {
            maximum_restarts: 5,
            initial_backoff_ms: 500,
            maximum_backoff_ms: 8_000,
            graceful_stop_ms: 1_500,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChildExit {
    Success,
    ExitCode(i32),
    Signal(i32),
    SpawnFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupervisorDecision {
    SpawnNow,
    RestartAfter(u64),
    StopCleanly,
    GiveUp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessSupervisorPolicy {
    policy: SupervisorPolicy,
    restarts: u8,
    stopping: bool,
    gave_up: bool,
}

impl Default for ProcessSupervisorPolicy {
    fn default() -> Self {
        Self::new(SupervisorPolicy::default())
    }
}

impl ProcessSupervisorPolicy {
    #[must_use]
    pub const fn new(policy: SupervisorPolicy) -> Self {
        Self {
            policy,
            restarts: 0,
            stopping: false,
            gave_up: false,
        }
    }

    #[must_use]
    pub const fn restarts(&self) -> u8 {
        self.restarts
    }

    #[must_use]
    pub const fn gave_up(&self) -> bool {
        self.gave_up
    }

    pub fn request_stop(&mut self) -> SupervisorDecision {
        self.stopping = true;
        SupervisorDecision::StopCleanly
    }

    pub fn parent_died(&mut self) -> SupervisorDecision {
        self.stopping = true;
        SupervisorDecision::StopCleanly
    }

    pub fn child_exited(&mut self, _exit: ChildExit) -> SupervisorDecision {
        if self.stopping {
            return SupervisorDecision::StopCleanly;
        }
        if self.restarts >= self.policy.maximum_restarts {
            self.gave_up = true;
            return SupervisorDecision::GiveUp;
        }
        let delay = self.backoff_for(self.restarts);
        self.restarts = self.restarts.saturating_add(1);
        SupervisorDecision::RestartAfter(delay)
    }

    #[must_use]
    pub const fn graceful_stop_ms(&self) -> u64 {
        self.policy.graceful_stop_ms
    }

    #[must_use]
    fn backoff_for(&self, attempt: u8) -> u64 {
        let shift = u32::from(attempt.min(7));
        self.policy
            .initial_backoff_ms
            .saturating_mul(1_u64 << shift)
            .min(self.policy.maximum_backoff_ms)
    }
}

pub type DesktopSupervisorPolicy = ProcessSupervisorPolicy;

#[must_use]
pub fn default_paths_from_home(bindir: impl AsRef<Path>, home: impl AsRef<Path>) -> SessionPaths {
    let bindir = bindir.as_ref().to_path_buf();
    SessionPaths {
        data_root: bindir
            .parent()
            .unwrap_or_else(|| Path::new("/usr"))
            .join("share"),
        bindir,
        config_home: home.as_ref().join(".config"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launch_plan_contains_only_flamewm_owned_processes() {
        let paths = default_paths_from_home("/usr/bin", "/home/test");
        let plan = build_launch_plan(&paths, &[], &BTreeMap::new());
        assert_eq!(plan.processes.len(), 2);
        assert_eq!(plan.processes[0].program, PathBuf::from("/usr/bin/flamewm"));
        assert_eq!(
            plan.processes[1].program,
            PathBuf::from("/usr/bin/flamewm-shell")
        );
    }

    #[test]
    fn config_directory_override_cannot_be_injected_through_passthrough() {
        let paths = default_paths_from_home("/usr/bin", "/home/test");
        let passthrough = vec!["--replace".to_owned(), "--config-dir=/tmp/evil".to_owned()];
        let plan = build_launch_plan(&paths, &passthrough, &BTreeMap::new());
        assert_eq!(plan.processes[0].arguments, vec!["--replace"]);
    }

    #[test]
    fn cursor_theme_core_and_library_fallback_are_explicit() {
        let paths = default_paths_from_home("/usr/bin", "/home/test");
        let plan = build_launch_plan_with_cursor_library_path(
            &paths,
            &[],
            &BTreeMap::new(),
            Some("/usr/share/icons:/usr/share/pixmaps"),
        );
        let environment = &plan.processes[0].environment;
        assert_eq!(
            environment.get("XCURSOR_THEME_CORE").map(String::as_str),
            Some("1")
        );
        assert!(
            environment
                .get("XCURSOR_PATH")
                .is_some_and(|value| value.ends_with("/usr/share/icons:/usr/share/pixmaps"))
        );
    }

    #[test]
    fn restart_policy_is_bounded() {
        let mut supervisor = ProcessSupervisorPolicy::default();
        let mut delays = Vec::new();
        for _ in 0..5 {
            if let SupervisorDecision::RestartAfter(delay) =
                supervisor.child_exited(ChildExit::ExitCode(1))
            {
                delays.push(delay);
            }
        }
        assert_eq!(delays, vec![500, 1_000, 2_000, 4_000, 8_000]);
        assert_eq!(
            supervisor.child_exited(ChildExit::ExitCode(1)),
            SupervisorDecision::GiveUp
        );
    }
}
