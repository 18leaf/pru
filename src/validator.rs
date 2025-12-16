use serde_json::json;

// TODO! -> maybe read file contents as string instead.... as this will be reading from an LSP on text
// document opened. lifetime param for file in.. not suer about json schema path.. this can be
// loaded and kept in memory for the server, as it will need be validated throughout changes.

/// takes a Json_schema Object, and text of file contents
///
/// lifetime of jsonValue fro json schema might need to persist.. double check this later
pub fn schema_validated_filecontents<'filetext>(
    json_schema: &serde_json::Value,
    file_contents: &'filetext str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Step 1.. Corece filetext as string into JSON content
    let file_as_json: serde_json::Value = serde_json::from_str(file_contents)?;

    // init validator to parse errors
    // if the below fails.. invalid schema is present (for now assume not possible)
    let validator = jsonschema::validator_for(json_schema)?;

    // iterator for each error present
    for error in validator.iter_errors(&file_as_json) {
        eprintln!("Error: {error}");
        eprintln!("Location: {}", error.instance_path());
    }

    Ok(())
}

fn example() -> Result<(), Box<dyn std::error::Error>> {
    let schema = json!({"maxLength": 5});
    let instance = json!("foo");

    // One-off validation
    assert!(jsonschema::is_valid(&schema, &instance));
    assert!(jsonschema::validate(&schema, &instance).is_ok());

    // Build & reuse (faster)
    let validator = jsonschema::validator_for(&schema)?;

    // Fail on first error
    assert!(validator.validate(&instance).is_ok());

    // Iterate over errors
    for error in validator.iter_errors(&instance) {
        eprintln!("Error: {error}");
        eprintln!("Location: {}", error.instance_path());
    }

    // Boolean result
    assert!(validator.is_valid(&instance));

    // Structured output (JSON Schema Output v1)
    let evaluation = validator.evaluate(&instance);
    for annotation in evaluation.iter_annotations() {
        eprintln!(
            "Annotation at {}: {:?}",
            annotation.schema_location,
            annotation.annotations.value()
        );
    }

    Ok(())
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::fs::File;
    use std::io::BufReader;

    #[test]
    fn validate_schema_works() -> Result<(), Box<dyn std::error::Error>> {
        // for testing we are using service.schema.json file in testing/ dir for now
        // attempt to load from file.
        // For now use hardcoded json below
        let file = File::open("service.schema.json")?;
        let reader = BufReader::new(file);
        let json_schema: serde_json::Value = serde_json::from_reader(reader)?;

        // hardocded -- should pass
        let test_control_json = r#"{
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
        schema_validated_filecontents(&json_schema, &test_control_json)
    }

    #[test]
    fn example_works() -> Result<(), Box<dyn std::error::Error>> {
        example()
    }
}
