use std::sync::OnceLock;

use regex::Regex;
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};
use tracing::{debug, error, instrument, trace, warn};

use crate::error::SchemaValidationError;

/// Returns a Schema identifier if one can be found, trying to use the standard for the particular language, otherwise falling back to a regex solution
pub fn extract_schema_reference(content: &serde_json::Value) -> Option<String> {
    // Check shebang pattern first (must be on first line, first char)
    if let Some(schema) = check_shebang_schema(&content.to_string()) {
        return Some(schema);
    }

    // Fall back to JSON $schema field
    Some(content.get("$schema")?.to_string())
}

/// Check for shebang-style: #$schema IDENTIFIER
/// Must be at the very start of the file (first line, first character)
fn check_shebang_schema(content: &str) -> Option<String> {
    static SHEBANG_REGEX: OnceLock<Regex> = OnceLock::new();
    let regex = SHEBANG_REGEX.get_or_init(|| {
        // Match #$schema followed by whitespace and capture the identifier
        // \S+ captures non-whitespace characters (the identifier)
        Regex::new(r"^#\$schema\s+(\S+)").expect("Valid regex")
    });

    // Only check the first line
    let first_line = content.lines().next()?;

    regex
        .captures(first_line)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().to_string())
}

/// Internal enum to represent parsed content state
pub enum ParsedContent {
    Valid(serde_json::Value),
    ParseError(Diagnostic),
}

impl ParsedContent {
    /// Parses JSON content and converts errors to diagnostics
    #[instrument(skip(file_contents), fields(content_len = file_contents.len()))]
    pub fn new(file_contents: &str) -> Result<Self, SchemaValidationError> {
        trace!("Attempting to parse file contents as JSON");

        match serde_json::from_str(file_contents) {
            Ok(json) => {
                debug!("Successfully parsed JSON content");
                Ok(ParsedContent::Valid(json))
            }
            Err(e) => {
                error!(error = %e, "JSON parsing failed");
                Ok(ParsedContent::ParseError(
                    ParseErrorDiagnostic::from(e).into(),
                ))
            }
        }
    }
}

/// Wrapper for creating parse error diagnostics
pub struct ParseErrorDiagnostic {
    line: u32,
    column: u32,
    message: String,
}

impl From<serde_json::Error> for ParseErrorDiagnostic {
    #[instrument(skip(error), fields(line = error.line(), column = error.column()))]
    fn from(error: serde_json::Error) -> Self {
        let (line, column) = (error.line() as u32 - 1, error.column() as u32);

        trace!(
            line = line,
            column = column,
            error = %error,
            "Creating parse error diagnostic"
        );

        Self {
            line,
            column,
            message: error.to_string(),
        }
    }
}

impl From<ParseErrorDiagnostic> for Diagnostic {
    fn from(diag: ParseErrorDiagnostic) -> Self {
        Diagnostic {
            range: Range {
                // can fail if usize > size of u32
                start: Position {
                    line: diag.line,
                    character: 0,
                },
                end: Position {
                    line: diag.line,
                    character: diag.column.saturating_sub(1),
                },
            },
            // Note could use a DiagnosticRelatedInformation struct here instead.. as it
            // points to the error in source code where error occurs.. Come back here
            message: diag.message,
            severity: Some(DiagnosticSeverity::ERROR),
            ..Default::default()
        }
    }
}
