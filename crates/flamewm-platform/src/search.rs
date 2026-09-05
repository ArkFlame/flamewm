#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SearchEntry {
    pub id: String,
    pub name: String,
    pub exec_display: String,
    pub keywords: String,
}

#[must_use]
pub fn matches(entry: &SearchEntry, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    let query = query.to_lowercase();
    [&entry.id, &entry.name, &entry.exec_display, &entry.keywords]
        .iter()
        .any(|field| field.to_lowercase().contains(&query))
}

#[must_use]
pub fn filter(entries: &[SearchEntry], query: &str) -> Vec<SearchEntry> {
    entries
        .iter()
        .filter(|entry| matches(entry, query))
        .cloned()
        .collect()
}

#[must_use]
pub fn sanitize_exec_for_display(exec: &str) -> String {
    let mut result = String::with_capacity(exec.len());
    let mut chars = exec.chars().peekable();
    while let Some(character) = chars.next() {
        if character == '%' {
            if let Some(next) = chars.peek().copied() {
                if matches!(
                    next,
                    'f' | 'F'
                        | 'u'
                        | 'U'
                        | 'd'
                        | 'D'
                        | 'n'
                        | 'N'
                        | 'i'
                        | 'c'
                        | 'k'
                        | 'v'
                        | 'm'
                        | '%'
                ) {
                    chars.next();
                    continue;
                }
            }
        }
        result.push(character);
    }
    result.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[must_use]
pub fn contains_shell_meta(value: &str) -> bool {
    value.chars().any(|character| {
        matches!(
            character,
            '`' | '$'
                | '|'
                | '&'
                | ';'
                | '<'
                | '>'
                | '('
                | ')'
                | '{'
                | '}'
                | '!'
                | '\\'
                | '\n'
                | '\r'
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_matches_name_keywords_and_identity() {
        let entry = SearchEntry {
            id: "org.mozilla.firefox.desktop".to_owned(),
            name: "Firefox".to_owned(),
            exec_display: "firefox".to_owned(),
            keywords: "browser web".to_owned(),
        };
        assert!(matches(&entry, "FIRE"));
        assert!(matches(&entry, "browser"));
        assert!(matches(&entry, "mozilla"));
        assert!(!matches(&entry, "editor"));
    }

    #[test]
    fn exec_display_removes_freedesktop_field_codes() {
        assert_eq!(
            sanitize_exec_for_display("code --new-window %F"),
            "code --new-window"
        );
    }

    #[test]
    fn launchers_can_reject_shell_interpolation() {
        assert!(contains_shell_meta("foo; rm -rf /"));
        assert!(!contains_shell_meta("firefox --new-window"));
    }
}
