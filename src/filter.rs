use regex::Regex;

/// A range representing a match within a line
#[derive(Clone, Copy, Debug)]
pub struct MatchRange {
    pub start: usize,
    pub end: usize,
}

/// A filter that can be applied to log lines
#[derive(Clone)]
pub struct ActiveFilter {
    /// The pattern string
    pub pattern: String,
    /// Whether to treat the pattern as a regex
    pub is_regex: bool,
    /// Compiled regex (if is_regex is true and pattern is valid)
    compiled: Option<Regex>,
    /// Lowercase pattern for case-insensitive substring matching
    pattern_lower: String,
}

impl ActiveFilter {
    pub fn new(pattern: String, is_regex: bool) -> Self {
        let compiled = if is_regex {
            Regex::new(&pattern).ok()
        } else {
            None
        };
        let pattern_lower = pattern.to_lowercase();

        Self {
            pattern,
            is_regex,
            compiled,
            pattern_lower,
        }
    }

    /// Check if a line matches this filter
    pub fn matches(&self, line: &str) -> bool {
        if self.is_regex {
            if let Some(ref regex) = self.compiled {
                regex.is_match(line)
            } else {
                // Invalid regex, treat as substring match
                line.contains(&self.pattern)
            }
        } else {
            // Case-insensitive substring match
            line.to_lowercase().contains(&self.pattern_lower)
        }
    }

    /// Find all match ranges in a line
    pub fn find_matches(&self, line: &str) -> Vec<MatchRange> {
        let mut matches = Vec::new();

        if self.is_regex {
            if let Some(ref regex) = self.compiled {
                for m in regex.find_iter(line) {
                    matches.push(MatchRange {
                        start: m.start(),
                        end: m.end(),
                    });
                }
            } else {
                // Invalid regex, fall back to substring
                self.find_substring_matches(line, &mut matches);
            }
        } else {
            self.find_substring_matches(line, &mut matches);
        }

        matches
    }

    /// Find all case-insensitive substring matches
    fn find_substring_matches(&self, line: &str, matches: &mut Vec<MatchRange>) {
        if self.pattern_lower.is_empty() {
            return;
        }

        let line_lower = line.to_lowercase();
        let mut start = 0;

        while let Some(pos) = line_lower[start..].find(&self.pattern_lower) {
            let match_start = start + pos;
            let match_end = match_start + self.pattern.len();
            matches.push(MatchRange {
                start: match_start,
                end: match_end,
            });
            start = match_end;
        }
    }
}

/// A saved filter with a name
#[derive(Clone)]
pub struct SavedFilter {
    pub name: String,
    pub pattern: String,
    pub is_regex: bool,
}
