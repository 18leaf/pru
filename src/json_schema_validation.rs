use serde_json::json;

fn validate_from_file(
    file_in_path: &str,
    json_schema_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
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

    #[test]
    fn validate_schema_works() -> Result<(), Box<dyn std::error::Error>> {
        // load file

        Ok(())
    }

    #[test]
    fn example_works() -> Result<(), Box<dyn std::error::Error>> {
        example()
    }
}
