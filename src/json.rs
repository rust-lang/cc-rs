// Vendored from `smoljson` bef592c5da1c3fe38b2462a8d231b0e0c8a86f80, with explicit permission
// from the author (`thomcc`). Minimized for cc/simplicity. Modifications and additions made to fit cc's needs.
#![allow(dead_code)]

use std::borrow::Cow;
/// First lifetime is for strings borrowed from the source.
/// Second lifetime is for strings borrowed from the parser.
#[derive(PartialEq, Debug, Clone)]
pub(crate) enum Token<'s> {
    Null,
    Bool(bool),
    NumU(u64),
    NumI(i64),
    NumF(f64),
    StrBorrow(&'s str),
    StrOwn(Box<str>),
    Colon,
    Comma,
    ObjectBegin,
    ObjectEnd,
    ArrayBegin,
    ArrayEnd,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Error(());

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("JSON parse error")
    }
}

pub type Result<T, E = Error> = core::result::Result<T, E>;

pub(crate) struct Reader<'a> {
    input: &'a str,
    bytes: &'a [u8],
    tok_start: usize,
    pos: usize,
    buf: String,
    stash: Option<Token<'a>>,
}

impl<'a> Reader<'a> {
    /// Create a reader which uses the [default `Dialect`](Dialect::DEFAULT).
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            bytes: input.as_bytes(),
            pos: 0,
            buf: String::new(),
            tok_start: 0,
            stash: None,
        }
    }

    #[inline]
    pub fn position(&self) -> usize {
        self.pos.min(self.bytes.len())
    }

    #[cold]
    pub(super) fn err(&self) -> Error {
        Error(())
    }

    /// Returns `Err` if there are any more non-whitespace/non-comment (if this
    /// reader's dialect allows comments) characters in the input.
    pub fn finish(mut self) -> Result<()> {
        match self.next_token() {
            Ok(Some(_)) => Err(self.err()),
            Ok(None) => Ok(()),
            Err(e) => Err(e),
        }
    }

    fn bnext_if(&mut self, b: u8) -> bool {
        if self.pos < self.bytes.len() && self.bytes[self.pos] == b {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn bnext(&mut self) -> Option<u8> {
        if self.pos < self.bytes.len() {
            let ch = self.bytes[self.pos];
            self.pos += 1;
            Some(ch)
        } else {
            None
        }
    }

    fn bnext_or_err(&mut self) -> Result<u8> {
        match self.bnext() {
            Some(c) => Ok(c),
            None => Err(self.err()),
        }
    }

    fn bpeek(&mut self) -> Option<u8> {
        if self.pos < self.bytes.len() {
            Some(self.bytes[self.pos])
        } else {
            None
        }
    }

    fn bpeek_or_nul(&mut self) -> u8 {
        self.bpeek().unwrap_or(b'\0')
    }

    fn bump(&mut self) {
        self.pos += 1;
        debug_assert!(self.pos <= self.input.len());
    }

    fn finished(&self) -> bool {
        self.pos >= self.bytes.len()
    }

    pub(super) fn ref_stash(&self) -> Option<&Token<'a>> {
        self.stash.as_ref()
    }

    pub(super) fn mut_stash(&mut self) -> &mut Option<Token<'a>> {
        &mut self.stash
    }
    pub(super) fn take_stash(&mut self) -> Option<Token<'a>> {
        self.stash.take()
    }

    pub(super) fn skipnpeek(&mut self) -> Result<Option<u8>> {
        debug_assert!(self.stash.is_none());
        self.skip_trivial()?;
        Ok(self.bpeek())
    }

    fn skip_trivial(&mut self) -> Result<()> {
        loop {
            self.skip_ws_only();
            if !self.bnext_if(b'/') {
                return Ok(());
            }
            match self.bnext() {
                Some(b'*') => self.skip_block_comment()?,
                Some(b'/') => self.skip_line_comment(),
                _ => return Err(self.err()),
            }
        }
    }

    fn skip_line_comment(&mut self) {
        let (mut p, bs) = (self.pos, self.bytes);
        while p < bs.len() && bs[p] != b'\n' {
            p += 1;
        }
        self.pos = p;
    }

    fn skip_block_comment(&mut self) -> Result<()> {
        let (mut p, bs) = (self.pos, self.bytes);
        loop {
            if p + 1 >= bs.len() {
                self.pos = p;
                return Err(self.err());
            }
            if bs[p] == b'*' && bs[p + 1] == b'/' {
                self.pos = p + 2;
                return Ok(());
            }
            p += 1;
        }
    }

    fn skip_ws_only(&mut self) {
        let (mut p, bs) = (self.pos, self.bytes);
        while p < bs.len() && matches!(bs[p], b'\n' | b' ' | b'\t' | b'\r') {
            p += 1;
        }
        self.pos = p;
    }

    fn cur_ch(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    fn single_hex_escape(&mut self) -> Result<u16> {
        let mut acc = 0;
        for _ in 0..4 {
            let b = self.bnext_or_err()?;
            let n = match b {
                b'0'..=b'9' => b - b'0',
                b'a'..=b'f' => b - b'a' + 10,
                b'A'..=b'F' => b - b'A' + 10,
                _ => return Err(self.err()),
            };
            acc = acc * 16 + (n as u16);
        }
        Ok(acc)
    }

    fn read_hex_escape(&mut self) -> Result<()> {
        // todo: option where we reutrn an error (instead using replacement
        // char) if unescaping produces unpaired surrogates.
        use core::char::REPLACEMENT_CHARACTER as REPLACEMENT;
        const LEAD: core::ops::Range<u16> = 0xd800..0xdc00;
        const TRAIL: core::ops::Range<u16> = 0xdc00..0xe000;

        let lead = self.single_hex_escape()?;
        if let Some(c) = core::char::from_u32(lead as u32) {
            self.buf.push(c);
            return Ok(());
        }
        if TRAIL.contains(&lead) {
            self.buf.push(REPLACEMENT);
            return Ok(());
        }
        debug_assert!(LEAD.contains(&lead));
        let p = self.pos;
        let trail = if self.bytes[p..].starts_with(b"\\u") {
            self.pos += 2;
            self.single_hex_escape()?
        } else {
            self.buf.push(REPLACEMENT);
            return Ok(());
        };
        if !TRAIL.contains(&trail) {
            // rewind here so we follow algorithm 2 (max subparts of illegal
            // sequence) for https://www.unicode.org/review/pr-121.html.
            self.pos = p;
            self.buf.push(REPLACEMENT);
            return Ok(());
        }
        let scalar = (((lead as u32 - 0xd800) << 10) | (trail as u32 - 0xdc00)) + 0x10000;
        debug_assert!(
            core::char::from_u32(scalar).is_some(),
            r#""\u{:04x}\u{:04x}" => {:#x}"#,
            lead,
            trail,
            scalar,
        );
        // all well-formed surrogate pairs map to `char`s (e.g. unicode scalar
        // values), so unwrap is fine
        self.buf.push(core::char::from_u32(scalar).unwrap());
        Ok(())
    }

    fn expect_next(&mut self, next: &[u8]) -> Result<()> {
        for &i in next {
            if Some(i) != self.bnext() {
                return Err(self.err());
            }
        }
        Ok(())
    }

    fn unescape_next(&mut self) -> Result<()> {
        let b = self.bnext_or_err()?;
        match b {
            b'b' => self.buf.push('\x08'),
            b'f' => self.buf.push('\x0c'),
            b'n' => self.buf.push('\n'),
            b'r' => self.buf.push('\r'),
            b't' => self.buf.push('\t'),
            b'\\' => self.buf.push('\\'),
            b'/' => self.buf.push('/'),
            b'\"' => self.buf.push('\"'),
            b'u' => return self.read_hex_escape(),
            _ => return Err(self.err()),
        }
        Ok(())
    }

    fn read_keyword(&mut self, id: &[u8], t: Token<'a>) -> Result<Token<'a>> {
        debug_assert_eq!(self.bytes[self.pos - 1], id[0]);
        self.expect_next(&id[1..])?;
        Ok(t)
    }

    pub(crate) fn unpeek(&mut self, t: Token<'a>) {
        assert!(self.stash.is_none());
        self.stash = Some(t);
    }
    pub(crate) fn next_token(&mut self) -> Result<Option<Token<'a>>> {
        if let Some(t) = self.stash.take() {
            return Ok(Some(t));
        }
        self.skip_trivial()?;
        if self.pos >= self.input.len() {
            return Ok(None);
        }
        self.tok_start = self.pos;
        let tok = match self.bnext_or_err()? {
            b':' => return Ok(Some(Token::Colon)),
            b',' => return Ok(Some(Token::Comma)),
            b'{' => return Ok(Some(Token::ObjectBegin)),
            b'}' => return Ok(Some(Token::ObjectEnd)),
            b'[' => return Ok(Some(Token::ArrayBegin)),
            b']' => return Ok(Some(Token::ArrayEnd)),
            b'"' => self.read_string(),
            b't' => self.read_keyword(b"true", Token::Bool(true)),
            b'f' => self.read_keyword(b"false", Token::Bool(false)),
            b'n' => self.read_keyword(b"null", Token::Null),
            b'-' | b'0'..=b'9' => self.read_num(),
            _ => return Err(self.err()),
        };
        Ok(Some(tok?))
    }

    fn is_delim_byte(&self, b: u8) -> bool {
        matches!(b, b',' | b'}' | b']' | b' ' | b'\t' | b'\n' | b'\r')
    }

    fn read_num(&mut self) -> Result<Token<'a>> {
        let neg = self.bytes[self.tok_start] == b'-';
        let mut float = false;
        while let Some(b) = self.bpeek() {
            match b {
                b'.' | b'e' | b'E' | b'+' | b'-' => {
                    float = true;
                    self.bump();
                }
                b'0'..=b'9' => {
                    self.bump();
                }
                b if self.is_delim_byte(b) => break,
                _ => return Err(self.err()),
            }
        }
        let text = &self.input[self.tok_start..self.pos];
        if !float {
            if neg {
                if let Ok(i) = text.parse::<i64>() {
                    debug_assert!(i < 0);
                    return Ok(Token::NumI(i));
                }
            } else if let Ok(u) = text.parse::<u64>() {
                return Ok(Token::NumU(u));
            }
        }
        if let Ok(v) = text.parse::<f64>() {
            Ok(Token::NumF(v))
        } else {
            Err(self.err())
        }
    }

    fn read_string(&mut self) -> Result<Token<'a>> {
        self.buf.clear();
        let bs = self.bytes;
        loop {
            let mut p = self.pos;
            let start = p;
            while p < bs.len() && bs[p] != b'"' && bs[p] != b'\\' {
                p += 1;
            }
            if p == bs.len() || !self.input.is_char_boundary(p) {
                self.pos = p;
                return Err(self.err());
            }
            self.pos = p + 1;
            if bs[p] == b'"' && self.buf.is_empty() {
                // didn't need any unescaping.
                return Ok(Token::StrBorrow(&self.input[start..p]));
            }
            self.buf.push_str(&self.input[start..p]);
            if bs[p] == b'"' {
                return Ok(Token::StrOwn(self.buf.clone().into_boxed_str()));
            }
            debug_assert_eq!(bs[p], b'\\');
            self.unescape_next()?
        }
    }
}

