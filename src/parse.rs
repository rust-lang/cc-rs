//! POSIX-shell-style word splitter for `*FLAGS` environment variables.
//!
//! Adapted from the [`shlex`](https://crates.io/crates/shlex/1.3.0) crate
//! (1.3.0), iterator path only.
//! Copyright 2015 Nicholas Allegra (comex). MIT or Apache-2.0.

/// An iterator that takes an input string and splits it into the words using the same syntax as
/// the POSIX shell.
///
/// If the input ends while inside a quotation or right after an unescaped backslash, the
/// in-progress token is dropped and iteration ends.
pub struct Shlex<'a> {
    in_iter: core::slice::Iter<'a, u8>,
}

impl<'a> Shlex<'a> {
    pub fn new(in_str: &'a str) -> Self {
        Shlex {
            in_iter: in_str.as_bytes().iter(),
        }
    }

    fn parse_word(&mut self, mut ch: u8) -> Option<Vec<u8>> {
        let mut result: Vec<u8> = Vec::new();
        loop {
            match ch as char {
                '"' => {
                    if let Err(()) = self.parse_double(&mut result) {
                        return None;
                    }
                }
                '\'' => {
                    if let Err(()) = self.parse_single(&mut result) {
                        return None;
                    }
                }
                '\\' => {
                    if let Some(ch2) = self.next_char() {
                        // `\<newline>` is a line continuation.
                        if ch2 != b'\n' {
                            result.push(ch2);
                        }
                    } else {
                        return None;
                    }
                }
                ' ' | '\t' | '\n' => {
                    break;
                }
                _ => {
                    result.push(ch);
                }
            }
            if let Some(ch2) = self.next_char() {
                ch = ch2;
            } else {
                break;
            }
        }
        Some(result)
    }

    fn parse_double(&mut self, result: &mut Vec<u8>) -> Result<(), ()> {
        loop {
            if let Some(ch2) = self.next_char() {
                match ch2 as char {
                    '\\' => {
                        if let Some(ch3) = self.next_char() {
                            match ch3 as char {
                                // `\$`, `` \` ``, `\"`, `\\` escape the next character.
                                '$' | '`' | '"' | '\\' => {
                                    result.push(ch3);
                                }
                                // `\<newline>` is a line continuation.
                                '\n' => {}
                                // Any other escape preserves the backslash.
                                _ => {
                                    result.push(b'\\');
                                    result.push(ch3);
                                }
                            }
                        } else {
                            return Err(());
                        }
                    }
                    '"' => {
                        return Ok(());
                    }
                    _ => {
                        result.push(ch2);
                    }
                }
            } else {
                return Err(());
            }
        }
    }

    fn parse_single(&mut self, result: &mut Vec<u8>) -> Result<(), ()> {
        loop {
            if let Some(ch2) = self.next_char() {
                match ch2 as char {
                    '\'' => {
                        return Ok(());
                    }
                    _ => {
                        result.push(ch2);
                    }
                }
            } else {
                return Err(());
            }
        }
    }

    fn next_char(&mut self) -> Option<u8> {
        self.in_iter.next().copied()
    }
}

impl<'a> Iterator for Shlex<'a> {
    type Item = String;
    fn next(&mut self) -> Option<String> {
        if let Some(mut ch) = self.next_char() {
            // Skip leading whitespace and line comments.
            loop {
                match ch as char {
                    ' ' | '\t' | '\n' => {}
                    '#' => {
                        while let Some(ch2) = self.next_char() {
                            if ch2 as char == '\n' {
                                break;
                            }
                        }
                    }
                    _ => {
                        break;
                    }
                }
                if let Some(ch2) = self.next_char() {
                    ch = ch2;
                } else {
                    return None;
                }
            }
            self.parse_word(ch).map(|byte_word| {
                // Safety: input is &str (valid UTF-8) and parse_word only treats ASCII bytes
                // specially, so the resulting Vec<u8> is also valid UTF-8.
                unsafe { String::from_utf8_unchecked(byte_word) }
            })
        } else {
            // no initial character
            None
        }
    }
}

#[cfg(test)]
pub fn split(in_str: &str) -> Shlex<'_> {
    Shlex::new(in_str)
}

