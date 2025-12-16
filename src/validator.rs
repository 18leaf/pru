use serde_json::json;

use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};

/// Takes Json Schema (From HAshmap on BAckend Struct)
/// Returns All Errors from schema validation as Lsp Daignostics with Error Severity
///
/// Improvements TODO
/// - Retrieve Actual Range for Diagnostic (Maps to File_contents) from JsonPointer
/// - Use above function with SchemaPath to get hint from SchemaPath
pub fn schema_validated_filecontents(
    json_schema: &serde_json::Value,
    file_contents: &str,
) -> Result<Vec<Diagnostic>, Box<dyn std::error::Error>> {
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
                .map(move |e| Diagnostic {
                    // TODO FOR RANGE -> take Json pointer from
                    // TODO create function to return File Position from JsonPointer/find crate
                    // e.instance_path() -> And map to a Range on the original file contents
                    range: Range {
                        ..Default::default()
                    },
                    severity: Some(DiagnosticSeverity::ERROR),
                    message: e.to_string(),
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
            let parse_diagnostic = Diagnostic {
                range: Range {
                    // can fail if usize > size of u32
                    start: Position {
                        line: e.line() as u32,
                        character: e.column() as u32,
                    },
                    // default for now.. maybe there is a better way for this
                    // TODO comeback and doublecheck
                    end: Default::default(),
                },
                message: e.to_string(),
                severity: Some(DiagnosticSeverity::ERROR),
                ..Default::default()
            };

            let diagnostics: Vec<Diagnostic> = vec![parse_diagnostic];
            Ok(diagnostics)
        }
    }
    // Ok => continue with validation on schema
    // Err -> invalid Json (maybe listen to serde_json and try to insert stuff to fix it here to
    // attempt to look at future fixes... ie insert colon at line x, char y  then try again.. so
    // forth)
    //
    // TODO here match serde_json::from_str for errors + generate diagnostics from them
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
}
