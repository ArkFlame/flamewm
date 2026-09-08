#[must_use]
pub fn workspace_labels(count: usize) -> Vec<String> {
    (1..=count).map(|index| index.to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::workspace_labels;

    #[test]
    fn named_workspace_compact_labels_are_numeric_index_plus_one() {
        assert_eq!(workspace_labels(4), vec!["1", "2", "3", "4"]);
    }
}
