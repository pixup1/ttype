pub fn nth_char_idx(text: &str, idx: usize) -> usize {
	text
		.char_indices()
		.nth(idx)
		.expect("Invalid cursor position")
		.0
}