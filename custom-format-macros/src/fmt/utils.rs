//! Some useful types.

use std::str::Chars;

/// A `StrCursor` contains an iterator over the [char]s of a string slice.
#[derive(Debug, Clone)]
pub struct StrCursor<'a> {
    /// Iterator of chars representing the remaining data to be read
    chars: Chars<'a>,
}

impl<'a> StrCursor<'a> {
    /// Construct a new `StrCursor` from remaining data
    pub fn new(input: &'a str) -> Self {
        Self { chars: input.chars() }
    }

    /// Returns remaining data
    pub fn remaining(&self) -> &'a str {
        self.chars.as_str()
    }

    /// Returns the next char
    pub fn next(&mut self) -> Option<char> {
        self.chars.next()
    }

    /// Read chars as long as the provided predicate is true
    pub fn read_while<F: Fn(char) -> bool>(&mut self, f: F) -> &'a str {
        let remaining = self.chars.as_str();

        loop {
            let old_chars = self.chars.clone();

            match self.chars.next() {
                None => return remaining,
                Some(c) => {
                    if !f(c) {
                        self.chars = old_chars;
                        return &remaining[..remaining.len() - self.chars.as_str().len()];
                    }
                }
            }
        }
    }

    /// Read chars until the provided predicate is true
    pub fn read_until<F: Fn(char) -> bool>(&mut self, f: F) -> &'a str {
        self.read_while(|x| !f(x))
    }

    /// Read chars until and including the first char for which the provided predicate is true
    pub fn read_until_included<F: Fn(char) -> bool>(&mut self, f: F) -> &'a str {
        let remaining = self.chars.as_str();
        self.chars.position(f);
        &remaining[..remaining.len() - self.chars.as_str().len()]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_remaining() {
        let mut cursor = StrCursor::new("©⓪ßéèç0€");
        assert_eq!(cursor.next(), Some('©'));
        assert_eq!(cursor.remaining(), "⓪ßéèç0€");
    }

    #[test]
    fn test_read_while() {
        let mut cursor = StrCursor::new("©⓪ßéèç0€");
        assert_eq!(cursor.read_while(|c| c != 'ß'), "©⓪");
        assert_eq!(cursor.read_while(|c| c != 'ç'), "ßéè");
        assert_eq!(cursor.read_while(|c| c != ' '), "ç0€");
    }

    #[test]
    fn test_read_until() {
        let mut cursor = StrCursor::new("©⓪ßéèç0€");
        assert_eq!(cursor.read_until(|c| c == 'ß'), "©⓪");
        assert_eq!(cursor.read_until(|c| c == 'ç'), "ßéè");
        assert_eq!(cursor.read_until(|c| c == ' '), "ç0€");
    }

    #[test]
    fn test_read_until_included() {
        let mut cursor = StrCursor::new("©⓪ßéèç0€");
        assert_eq!(cursor.read_until_included(|c| c == 'ß'), "©⓪ß");
        assert_eq!(cursor.read_until_included(|c| c == 'ç'), "éèç");
        assert_eq!(cursor.read_until_included(|c| c == ' '), "0€");
    }
}
