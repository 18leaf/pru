use tracing::{debug, instrument, trace, warn};

#[instrument(skip(raw_file_contents), fields(pointer = json_pointer))]
pub(crate) fn calculate(json_pointer: &str, raw_file_contents: &str) -> usize {
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