macro_rules! tok_tester {
    ($($func:ident matches $tok:ident);*) => {$(
        pub(crate) fn $func(&mut self) -> Result<()> {
            match self.next_token() {
                Ok(Some(Token::$tok)) => Ok(()),
                Err(e) => Err(e),
                _ => Err(self.err()),
            }
        }
    )*};
}
impl<'a> Reader<'a> {
    pub(crate) fn next(&mut self) -> Result<Token<'a>> {
        match self.next_token() {
            Ok(Some(v)) => Ok(v),
            Err(e) => Err(e),
            _ => Err(self.err()),
        }
    }
    tok_tester! {
        array_begin matches ArrayBegin;
        // array_end matches ArrayEnd;
        obj_begin matches ObjectBegin;
        // obj_end matches ObjectEnd;
        comma matches Comma;
        colon matches Colon;
        null matches Null
    }
    pub(crate) fn comma_or_obj_end(&mut self) -> Result<bool> {
        match self.next_token() {
            Ok(Some(Token::Comma)) => Ok(true),
            Ok(Some(Token::ObjectEnd)) => Ok(false),
            Err(e) => Err(e),
            _ => Err(self.err()),
        }
    }
    pub(crate) fn comma_or_array_end(&mut self) -> Result<bool> {
        match self.next_token() {
            Ok(Some(Token::Comma)) => Ok(true),
            Ok(Some(Token::ArrayEnd)) => Ok(false),
            Err(e) => Err(e),
            _ => Err(self.err()),
        }
    }
    pub(crate) fn key(&mut self) -> Result<Cow<'a, str>> {
        match self.next_token() {
            Ok(Some(Token::StrBorrow(b))) => Ok(Cow::Borrowed(b)),
            Ok(Some(Token::StrOwn(b))) => Ok(Cow::Owned(b.into())),
            Err(e) => Err(e),
            Ok(Some(_t)) => Err(self.err()),
            _o => Err(self.err()),
        }
    }
}

