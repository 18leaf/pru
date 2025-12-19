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
    use tower_lsp::lsp_types::{Position, Range};

    use crate::json_pointer;

    use super::*;
    use std::fs::File;
    use std::io::BufReader;

    const TEST_CONTROL_JSON: &str = r#"{
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

    const TEST_ERROR_JSON: &str = r#"{
  "service": "api",
  "version": "1.2.3",
  "runtime": {
    "type": "docker",
    "ocker": {
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

    struct TestSchema {
        json_schema: serde_json::Value,
    }

    impl TestSchema {
        fn new() -> Result<TestSchema, Box<dyn std::error::Error>> {
            let file = File::open("schemas/service.schema.json")?;
            let reader = BufReader::new(file);
            let json_schema: serde_json::Value = serde_json::from_reader(reader)?;

            Ok(TestSchema {
                json_schema: json_schema,
            })
        }
    }

    #[test]
    fn validate_schema_works() -> Result<(), Box<dyn std::error::Error>> {
        // hardocded -- should pass
        let test_control_json = TEST_CONTROL_JSON;
        let test = TestSchema::new()?;

        let diagnostics = validate_liberally(&test.json_schema, &test_control_json)?;
        let expected_diagnostics: Vec<Diagnostic> = Vec::default();

        assert_eq!(diagnostics, expected_diagnostics);
        Ok(())
    }

    #[test]
    fn first_diagnostic_found() -> Result<(), Box<dyn std::error::Error>> {
        // hardocded -- should pass
        let mut test_control_json = TEST_CONTROL_JSON.to_owned();
        // fopefully remove semi colon
        test_control_json.remove(13);

        let test = TestSchema::new()?;
        let diagnostics = validate_liberally(&test.json_schema, &test_control_json);

        match diagnostics {
            Ok(e) => assert!(e.len() >= 1usize),
            Err(e) => eprintln!("{}", e),
        }
        Ok(())
    }

    #[test]
    fn json_pointer_to_range_works() {
        let test_error = TEST_ERROR_JSON.to_owned();

        let test = TestSchema::new().expect("For testing");

        let diagnostics = validate_liberally(&test.json_schema, &test_error);

        let expected_range = Range {
            start: Position {
                line: 1,
                character: 0,
            },
            end: Position {
                line: 1,
                character: 0,
            },
        };

        match diagnostics {
            Ok(ds) => {
                let d = ds.iter().next().unwrap();
                dbg!(&d.source);
                let range = json_pointer::into_range(&d.source.clone().unwrap(), &test_error);
                match range {
                    Some(r) => {
                        eprintln!("line: {}, char: {}", r.start.line, r.start.character);
                        assert_eq!(range, Some(expected_range));
                    }
                    None => eprintln!("None Found"),
                }
            }
            Err(e) => {
                eprintln!("Internal Error: {}", e);
            }
        }
    }
}
