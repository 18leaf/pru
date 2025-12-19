use tower_lsp::lsp_types::Range;
use tracing::{debug, instrument, trace, warn};

use crate::json_pointer;

/// Resolves the range for a diagnostic from a JSON pointer
#[instrument(skip(file_contents), fields(pointer = json_pointer))]
pub fn from_pointer(json_pointer: &str, file_contents: &str) -> Range {
    match json_pointer::into_range(json_pointer, file_contents) {
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
