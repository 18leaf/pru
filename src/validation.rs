use crate::{diagnostic_range, error::SchemaValidationError};

use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Range};
use tracing::{debug, info, instrument, trace, warn};

/// Validates JSON against schema and returns diagnostics
pub struct SchemaValidator<'a> {
    json_schema: &'a serde_json::Value,
    file_as_json: &'a serde_json::Value,
    file_contents: &'a str,
}

impl<'a> SchemaValidator<'a> {
    pub fn new(
        json_schema: &'a serde_json::Value,
        file_as_json: &'a serde_json::Value,
        file_contents: &'a str,
    ) -> Self {
        Self {
            json_schema,
            file_as_json,
            file_contents,
        }
    }

    #[instrument(skip(self))]
    pub fn validate(self) -> Result<Vec<Diagnostic>, SchemaValidationError> {
        // init validator to parse errors
        // if the below fails.. invalid schema is present (this should not really be something that can
        // happen. the schemas NEED to be correct for any of this to matter)
        trace!("Creating schema validator");
        let validator = jsonschema::validator_for(self.json_schema)
            .expect("Internal schema violated: Schema needs to be valid"); // expect since LSP
        // diagnostics are based on correctness of schema

        debug!("Schema validator created successfully");

        // map errors to diagnostics
        // see here for more info on ValidationError + uses
        // Additionally -> Here is where we can use SchemaPath -> JsonPointer as str to find correct
        // usage according to schema doc for hints/autocomplete
        // https://docs.rs/jsonschema/latest/jsonschema/error/struct.ValidationError.html
        let validation_errors: Vec<_> = validator.iter_errors(self.file_as_json).collect();

        if validation_errors.is_empty() {
            info!("Schema validation passed with no errors");
        } else {
            warn!(
                error_count = validation_errors.len(),
                "Schema validation found errors"
            );
        }

        let diagnostics = validation_errors
            .into_iter()
            // todo.. Add Diagnostic Code for schema validation errors vs json syntax errors.
            .map(|e| ValidationDiagnostic::new(e, self.file_contents).into())
            .collect();

        Ok(diagnostics)
    }
}

/// Wrapper for creating validation diagnostics
pub struct ValidationDiagnostic {
    instance_path: String,
    error_message: String,
    range: Range,
}

impl ValidationDiagnostic {
    #[instrument(skip(error, file_contents), fields(instance_path = %error.instance_path()))]
    pub fn new(error: jsonschema::ValidationError, file_contents: &str) -> Self {
        let instance_path = error.instance_path().to_string();
        let error_message = error.to_string();

        trace!(
            path = %instance_path,
            error = %error_message,
            "Creating validation diagnostic"
        );

        // TODO FOR RANGE -> take Json pointer from
        // TODO create function to return File Position from JsonPointer/find crate
        // e.instance_path() -> And map to a Range on the original file contents
        let range = diagnostic_range::from_pointer(error.instance_path().as_str(), file_contents);

        Self {
            instance_path,
            error_message,
            range,
        }
    }
}

impl From<ValidationDiagnostic> for Diagnostic {
    fn from(diag: ValidationDiagnostic) -> Self {
        Diagnostic {
            severity: Some(DiagnosticSeverity::ERROR),
            message: format!("Path {}, Error: {}", diag.instance_path, diag.error_message),
            range: diag.range,
            source: Some(diag.instance_path),
            ..Default::default()
        }
    }
}