impl<'a> Reader<'a> {
    fn read_str(&mut self) -> Result<Option<Cow<'a, str>>> {
        match self.next_token() {
            Ok(Some(Token::StrBorrow(s))) => Ok(Some(Cow::Borrowed(s))),
            Ok(Some(Token::StrOwn(s))) => Ok(Some(Cow::Owned(s.into()))),
            Ok(Some(_)) => Ok(None),
            Ok(None) => Err(self.err()),
            Err(e) => Err(e),
        }
    }

    pub fn read_str_from_object(
        &mut self,
        key: &str,
        parent_object: Option<&str>,
    ) -> Result<Cow<'a, str>> {
        let inside_nested_object = parent_object.is_some();
        if let Some(parent_name) = parent_object {
            // If the field we want is inside a nested object, skip into that object
            loop {
                match self.read_str()? {
                    Some(value) => {
                        if value == Cow::Borrowed(parent_name) {
                            if self.next()? != Token::Colon {
                                return Err(self.err());
                            }
                            if self.next()? != Token::ObjectBegin {
                                return Err(self.err());
                            }
                            break;
                        }
                    }
                    None => continue,
                }
            }
        }

        let mut nesting = false;
        loop {
            let value = match self.next()? {
                Token::StrBorrow(s) => Cow::Borrowed(s),
                Token::StrOwn(s) => Cow::Owned(s.into()),
                Token::ObjectBegin => {
                    nesting = true;
                    continue;
                }
                Token::ObjectEnd => {
                    if nesting {
                        // Exit nested object, we know its `ObjectEnd` isn't for the seeked one..
                        nesting = false;
                    } else if inside_nested_object || self.skipnpeek() == Ok(None) {
                        // Finding the end of the current object without finding a matching key is an error
                        // when a specific scope is provided.
                        // If this `ObjectEnd` was the last in the file, error too.
                        return Err(self.err());
                    }
                    continue;
                }
                _ => continue,
            };

            if value != key {
                continue;
            }

            // If the parser is inside a nested object but the caller wanted something in the parent
            // structure, don't read anything out of this object.
            if nesting && !inside_nested_object {
                continue;
            }

            if self.next()? != Token::Colon {
                return Err(self.err());
            }

            return match self.read_str() {
                Ok(Some(val)) => Ok(val),
                Ok(None) => Err(self.err()),
                Err(e) => Err(e),
            };
        }
    }
}
