pub fn get_utf_index(line: &str, idx: usize) -> usize {
    line.char_indices()
        .nth(idx)
        .map(|(i, _)| i)
        .unwrap_or(line.len())
}

pub trait VimCharExt {
    fn is_vim_punctuation(&self) -> bool;
}

impl VimCharExt for char {
    fn is_vim_punctuation(&self) -> bool {
        self.is_ascii_punctuation()
    }
}
