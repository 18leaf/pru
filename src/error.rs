use thiserror::Error;
use tower_lsp::lsp_types::{Position, Range};

#[derive(Error, Debug)]
pub enum SchemaValidationError {
    /// JSON parsing failed when reading file contents
    #[error("Failed to parse JSON: {0}")]
    JsonParseError(#[from] serde_json::Error),

    /// Schema file could not be read
    #[error("Failed to read schema file: {0}")]
    SchemaFileReadError(#[from] std::io::Error),

    /// The provided JSON schema itself is invalid
    #[error("Invalid JSON schema provided: {0}")]
    InvalidSchemaError(String),

    /// JSON schema validation failed (contains diagnostics)
    #[error("Schema validation failed with {0} error(s)")]
    ValidationFailed(usize),

    /// Failed to compile the JSON schema validator
    #[error("Failed to compile schema validator: {0}")]
    ValidatorCompilationError(String),

    /// JSON pointer path could not be resolved
    #[error("Failed to resolve JSON pointer path: {0}")]
    JsonPointerResolutionError(String),

    /// Failed to convert position data (e.g., usize to u32)
    #[error("Position conversion overflow at line {line}, column {column}")]
    PositionConversionError { line: usize, column: usize },

    /// Invalid JSON pointer format
    #[error("Invalid JSON pointer format: {0}")]
    InvalidJsonPointer(String),

    /// Range calculation failed
    #[error("Failed to calculate range for diagnostic at {0}")]
    RangeCalculationError(String),

    /// File contents are empty or invalid
    #[error("File contents are empty or invalid")]
    EmptyFileContents,

    /// UTF-8 encoding error in file contents
    #[error("Invalid UTF-8 in file contents: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),

    /// Generic diagnostic generation error
    #[error("Failed to generate diagnostic: {0}")]
    DiagnosticGenerationError(String),
}

// Helper type alias for Results using this error type
pub type ValidationResult<T> = Result<T, SchemaValidationError>;

impl SchemaValidationError {
    /// Convert error to LSP diagnostic range (useful for error recovery)
    pub fn to_diagnostic_range(&self) -> Range {
        Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 0,
                character: 0,
            },
        }
    }

    /// Check if the error is recoverable (can continue processing)
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            SchemaValidationError::JsonPointerResolutionError(_)
                | SchemaValidationError::RangeCalculationError(_)
                | SchemaValidationError::ValidationFailed(_)
        )
    }
}

