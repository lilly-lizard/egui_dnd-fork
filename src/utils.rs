/// Removes value at `source_idx` and places it at `target_idx`. Does nothing if `source_idx` is
/// equal to `target_idx` or if either index is outside the vec bounds.
pub fn shift_vec<T>(source_idx: usize, mut target_idx: usize, vec: &mut Vec<T>) {
    if source_idx == target_idx || source_idx >= vec.len() || target_idx > vec.len() {
        return;
    }

    if source_idx < target_idx {
        target_idx -= 1
    };

    let item = vec.remove(source_idx);
    vec.insert(target_idx, item);
}
