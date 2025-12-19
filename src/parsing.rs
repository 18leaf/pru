use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};
use tracing::{debug, error, instrument, trace, warn};

use crate::error::SchemaValidationError;

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
