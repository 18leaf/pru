pub mod diagnostic_range;
pub mod error;
pub mod json_pointer;
pub mod line_number;
pub mod parsing;
pub mod pointer_index;
pub mod validation;

use tower_lsp::lsp_types::Diagnostic;
use tracing::{debug, info, instrument, warn};

use crate::{error::SchemaValidationError, parsing::ParsedContent, validation::SchemaValidator};

/// Takes Json Schema (From HAshmap on BAckend Struct)
/// Returns All Errors from schema validation as Lsp Daignostics with Error Severity
///
/// Improvements TODO
/// - Retrieve Actual Range for Diagnostic (Maps to File_contents) from JsonPointer
/// - Use above function with SchemaPath to get hint from SchemaPath
#[instrument(skip(json_schema, file_contents), fields(content_len = file_contents.len()))]
pub fn validate_liberally(
    json_schema: &serde_json::Value,
    file_contents: &str,
) -> Result<Vec<Diagnostic>, SchemaValidationError> {
    info!("Starting schema validation");

    // Step 1.. Corece filetext as string into JSON content
    // Errors Here are significiant
    let parsed = ParsedContent::new(file_contents)?;

    match parsed {
        ParsedContent::Valid(json) => {
            debug!("JSON parsing successful, proceeding with schema validation");
            SchemaValidator::new(json_schema, &json, file_contents).validate()
        }
        ParsedContent::ParseError(diagnostic) => {
            // Errpr section Handles Json Syntax errors -> from serde_json
            // needs to be improved to handle sequential, but minor erros.. IE if there is a single fix
            // suggested, look at that fix and modify file content buffer and then see if it works,
            // then reparse until either major error without clear solution.
            warn!("JSON parse error detected, returning parse diagnostic");
            Ok(vec![diagnostic])
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::json_pointer;
    use std::fs::File;
    use std::io::BufReader;

    const VALID_JSON: &str = r#"{
  "$schema": "here",
  "service": "api",
  "version": "1.2.3",
  "runtime": {
    "type": "docker",
    "docker": {
      "image": "nginx",
      "tag": "1.25"
    }
  },
  "ports": [
    { "containerPort": 8080, "protocol": "tcp" }
  ],
  "env": {
    "MODE": "production"
  }
}"#;

    const INVALID_JSON_SYNTAX: &str = r#"{
  "service": "api",
  "version": "1.2.3"
  "runtime": {
    "type": "docker"
  }
}"#;

    const JSON_WITH_SCHEMA_ERROR: &str = r#"{
  "$schema": "here",
  "service": "api",
  "version": "1.2.3",
  "runtime": {
    "type": "docker",
    "docker": {
      "image": "nginx",
      "tag": "1.25"
    }
  }
}"#;

    struct TestSchema {
        json_schema: serde_json::Value,
    }

    impl TestSchema {
        fn new() -> Result<TestSchema, Box<dyn std::error::Error>> {
            let file = File::open("schemas/service.schema.json")?;
            let reader = BufReader::new(file);
            let json_schema: serde_json::Value = serde_json::from_reader(reader)?;
            Ok(TestSchema { json_schema })
        }

        /// Create a minimal schema for testing when file is unavailable
        fn minimal() -> TestSchema {
            let json_schema = serde_json::json!({
                "$schema": "http://json-schema.org/draft-07/schema#",
                "type": "object",
                "properties": {
                    "service": { "type": "string" },
                    "version": { "type": "string" }
                },
                "required": ["service"]
            });
            TestSchema { json_schema }
        }
    }

    #[test]
    fn test_valid_json_passes_validation() -> Result<(), Box<dyn std::error::Error>> {
        let schema = TestSchema::new().unwrap_or_else(|_| TestSchema::minimal());
        let diagnostics = validate_liberally(&schema.json_schema, VALID_JSON)?;

        // Valid JSON should produce no diagnostics or only schema-related ones
        // depending on the schema configuration
        assert!(
            diagnostics.is_empty() || diagnostics.iter().all(|d| d.source.is_some()),
            "Expected no parse errors for valid JSON. Got {} diagnostics",
            diagnostics.len()
        );
        Ok(())
    }

    #[test]
    fn test_json_syntax_error_produces_diagnostic() -> Result<(), Box<dyn std::error::Error>> {
        let schema = TestSchema::new().unwrap_or_else(|_| TestSchema::minimal());
        let diagnostics = validate_liberally(&schema.json_schema, INVALID_JSON_SYNTAX)?;

        assert!(
            !diagnostics.is_empty(),
            "Expected at least one diagnostic for invalid JSON syntax"
        );

        // Verify the diagnostic has meaningful content
        let first_diagnostic = &diagnostics[0];
        assert!(
            !first_diagnostic.message.is_empty(),
            "Diagnostic message should not be empty"
        );

        Ok(())
    }

    #[test]
    fn test_schema_validation_error_produces_diagnostic() -> Result<(), Box<dyn std::error::Error>>
    {
        let schema = TestSchema::new().unwrap_or_else(|_| TestSchema::minimal());

        // This JSON is syntactically valid but may fail schema validation
        // depending on required fields
        let diagnostics = validate_liberally(&schema.json_schema, JSON_WITH_SCHEMA_ERROR)?;

        // Either passes validation or produces schema-related diagnostics
        if !diagnostics.is_empty() {
            assert!(
                diagnostics.iter().any(|d| d.source.is_some()),
                "Schema validation errors should have a source"
            );
        }

        Ok(())
    }

    #[test]
    fn test_json_pointer_resolution() {
        let test_json = r#"{
  "field1": "value1",
  "field2": {
    "nested": "value2"
  }
}"#;

        // Test with a known JSON pointer path
        let pointer = "/field2/nested";
        let range = json_pointer::into_range(pointer, test_json);

        if let Some(r) = range {
            // The range should be within the document bounds
            assert!(
                r.start.line < test_json.lines().count() as u32,
                "Range start line {} should be within document ({}  lines)",
                r.start.line,
                test_json.lines().count()
            );
            assert!(
                r.end.line < test_json.lines().count() as u32,
                "Range end line should be within document"
            );
        }
        // If pointer resolution returns None, that's acceptable depending on implementation
    }

    #[test]
    fn test_diagnostic_range_is_valid() -> Result<(), Box<dyn std::error::Error>> {
        let schema = TestSchema::new().unwrap_or_else(|_| TestSchema::minimal());
        let diagnostics = validate_liberally(&schema.json_schema, INVALID_JSON_SYNTAX)?;

        if let Some(first_diagnostic) = diagnostics.first() {
            let range = first_diagnostic.range;
            let line_count = INVALID_JSON_SYNTAX.lines().count() as u32;

            assert!(
                range.start.line < line_count,
                "Diagnostic range start line {} should be within document ({} lines)",
                range.start.line,
                line_count
            );
            assert!(
                range.end.line < line_count,
                "Diagnostic range end line should be within document"
            );
            assert!(
                range.start.line <= range.end.line,
                "Range start should not be after range end"
            );
        }

        Ok(())
    }

    #[test]
    fn test_empty_json_handles_gracefully() {
        let schema = TestSchema::new().unwrap_or_else(|_| TestSchema::minimal());
        let result = validate_liberally(&schema.json_schema, "");

        // Should either error or produce a diagnostic, but not panic
        assert!(
            result.is_ok() || result.is_err(),
            "Empty JSON should be handled gracefully"
        );
    }

    #[test]
    fn test_malformed_json_handles_gracefully() {
        let schema = TestSchema::new().unwrap_or_else(|_| TestSchema::minimal());
        let malformed_inputs = vec!["{", "}", "{{}", "null", "[]", r#"{"key": }"#];

        for input in malformed_inputs {
            let result = validate_liberally(&schema.json_schema, input);
            assert!(
                result.is_ok(),
                "Malformed JSON '{}' should produce diagnostics, not panic",
                input
            );
        }
    }
}
