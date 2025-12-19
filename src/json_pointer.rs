use tower_lsp::lsp_types::{Position, Range};
use tracing::{debug, instrument, trace, warn};

use crate::{line_number, pointer_index};

/// Converts Json Pointer to start Position, end Position
/// Takes a &str JsonPointer and the original raw_file_contents,
/// outputs None on no find, match on something.
#[instrument(skip(raw_file_contents), fields(
    pointer = json_pointer,
    content_len = raw_file_contents.len()
))]
pub fn into_range(json_pointer: &str, raw_file_contents: &str) -> Option<Range> {
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

    let index_summation = pointer_index::calculate(json_pointer, raw_file_contents);

    debug!(
        pointer = json_pointer,
        resolved_index = index_summation,
        "Calculated index for JSON pointer"
    );

    // count byte occurences of newline char for the line position.
    let line_number = line_number::from_index(raw_file_contents, index_summation);

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
