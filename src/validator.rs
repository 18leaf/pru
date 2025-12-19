use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};

use crate::error::SchemaValidationError;

/// Takes Json Schema (From HAshmap on BAckend Struct)
/// Returns All Errors from schema validation as Lsp Daignostics with Error Severity
///
/// Improvements TODO
/// - Retrieve Actual Range for Diagnostic (Maps to File_contents) from JsonPointer
/// - Use above function with SchemaPath to get hint from SchemaPath
pub fn schema_validated_filecontents(
    json_schema: &serde_json::Value,
    file_contents: &str,
) -> Result<Vec<Diagnostic>, SchemaValidationError> {
    // Step 1.. Corece filetext as string into JSON content
    // Errors Here are significiant
    let file_as_json = serde_json::from_str(file_contents);

    match file_as_json {
        Ok(file_as_json) => {
            // init validator to parse errors
            // if the below fails.. invalid schema is present (this should not really be something that can
            // happen. the schemas NEED to be correct for any of this to matter)
            let validator = jsonschema::validator_for(json_schema)
                .expect("Internal schema violated: Schema needs to be valid"); // expect since LSP
            // diagnostics are based on correctness of schema

            // map errors to diagnostics
            // see here for more info on ValidationError + uses
            // Additionally -> Here is where we can use SchemaPath -> JsonPointer as str to find correct
            // usage according to schema doc for hints/autocomplete
            // https://docs.rs/jsonschema/latest/jsonschema/error/struct.ValidationError.html
            let diagnostics: Vec<Diagnostic> = validator
                .iter_errors(&file_as_json)
                // todo.. Add Diagnostic Code for schema validation errors vs json syntax errors.
                .map(move |e| Diagnostic {
                    // TODO FOR RANGE -> take Json pointer from
                    // TODO create function to return File Position from JsonPointer/find crate
                    // e.instance_path() -> And map to a Range on the original file contents
                    severity: Some(DiagnosticSeverity::ERROR),
                    message: format!("Path {}, Error: {}", e.instance_path(), e.to_string()),
                    range: {
                        match json_pointer_into_range(e.instance_path().as_str(), file_contents) {
                            Some(r) => r,
                            None => Range {
                                ..Default::default()
                            },
                        }
                    },
                    source: Some(e.instance_path().to_string()),
                    ..Default::default()
                })
                .collect();

            Ok(diagnostics)
        }
        // Errpr section Handles Json Syntax errors -> from serde_json
        // needs to be improved to handle sequential, but minor erros.. IE if there is a single fix
        // suggested, look at that fix and modify file content buffer and then see if it works,
        // then reparse until either major error without clear solution.
        Err(e) => {
            let (line, column) = (e.line() as u32 - 1, e.column() as u32);
            let parse_diagnostic = Diagnostic {
                range: Range {
                    // can fail if usize > size of u32
                    start: Position {
                        line: line,
                        character: 0,
                    },
                    end: Position {
                        line: line,
                        character: column - 1,
                    },
                },
                // Note could use a DiagnosticRelatedInformation struct here instead.. as it
                // points to the error in source code where error occurs.. Come back here
                message: e.to_string(),
                severity: Some(DiagnosticSeverity::ERROR),
                ..Default::default()
            };

            let diagnostics: Vec<Diagnostic> = vec![parse_diagnostic];
            Ok(diagnostics)
        }
    }
}

/// Converts Json Pointer to start Position, end Position
/// Takes a &str JsonPointer and the original raw_file_contents,
/// outputs None on no find, match on something.
fn json_pointer_into_range(json_pointer: &str, raw_file_contents: &str) -> Option<Range> {
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

    // stacked_file_contents -> shrinks at each iteration of found path
    let mut stacked_file_contents = raw_file_contents.to_owned().clone();
    let mut index_summation: usize = 0;
    for path_item in json_pointer.split("/") {
        // if not found, continue.. search for next item
        let temp_index = stacked_file_contents.find(&path_item).unwrap_or(0);
        index_summation += temp_index;
        stacked_file_contents = stacked_file_contents.split_off(temp_index);
    }

    // count byte occurences of newline char for the line position.
    let line_number = raw_file_contents[..index_summation]
        .chars()
        .filter(|x| *x == '\n')
        .count() as u32;

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

#[cfg(test)]
pub mod tests {
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
