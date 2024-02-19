#[derive(Debug, Clone)]
pub enum ErrorLevel {
    Fatal,
    Warning,

    /// A Raw error should not be formatted with get_message().
    /// And it is for private use.
    Raw,
}

#[derive(Debug, Clone)]
pub struct PhpError {
    pub level: ErrorLevel,
    pub message: String,

    /// Note that in many parts of the program this field will be set to 0.
    /// This is because it is another part of the program that has the line
    /// where the error was generated and not the part that creates the structure.
    pub line: usize,
}

impl PhpError {
    pub fn get_message(self, input: &str) -> String {
        if let ErrorLevel::Raw = self.level {
            return self.message;
        }

        let level_error = match self.level {
            ErrorLevel::Fatal => "Fatal error",
            ErrorLevel::Warning => "Warning",
            _ => "",
        };

        format!(
            "PHP {}: {} in {} on line {}",
            level_error, self.message, input, self.line
        )
    }
}
