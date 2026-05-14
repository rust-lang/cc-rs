//! POSIX-shell-style word splitter for `*FLAGS` environment variables.
//!
//! Adapted from the [`shlex`](https://crates.io/crates/shlex/1.3.0) crate
//! (1.3.0), iterator path only.
//! Copyright 2015 Nicholas Allegra (comex). MIT or Apache-2.0.

/// An iterator that takes an input string and splits it into the words using the same syntax as
/// the POSIX shell.
pub struct Shlex<'a> {
    in_iter: core::slice::Iter<'a, u8>,
    /// The number of newlines read so far, plus one.
    pub line_no: usize,
    /// An input string is erroneous if it ends while inside a quotation or right after an
    /// unescaped backslash.  Since Iterator does not have a mechanism to return an error, if that
    /// happens, Shlex just throws out the last token, ends the iteration, and sets 'had_error' to
    /// true; best to check it after you're done iterating.
    pub had_error: bool,
}

impl<'a> Shlex<'a> {
    pub fn new(in_str: &'a str) -> Self {
        Shlex {
            in_iter: in_str.as_bytes().iter(),
            line_no: 1,
            had_error: false,
        }
    }

    fn parse_word(&mut self, mut ch: u8) -> Option<Vec<u8>> {
        let mut result: Vec<u8> = Vec::new();
        loop {
            match ch as char {
                '"' => if let Err(()) = self.parse_double(&mut result) {
                    self.had_error = true;
                    return None;
                },
                '\'' => if let Err(()) = self.parse_single(&mut result) {
                    self.had_error = true;
                    return None;
                },
                '\\' => if let Some(ch2) = self.next_char() {
                    if ch2 != '\n' as u8 { result.push(ch2); }
                } else {
                    self.had_error = true;
                    return None;
                },
                ' ' | '\t' | '\n' => { break; },
                _ => { result.push(ch as u8); },
            }
            if let Some(ch2) = self.next_char() { ch = ch2; } else { break; }
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
                                // \$ => $
                                '$' | '`' | '"' | '\\' => { result.push(ch3); },
                                // \<newline> => nothing
                                '\n' => {},
                                // \x => =x
                                _ => { result.push('\\' as u8); result.push(ch3); }
                            }
                        } else {
                            return Err(());
                        }
                    },
                    '"' => { return Ok(()); },
                    _ => { result.push(ch2); },
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
                    '\'' => { return Ok(()); },
                    _ => { result.push(ch2); },
                }
            } else {
                return Err(());
            }
        }
    }

    fn next_char(&mut self) -> Option<u8> {
        let res = self.in_iter.next().copied();
        if res == Some(b'\n') { self.line_no += 1; }
        res
    }
}

impl<'a> Iterator for Shlex<'a> {
    type Item = String;
    fn next(&mut self) -> Option<String> {
        if let Some(mut ch) = self.next_char() {
            // skip initial whitespace
            loop {
                match ch as char {
                    ' ' | '\t' | '\n' => {},
                    '#' => {
                        while let Some(ch2) = self.next_char() {
                            if ch2 as char == '\n' { break; }
                        }
                    },
                    _ => { break; }
                }
                if let Some(ch2) = self.next_char() { ch = ch2; } else { return None; }
            }
            self.parse_word(ch).map(|byte_word| {
                // Safety: input is &str (valid UTF-8) and parse_word only treats ASCII bytes
                // specially, so the resulting Vec<u8> is also valid UTF-8.
                unsafe { String::from_utf8_unchecked(byte_word) }
            })
        } else { // no initial character
            None
        }
    }
}

#[cfg(test)]
pub fn split(in_str: &str) -> Shlex<'_> {
    Shlex::new(in_str)
}

#[cfg(test)]
static SPLIT_TEST_ITEMS: &'static [(&'static str, Option<&'static [&'static str]>)] = &[
    ("foo$baz", Some(&["foo$baz"])),
    ("foo baz", Some(&["foo", "baz"])),
    ("foo\"bar\"baz", Some(&["foobarbaz"])),
    ("foo \"bar\"baz", Some(&["foo", "barbaz"])),
    ("   foo \nbar", Some(&["foo", "bar"])),
    ("foo\\\nbar", Some(&["foobar"])),
    ("\"foo\\\nbar\"", Some(&["foobar"])),
    ("'baz\\$b'", Some(&["baz\\$b"])),
    ("'baz\\\''", None),
    ("\\", None),
    ("\"\\", None),
    ("'\\", None),
    ("\"", None),
    ("'", None),
    ("foo #bar\nbaz", Some(&["foo", "baz"])),
    ("foo #bar", Some(&["foo"])),
    ("foo#bar", Some(&["foo#bar"])),
    ("foo\"#bar", None),
    ("'\\n'", Some(&["\\n"])),
    ("'\\\\n'", Some(&["\\\\n"])),
];

#[cfg(test)]
mod tests {
    use super::{Shlex, SPLIT_TEST_ITEMS};

    fn split(s: &str) -> Vec<String> {
        super::split(s).collect()
    }

    #[test]
    fn test_split() {
        for &(input, output) in SPLIT_TEST_ITEMS {
            let mut sh = Shlex::new(input);
            let res: Vec<String> = sh.by_ref().collect();
            match output {
                Some(expected) => {
                    assert!(!sh.had_error, "input {:?}: unexpected error", input);
                    assert_eq!(
                        res,
                        expected.iter().map(|&x| x.to_owned()).collect::<Vec<_>>(),
                        "input {:?}",
                        input,
                    );
                }
                None => {
                    assert!(sh.had_error, "input {:?}: expected error", input);
                }
            }
        }
    }

    #[test]
    fn test_lineno() {
        let mut sh = Shlex::new("\nfoo\nbar");
        while let Some(word) = sh.next() {
            if word == "bar" {
                assert_eq!(sh.line_no, 3);
            }
        }
    }

    #[test]
    fn whitespace_separated() {
        assert_eq!(split("foo bar baz"), vec!["foo", "bar", "baz"]);
        assert_eq!(split("  foo\tbar\n baz "), vec!["foo", "bar", "baz"]);
        assert_eq!(split(""), Vec::<String>::new());
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
