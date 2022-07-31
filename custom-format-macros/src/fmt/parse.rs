//! Functions used for parsing standard format specifier.

use super::utils::StrCursor;
use super::{ArgKind, Count, Error, Id, Precision};

/// Process standard fill and alignment specifiers
pub(super) fn process_align(cursor: &mut StrCursor) -> [Option<char>; 2] {
    let cursor0 = cursor.clone();
    let c1 = cursor.next();
    let cursor1 = cursor.clone();
    let c2 = cursor.next();

    if c1.is_some() && matches!(c2, Some('<') | Some('^') | Some('>')) {
        [c1, c2]
    } else if matches!(c1, Some('<') | Some('^') | Some('>')) {
        *cursor = cursor1;
        [c1, None]
    } else {
        *cursor = cursor0;
        [None, None]
    }
}

/// Process standard sign specifier
pub(super) fn process_sign(cursor: &mut StrCursor) -> Option<char> {
    let old_cursor = cursor.clone();

    match cursor.next() {
        sign @ Some('+') | sign @ Some('-') => sign,
        _ => {
            *cursor = old_cursor;
            None
        }
    }
}

/// Process standard alternate specifier
pub(super) fn process_alternate(cursor: &mut StrCursor) -> Option<char> {
    let old_cursor = cursor.clone();

    match cursor.next() {
        sign @ Some('#') => sign,
        _ => {
            *cursor = old_cursor;
            None
        }
    }
}

/// Process standard sign-aware zero-padding specifier
pub(super) fn process_sign_aware_zero_pad(cursor: &mut StrCursor) -> Option<char> {
    let old_cursor = cursor.clone();
    let c = cursor.next();
    let next = cursor.remaining().bytes().next();

    match (c, next) {
        (sign @ Some('0'), next) if next != Some(b'$') => sign,
        _ => {
            *cursor = old_cursor;
            None
        }
    }
}

/// Process standard width specifier
pub(super) fn process_width<'a>(cursor: &mut StrCursor<'a>) -> Result<Option<Count<'a>>, Error> {
    process_count(cursor)
}

/// Process standard precision specifier
pub(super) fn process_precision<'a>(cursor: &mut StrCursor<'a>) -> Result<Option<Precision<'a>>, Error> {
    let mut old_cursor = cursor.clone();

    if !matches!(cursor.next(), Some('.')) {
        *cursor = old_cursor;
        return Ok(None);
    }

    old_cursor = cursor.clone();

    match cursor.next() {
        Some('*') => Ok(Some(Precision::Asterisk)),
        _ => {
            *cursor = old_cursor;
            match process_count(cursor)? {
                Some(count) => Ok(Some(Precision::WithCount(count))),
                None => Err("invalid count in format string".into()),
            }
        }
    }
}

/// Process standard count specifier
pub(super) fn process_count<'a>(cursor: &mut StrCursor<'a>) -> Result<Option<Count<'a>>, Error> {
    let old_cursor = cursor.clone();

    // Try parsing as argument with '$'
    match parse_argument(cursor)? {
        Some(arg_kind) if cursor.next() == Some('$') => return Ok(Some(Count::Argument(arg_kind))),
        _ => *cursor = old_cursor,
    }

    // Try parsing as integer
    match cursor.read_while(|c| c.is_ascii_digit()) {
        "" => Ok(None),
        integer => Ok(Some(Count::Integer(integer))),
    }
}

