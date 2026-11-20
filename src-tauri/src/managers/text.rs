//! Small helpers for reading column output.

/// Split a line into `n` whitespace-separated fields. The last field keeps
/// the rest of the line, including its inner spaces.
pub fn split_columns(line: &str, n: usize) -> Option<Vec<String>> {
    let mut fields = Vec::with_capacity(n);
    let mut rest = line.trim_start();
    for _ in 0..n.saturating_sub(1) {
        let end = rest.find(char::is_whitespace)?;
        fields.push(rest[..end].to_string());
        rest = rest[end..].trim_start();
    }
    if rest.is_empty() && n > 0 {
        return None;
    }
    fields.push(rest.trim_end().to_string());
    Some(fields)
}

/// Lines of output, without blank lines.
pub fn lines(s: &str) -> impl Iterator<Item = &str> {
    s.lines().filter(|l| !l.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_with_rest() {
        assert_eq!(
            split_columns("vlc  3.0.20  videolan✓  -  The ultimate media player", 5).unwrap(),
            vec![
                "vlc",
                "3.0.20",
                "videolan✓",
                "-",
                "The ultimate media player"
            ]
        );
        assert!(split_columns("only two", 3).is_none());
    }
}
