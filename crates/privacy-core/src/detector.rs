use crate::types::{SensitiveMatch, SensitiveType};

pub struct SensitiveDataDetector {
    patterns: Vec<(SensitiveType, regex::Regex)>,
}

impl SensitiveDataDetector {
    pub fn new() -> Self {
        let patterns = vec![
            (
                SensitiveType::Phone,
                regex::Regex::new(r"1[3-9]\d{9}").unwrap(),
            ),
            (
                SensitiveType::Email,
                regex::Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}").unwrap(),
            ),
            (
                SensitiveType::IpAddress,
                regex::Regex::new(r"\b(?:\d{1,3}\.){3}\d{1,3}\b").unwrap(),
            ),
            (
                SensitiveType::IdCard,
                regex::Regex::new(r"\b\d{17}[\dXx]\b").unwrap(),
            ),
            (
                SensitiveType::Token,
                regex::Regex::new(r"(?i)(token|key|secret|password)\s*[=:]\s*\S+").unwrap(),
            ),
        ];
        Self { patterns }
    }

    pub fn detect(&self, text: &str) -> Vec<SensitiveMatch> {
        let mut matches = Vec::new();
        for (stype, pattern) in &self.patterns {
            for m in pattern.find_iter(text) {
                matches.push(SensitiveMatch {
                    sensitive_type: *stype,
                    start: m.start(),
                    end: m.end(),
                    matched_text: m.as_str().to_string(),
                });
            }
        }
        matches
    }
}

impl Default for SensitiveDataDetector {
    fn default() -> Self {
        Self::new()
    }
}
