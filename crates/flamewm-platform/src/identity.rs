use std::collections::BTreeSet;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AppIdentityInput {
    pub desktop_id: String,
    pub startup_wm_class: String,
    pub wm_class: String,
    pub wm_instance: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextAction {
    Open,
    Unpin,
    Pin,
    Maximize,
    Minimize,
    Close,
    Separator,
}

#[must_use]
pub fn normalize_identity(raw: &str) -> String {
    raw.chars()
        .filter(|character| !character.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
}

#[must_use]
pub fn resolve_identity(input: &AppIdentityInput) -> String {
    for candidate in [
        &input.desktop_id,
        &input.startup_wm_class,
        &input.wm_class,
        &input.wm_instance,
    ] {
        if !candidate.is_empty() {
            return normalize_identity(candidate);
        }
    }
    "unknown".to_owned()
}

#[must_use]
pub fn merge_pinned_running(pinned: &[String], running: &[String]) -> Vec<String> {
    let mut seen = BTreeSet::new();
    pinned
        .iter()
        .chain(running)
        .map(|value| normalize_identity(value))
        .filter(|value| seen.insert(value.clone()))
        .collect()
}

#[must_use]
pub fn context_actions(
    pinned: bool,
    has_window: bool,
    visible: bool,
    maximized: bool,
) -> Vec<ContextAction> {
    if pinned && !has_window {
        return vec![ContextAction::Open, ContextAction::Unpin];
    }
    if !pinned && !has_window {
        return Vec::new();
    }
    let mut actions = vec![if pinned {
        ContextAction::Unpin
    } else {
        ContextAction::Pin
    }];
    actions.push(ContextAction::Separator);
    actions.push(if visible && maximized {
        ContextAction::Minimize
    } else {
        ContextAction::Maximize
    });
    actions.push(ContextAction::Close);
    actions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_order_never_uses_window_title() {
        let input = AppIdentityInput {
            desktop_id: String::new(),
            startup_wm_class: "Firefox".to_owned(),
            wm_class: "other".to_owned(),
            wm_instance: "Navigator".to_owned(),
        };
        assert_eq!(resolve_identity(&input), "firefox");
    }

    #[test]
    fn pinned_running_merge_is_stable_and_case_insensitive() {
        let pinned = vec!["Firefox".to_owned(), "code.desktop".to_owned()];
        let running = vec!["firefox".to_owned(), "Terminal".to_owned()];
        assert_eq!(
            merge_pinned_running(&pinned, &running),
            vec!["firefox", "code.desktop", "terminal"]
        );
    }
}
