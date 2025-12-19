use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};
use tracing::{debug, error, info, instrument, trace, warn};

use crate::{error::SchemaValidationError, validator::validation::SchemaValidator};

/// Takes Json Schema (From HAshmap on BAckend Struct)
/// Returns All Errors from schema validation as Lsp Daignostics with Error Severity
///
/// Improvements TODO
/// - Retrieve Actual Range for Diagnostic (Maps to File_contents) from JsonPointer
/// - Use above function with SchemaPath to get hint from SchemaPath
#[instrument(skip(json_schema, file_contents), fields(content_len = file_contents.len()))]
pub fn schema_validated_filecontents(
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

mod parsing {
    use super::*;

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
}

use parsing::ParsedContent;

mod validation {
    use crate::validator::pointer::DiagnosticRange;

    use super::*;

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
            let range =
                DiagnosticRange::from_pointer(error.instance_path().as_str(), file_contents);

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
}

mod pointer {
    use super::*;

    /// Helper for resolving diagnostic ranges from JSON pointers
    pub struct DiagnosticRange;

    impl DiagnosticRange {
        /// Resolves the range for a diagnostic from a JSON pointer
        #[instrument(skip(file_contents), fields(pointer = json_pointer))]
        pub fn from_pointer(json_pointer: &str, file_contents: &str) -> Range {
            match json_pointer_into_range(json_pointer, file_contents) {
                Some(range) => {
                    trace!(
                        line = range.start.line,
                        character = range.start.character,
                        "Successfully resolved diagnostic range"
                    );
                    range
                }
                None => {
                    debug!(
                        pointer = json_pointer,
                        "Failed to resolve range, using default"
                    );
                    Range::default()
                }
            }
        }
    }

    /// Converts Json Pointer to start Position, end Position
    /// Takes a &str JsonPointer and the original raw_file_contents,
    /// outputs None on no find, match on something.
    #[instrument(skip(raw_file_contents), fields(
    pointer = json_pointer,
    content_len = raw_file_contents.len()
))]
    pub fn json_pointer_into_range(json_pointer: &str, raw_file_contents: &str) -> Option<Range> {
        trace!("Converting JSON pointer to range");

        // json pointer looks like it gives the parent object//parent node of the error

        // since json pointer starts with /root/node/node/etc
        // iterate through / and then search for match

        // within json_pointer
        // convert to iterator
        // for each iteration
        //      find index of first char of matching iteration of json_pointer
        //      drop all string items before x
        //      increment summation index by index of that match
        // once final iteration occurs -> Found match... search for (in order { (then find next closing
        // symbol = } ), OR NEWLINE ... only NEWLINE for now)
        // find distance until NEWLINE / end terminator
        // that == end position of range

        let index_summation = PointerIndex::calculate(json_pointer, raw_file_contents);

        debug!(
            pointer = json_pointer,
            resolved_index = index_summation,
            "Calculated index for JSON pointer"
        );

        // count byte occurences of newline char for the line position.
        let line_number = LineNumber::from_index(raw_file_contents, index_summation);

        trace!(line = line_number, "Calculated line number from index");

        // note the + 1
        // editor start line number @ 1
        Some(Range {
            start: Position {
                line: line_number,
                character: 0,
            },
            end: Position {
                line: line_number,
                character: 0,
            },
        })
    }

    /// Calculates the byte index in the file for a given JSON pointer
    struct PointerIndex;

    impl PointerIndex {
        #[instrument(skip(raw_file_contents), fields(pointer = json_pointer))]
        fn calculate(json_pointer: &str, raw_file_contents: &str) -> usize {
            // stacked_file_contents -> shrinks at each iteration of found path
            let mut stacked_file_contents = raw_file_contents.to_owned();
            let mut index_summation: usize = 0;

            let path_items: Vec<&str> = json_pointer.split('/').collect();
            trace!(
                path_count = path_items.len(),
                "Splitting JSON pointer into path items"
            );

            for (idx, path_item) in path_items.iter().enumerate() {
                // if not found, continue.. search for next item
                let temp_index = stacked_file_contents.find(path_item).unwrap_or(0);

                if temp_index == 0 && !path_item.is_empty() {
                    debug!(
                        path_item = path_item,
                        iteration = idx,
                        "Path item not found in remaining content"
                    );
                }

                index_summation += temp_index;
                stacked_file_contents = stacked_file_contents.split_off(temp_index);

                trace!(
                    iteration = idx,
                    path_item = path_item,
                    temp_index = temp_index,
                    cumulative_index = index_summation,
                    "Processed path item"
                );
            }

            index_summation
        }
    }

    /// Calculates the line number from a byte index
    struct LineNumber;

    impl LineNumber {
        #[instrument(skip(raw_file_contents))]
        fn from_index(raw_file_contents: &str, index: usize) -> u32 {
            let safe_index = index.min(raw_file_contents.len());

            let line_number = raw_file_contents[..safe_index]
                .chars()
                .filter(|x| *x == '\n')
                .count() as u32;

            trace!(
                index = safe_index,
                line_number = line_number,
                "Calculated line number from index"
            );

            line_number
        }
    }
}

#[cfg(test)]
pub mod tests {
    use crate::validator::pointer::json_pointer_into_range;

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

        let diagnostics = schema_validated_filecontents(&test.json_schema, &test_control_json)?;
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
        let diagnostics = schema_validated_filecontents(&test.json_schema, &test_control_json);

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

        let diagnostics = schema_validated_filecontents(&test.json_schema, &test_error);

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
                let range = json_pointer_into_range(&d.source.clone().unwrap(), &test_error);
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
