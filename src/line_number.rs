use tracing::{instrument, trace};

#[instrument(skip(raw_file_contents))]
pub(crate) fn from_index(raw_file_contents: &str, index: usize) -> u32 {
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