/// Parse argument in a format specifier
pub(super) fn parse_argument<'a>(cursor: &mut StrCursor<'a>) -> Result<Option<ArgKind<'a>>, Error> {
    // Try parsing as integer
    let integer_argument = cursor.read_while(|c| c.is_ascii_digit());
    if !integer_argument.is_empty() {
        return Ok(Some(ArgKind::Positional(integer_argument.parse().unwrap())));
    }

    // Try parsing as identifier
    let old_cursor = cursor.clone();
    let remaining = cursor.remaining();

    let first_char = match cursor.next() {
        Some(first_char) => first_char,
        None => return Ok(None),
    };

    let first_char_len = remaining.len() - cursor.remaining().len();

    let identifier = match first_char {
        '_' => match cursor.read_while(unicode_ident::is_xid_continue).len() {
            0 => return Err("invalid argument: argument name cannot be a single underscore".into()),
            len => &remaining[..first_char_len + len],
        },
        c => {
            if unicode_ident::is_xid_start(c) {
                let len = cursor.read_while(unicode_ident::is_xid_continue).len();
                &remaining[..first_char_len + len]
            } else {
                *cursor = old_cursor;
                return Ok(None);
            }
        }
    };

    Ok(Some(ArgKind::Named(Id::new(identifier)?)))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_process_align() {
        let data = [
            ("^--", [Some('^'), None], "--"),
            ("<--", [Some('<'), None], "--"),
            (">--", [Some('>'), None], "--"),
            ("-^-", [Some('-'), Some('^')], "-"),
            ("-<-", [Some('-'), Some('<')], "-"),
            ("->-", [Some('-'), Some('>')], "-"),
            ("--^", [None, None], "--^"),
            ("--<", [None, None], "--<"),
            ("-->", [None, None], "-->"),
        ];

        for &(fmt, output, remaining) in &data {
            let mut cursor = StrCursor::new(fmt);
            assert_eq!(process_align(&mut cursor), output);
            assert_eq!(cursor.remaining(), remaining);
        }
    }

    #[test]
    fn test_process_sign() {
        let data = [("+000", Some('+'), "000"), ("-000", Some('-'), "000"), ("0000", None, "0000")];

        for &(fmt, output, remaining) in &data {
            let mut cursor = StrCursor::new(fmt);
            assert_eq!(process_sign(&mut cursor), output);
            assert_eq!(cursor.remaining(), remaining);
        }
    }

    #[test]
    fn test_process_alternate() {
        let data = [("#0", Some('#'), "0"), ("00", None, "00")];

        for &(fmt, output, remaining) in &data {
            let mut cursor = StrCursor::new(fmt);
            assert_eq!(process_alternate(&mut cursor), output);
            assert_eq!(cursor.remaining(), remaining);
        }
    }

    #[test]
    fn test_process_sign_aware_zero_pad() {
        let data = [("0123", Some('0'), "123"), ("0.6", Some('0'), ".6"), ("123", None, "123"), ("0$", None, "0$")];

        for &(fmt, output, remaining) in &data {
            let mut cursor = StrCursor::new(fmt);
            assert_eq!(process_sign_aware_zero_pad(&mut cursor), output);
            assert_eq!(cursor.remaining(), remaining);
        }
    }

    #[test]
    fn test_parse_argument() -> Result<(), Error> {
        let data = [
            ("05sdkfh-", Some(ArgKind::Positional(5)), "sdkfh-"),
            ("_sdkfh-", Some(ArgKind::Named(Id::new("_sdkfh")?)), "-"),
            ("_é€", Some(ArgKind::Named(Id::new("_é")?)), "€"),
            ("é€", Some(ArgKind::Named(Id::new("é")?)), "€"),
            ("@é€", None, "@é€"),
            ("€", None, "€"),
        ];

        for &(fmt, ref output, remaining) in &data {
            let mut cursor = StrCursor::new(fmt);
            assert_eq!(parse_argument(&mut cursor)?, *output);
            assert_eq!(cursor.remaining(), remaining);
        }

        assert_eq!(&*parse_argument(&mut StrCursor::new("_")).unwrap_err(), "invalid argument: argument name cannot be a single underscore");

        assert_eq!(
            &*parse_argument(&mut StrCursor::new("A\u{30a}")).unwrap_err(),
            r#"identifiers in format string must be normalized in Unicode NFC (`"A\u{30a}"` != `"Å"`)"#
        );

        Ok(())
    }

    #[test]
    fn test_process_width() -> Result<(), Error> {
        let data = [
            ("05sdkfh$-", Some(Count::Integer("05")), "sdkfh$-"),
            ("05$sdkfh-", Some(Count::Argument(ArgKind::Positional(5))), "sdkfh-"),
            ("_sdkfh$-", Some(Count::Argument(ArgKind::Named(Id::new("_sdkfh")?))), "-"),
            ("_é$€", Some(Count::Argument(ArgKind::Named(Id::new("_é")?))), "€"),
            ("é$€", Some(Count::Argument(ArgKind::Named(Id::new("é")?))), "€"),
            ("_sdkfh-$", None, "_sdkfh-$"),
            ("_é€$", None, "_é€$"),
            ("é€$", None, "é€$"),
            ("@é€", None, "@é€"),
            ("€", None, "€"),
        ];

        for &(fmt, ref output, remaining) in &data {
            let mut cursor = StrCursor::new(fmt);
            assert_eq!(process_width(&mut cursor)?, *output);
            assert_eq!(cursor.remaining(), remaining);
        }

        Ok(())
    }

    #[test]
    fn test_process_precision() -> Result<(), Error> {
        let data = [
            (".*--", Some(Precision::Asterisk), "--"),
            (".05sdkfh$-", Some(Precision::WithCount(Count::Integer("05"))), "sdkfh$-"),
            (".05$sdkfh-", Some(Precision::WithCount(Count::Argument(ArgKind::Positional(5)))), "sdkfh-"),
            ("._sdkfh$-", Some(Precision::WithCount(Count::Argument(ArgKind::Named(Id::new("_sdkfh")?)))), "-"),
            ("._é$€", Some(Precision::WithCount(Count::Argument(ArgKind::Named(Id::new("_é")?)))), "€"),
            (".é$€", Some(Precision::WithCount(Count::Argument(ArgKind::Named(Id::new("é")?)))), "€"),
            ("05sdkfh$-", None, "05sdkfh$-"),
            ("05$sdkfh-", None, "05$sdkfh-"),
            ("_sdkfh$-", None, "_sdkfh$-"),
            ("_é$€", None, "_é$€"),
            ("é$€", None, "é$€"),
            ("_sdkfh-$", None, "_sdkfh-$"),
            ("_é€$", None, "_é€$"),
            ("é€$", None, "é€$"),
            ("@é€", None, "@é€"),
            ("€", None, "€"),
        ];

        for &(fmt, ref output, remaining) in &data {
            let mut cursor = StrCursor::new(fmt);
            assert_eq!(process_precision(&mut cursor)?, *output);
            assert_eq!(cursor.remaining(), remaining);
        }

        assert_eq!(process_precision(&mut StrCursor::new("._sdkfh-$")).unwrap_err(), "invalid count in format string");
        assert_eq!(process_precision(&mut StrCursor::new("._é€$")).unwrap_err(), "invalid count in format string");
        assert_eq!(process_precision(&mut StrCursor::new(".é€$")).unwrap_err(), "invalid count in format string");
        assert_eq!(process_precision(&mut StrCursor::new(".@é€")).unwrap_err(), "invalid count in format string");
        assert_eq!(process_precision(&mut StrCursor::new(".€")).unwrap_err(), "invalid count in format string");

        Ok(())
    }
}
