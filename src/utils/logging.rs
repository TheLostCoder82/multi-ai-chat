//! Logging utilities for the application

use log::{LevelFilter, Metadata, Record};

/// Initialize the logging system
pub fn init_logging() {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info")
    ).init();
    
    log::info!("Logging initialized");
}

/// Custom logger for filtering specific message types
pub struct FilteredLogger {
    level: LevelFilter,
}

impl FilteredLogger {
    pub fn new(level: LevelFilter) -> Self {
        Self { level }
    }
    
    pub fn should_log(&self, record: &Record) -> bool {
        record.level() <= self.level
    }
}

/// Truncate a string to a maximum length with ellipsis
pub fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

/// Truncate content to a maximum word count
pub fn truncate_words(content: &str, max_words: usize) -> String {
    let words: Vec<&str> = content.split_whitespace().collect();
    if words.len() <= max_words {
        content.to_string()
    } else {
        format!("{}...", words[..max_words].join(" "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_string_short() {
        let result = truncate_string("Hello", 10);
        assert_eq!(result, "Hello");
    }

    #[test]
    fn test_truncate_string_long() {
        let result = truncate_string("Hello, World!", 5);
        assert_eq!(result, "He...");
    }

    #[test]
    fn test_truncate_words_short() {
        let result = truncate_words("Hello world", 10);
        assert_eq!(result, "Hello world");
    }

    #[test]
    fn test_truncate_words_long() {
        let result = truncate_words("This is a long sentence with many words", 5);
        assert_eq!(result, "This is a long...");
    }
}