/// Test corpus from upstream shlex 1.3.0. Inputs that upstream marked as erroneous
/// (`Option::None`) all produce no tokens here, because the in-progress token is dropped
/// when the parser hits an unterminated quote or trailing backslash.
#[cfg(test)]
static SPLIT_TEST_ITEMS: &'static [(&'static str, &'static [&'static str])] = &[
    ("foo$baz", &["foo$baz"]),
    ("foo baz", &["foo", "baz"]),
    ("foo\"bar\"baz", &["foobarbaz"]),
    ("foo \"bar\"baz", &["foo", "barbaz"]),
    ("   foo \nbar", &["foo", "bar"]),
    ("foo\\\nbar", &["foobar"]),
    ("\"foo\\\nbar\"", &["foobar"]),
    ("'baz\\$b'", &["baz\\$b"]),
    ("'baz\\\''", &[]),
    ("\\", &[]),
    ("\"\\", &[]),
    ("'\\", &[]),
    ("\"", &[]),
    ("'", &[]),
    ("foo #bar\nbaz", &["foo", "baz"]),
    ("foo #bar", &["foo"]),
    ("foo#bar", &["foo#bar"]),
    ("foo\"#bar", &[]),
    ("'\\n'", &["\\n"]),
    ("'\\\\n'", &["\\\\n"]),
];

#[cfg(test)]
mod tests {
    use super::SPLIT_TEST_ITEMS;

    fn split(s: &str) -> Vec<String> {
        super::split(s).collect()
    }

    #[test]
    fn test_split() {
        for &(input, expected) in SPLIT_TEST_ITEMS {
            assert_eq!(
                split(input),
                expected.iter().map(|&x| x.to_owned()).collect::<Vec<_>>(),
                "input {:?}",
                input,
            );
        }
    }

    #[test]
    fn whitespace_separated() {
        assert_eq!(split("foo bar baz"), vec!["foo", "bar", "baz"]);
        assert_eq!(split("  foo\tbar\n baz "), vec!["foo", "bar", "baz"]);
        assert_eq!(split(""), Vec::<String>::new());
    }

    #[test]
    fn newline_separates_and_skips_leading() {
        // Replaces upstream `test_lineno`, which used the same input ("\nfoo\nbar") to verify
        // that `sh.line_no == 3` after consuming the two `\n` bytes. We removed the `line_no`
        // counter as dead code, so we can no longer assert the count itself; instead we
        // assert the externally-observable consequence of those `\n` bytes: the leading one
        // is skipped and the middle one acts as a word separator, yielding ["foo", "bar"].
        assert_eq!(split("\nfoo\nbar"), vec!["foo", "bar"]);
    }

    #[test]
    fn double_quoted_preserves_spaces() {
        assert_eq!(split(r#"foo "bar baz""#), vec!["foo", "bar baz"]);
        assert_eq!(
            split(r#"-DFOO="bar baz" -I/x"#),
            vec!["-DFOO=bar baz", "-I/x"]
        );
    }

    #[test]
    fn single_quoted_is_literal() {
        assert_eq!(split(r#"'a \b $c \"'"#), vec![r#"a \b $c \""#]);
    }

    #[test]
    fn double_quote_escapes() {
        assert_eq!(split(r#""\$\`\"\\""#), vec![r#"$`"\"#]);
        assert_eq!(split(r#""\n""#), vec![r"\n"]);
    }

    #[test]
    fn backslash_outside_quotes_escapes_next_char() {
        assert_eq!(split(r"foo\ bar"), vec!["foo bar"]);
        assert_eq!(split(r"a\\b"), vec![r"a\b"]);
    }

    #[test]
    fn comment_starts_at_word_boundary() {
        assert_eq!(split("foo # this is a comment\nbar"), vec!["foo", "bar"]);
    }

    #[test]
    fn unclosed_quote_terminates_and_drops_token() {
        assert_eq!(split(r#"foo "unterminated"#), vec!["foo"]);
        assert_eq!(split(r"foo 'unterminated"), vec!["foo"]);
    }

    #[test]
    fn trailing_backslash_drops_token() {
        assert_eq!(split(r"foo bar\"), vec!["foo"]);
    }

    #[test]
    fn adjacent_quoted_segments_join_into_one_word() {
        assert_eq!(split(r#""foo"'bar'baz"#), vec!["foobarbaz"]);
    }

    #[test]
    fn utf8_input_passes_through() {
        assert_eq!(split("café \"naïve\""), vec!["café", "naïve"]);
    }
}
