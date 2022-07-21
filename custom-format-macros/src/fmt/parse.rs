use super::utils::StrCursor;
use super::{ArgType, Count, Id, Precision};

pub(super) fn process_align(cursor: &mut StrCursor) -> [Option<char>; 2] {
    let cursor0 = cursor.clone();
    let c1 = cursor.next();
    let cursor1 = cursor.clone();
    let c2 = cursor.next();

    match (c1, c2) {
        (fill @ Some(_), align @ Some('<' | '^' | '>')) => [fill, align],
        (align @ Some('<' | '^' | '>'), _) => {
            *cursor = cursor1;
            [align, None]
        }
        _ => {
            *cursor = cursor0;
            [None, None]
        }
    }
}

pub(super) fn process_sign(cursor: &mut StrCursor) -> Option<char> {
    let old_cursor = cursor.clone();

    match cursor.next() {
        sign @ Some('+' | '-') => sign,
        _ => {
            *cursor = old_cursor;
            None
        }
    }
}

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

pub(super) fn process_sign_aware_zero_pad(cursor: &mut StrCursor) -> Option<char> {
    let old_cursor = cursor.clone();

    match cursor.next() {
        sign @ Some('0') => sign,
        _ => {
            *cursor = old_cursor;
            None
        }
    }
}

pub(super) fn process_width<'a>(cursor: &mut StrCursor<'a>) -> Option<Count<'a>> {
    process_count(cursor)
}

pub(super) fn process_precision<'a>(cursor: &mut StrCursor<'a>) -> Option<Precision<'a>> {
    let mut old_cursor = cursor.clone();

    if !matches!(cursor.next(), Some('.')) {
        *cursor = old_cursor;
        return None;
    }

    old_cursor = cursor.clone();

    match cursor.next() {
        Some('*') => Some(Precision::Asterisk),
        _ => {
            *cursor = old_cursor;
            match process_count(cursor) {
                Some(count) => Some(Precision::WithCount(count)),
                None => panic!("invalid count in format string"),
            }
        }
    }
}

pub(super) fn process_count<'a>(cursor: &mut StrCursor<'a>) -> Option<Count<'a>> {
    let old_cursor = cursor.clone();

    // Try parsing as argument with '$'
    match parse_argument(cursor) {
        Some(arg_type) if cursor.next() == Some('$') => return Some(Count::Argument(arg_type)),
        _ => *cursor = old_cursor,
    }

    // Try parsing as integer
    match cursor.read_while(|c| c.is_ascii_digit()) {
        "" => None,
        integer => Some(Count::Integer(integer)),
    }
}

pub(super) fn parse_argument<'a>(cursor: &mut StrCursor<'a>) -> Option<ArgType<'a>> {
    // Try parsing as integer
    let integer_argument = cursor.read_while(|c| c.is_ascii_digit());
    if !integer_argument.is_empty() {
        return Some(ArgType::Positional(integer_argument.parse().unwrap()));
    }

    // Try parsing as identifier
    let old_cursor = cursor.clone();
    let remaining = cursor.remaining();

    let first_char = cursor.next()?;
    let first_char_len = remaining.len() - cursor.remaining().len();

    let identifier = match first_char {
        '_' => match cursor.read_while(unicode_ident::is_xid_continue).len() {
            0 => {
                *cursor = old_cursor;
                return None;
            }
            len => &remaining[..first_char_len + len],
        },
        c => {
            if unicode_ident::is_xid_start(c) {
                let len = cursor.read_while(unicode_ident::is_xid_continue).len();
                &remaining[..first_char_len + len]
            } else {
                *cursor = old_cursor;
                return None;
            }
        }
    };

    Some(ArgType::Named(Id::new(identifier)))
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

        for (fmt, output, remaining) in data {
            let mut cursor = StrCursor::new(fmt);
            assert_eq!(process_align(&mut cursor), output);
            assert_eq!(cursor.remaining(), remaining);
        }
    }

    #[test]
    fn test_process_sign() {
        let data = [("+000", Some('+'), "000"), ("-000", Some('-'), "000"), ("0000", None, "0000")];

        for (fmt, output, remaining) in data {
            let mut cursor = StrCursor::new(fmt);
            assert_eq!(process_sign(&mut cursor), output);
            assert_eq!(cursor.remaining(), remaining);
        }
    }

    #[test]
    fn test_process_alternate() {
        let data = [("#0", Some('#'), "0"), ("00", None, "00")];

        for (fmt, output, remaining) in data {
            let mut cursor = StrCursor::new(fmt);
            assert_eq!(process_alternate(&mut cursor), output);
            assert_eq!(cursor.remaining(), remaining);
        }
    }

    #[test]
    fn test_process_sign_aware_zero_pad() {
        let data = [("0123", Some('0'), "123"), ("123", None, "123")];

        for (fmt, output, remaining) in data {
            let mut cursor = StrCursor::new(fmt);
            assert_eq!(process_sign_aware_zero_pad(&mut cursor), output);
            assert_eq!(cursor.remaining(), remaining);
        }
    }

    #[test]
    fn test_parse_argument() {
        let data = [
            ("05sdkfh-", Some(ArgType::Positional(5)), "sdkfh-"),
            ("_sdkfh-", Some(ArgType::Named(Id::new("_sdkfh"))), "-"),
            ("_é€", Some(ArgType::Named(Id::new("_é"))), "€"),
            ("é€", Some(ArgType::Named(Id::new("é"))), "€"),
            ("@é€", None, "@é€"),
            ("€", None, "€"),
        ];

        for (fmt, output, remaining) in data {
            let mut cursor = StrCursor::new(fmt);
            assert_eq!(parse_argument(&mut cursor), output);
            assert_eq!(cursor.remaining(), remaining);
        }
    }

    #[test]
    #[should_panic(expected = "identifiers in format string must be normalized in Unicode NFC")]
    fn test_parse_argument_not_nfc() {
        parse_argument(&mut StrCursor::new("A\u{30a}"));
    }

    #[test]
    fn test_process_width() {
        let data = [
            ("05sdkfh$-", Some(Count::Integer("05")), "sdkfh$-"),
            ("05$sdkfh-", Some(Count::Argument(ArgType::Positional(5))), "sdkfh-"),
            ("_sdkfh$-", Some(Count::Argument(ArgType::Named(Id::new("_sdkfh")))), "-"),
            ("_é$€", Some(Count::Argument(ArgType::Named(Id::new("_é")))), "€"),
            ("é$€", Some(Count::Argument(ArgType::Named(Id::new("é")))), "€"),
            ("_sdkfh-$", None, "_sdkfh-$"),
            ("_é€$", None, "_é€$"),
            ("é€$", None, "é€$"),
            ("@é€", None, "@é€"),
            ("€", None, "€"),
        ];

        for (fmt, output, remaining) in data {
            let mut cursor = StrCursor::new(fmt);
            assert_eq!(process_width(&mut cursor), output);
            assert_eq!(cursor.remaining(), remaining);
        }
    }

    #[test]
    fn test_process_precision() {
        let data = [
            (".*--", Some(Precision::Asterisk), "--"),
            (".05sdkfh$-", Some(Precision::WithCount(Count::Integer("05"))), "sdkfh$-"),
            (".05$sdkfh-", Some(Precision::WithCount(Count::Argument(ArgType::Positional(5)))), "sdkfh-"),
            ("._sdkfh$-", Some(Precision::WithCount(Count::Argument(ArgType::Named(Id::new("_sdkfh"))))), "-"),
            ("._é$€", Some(Precision::WithCount(Count::Argument(ArgType::Named(Id::new("_é"))))), "€"),
            (".é$€", Some(Precision::WithCount(Count::Argument(ArgType::Named(Id::new("é"))))), "€"),
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

        for (fmt, output, remaining) in data {
            let mut cursor = StrCursor::new(fmt);
            assert_eq!(process_precision(&mut cursor), output);
            assert_eq!(cursor.remaining(), remaining);
        }
    }

    #[test]
    #[should_panic(expected = "invalid count in format string")]
    fn test_process_precision_invalid_1() {
        process_precision(&mut StrCursor::new("._sdkfh-$"));
    }

    #[test]
    #[should_panic(expected = "invalid count in format string")]
    fn test_process_precision_invalid_2() {
        process_precision(&mut StrCursor::new("._é€$"));
    }

    #[test]
    #[should_panic(expected = "invalid count in format string")]
    fn test_process_precision_invalid_3() {
        process_precision(&mut StrCursor::new(".é€$"));
    }

    #[test]
    #[should_panic(expected = "invalid count in format string")]
    fn test_process_precision_invalid_4() {
        process_precision(&mut StrCursor::new(".@é€"));
    }

    #[test]
    #[should_panic(expected = "invalid count in format string")]
    fn test_process_precision_invalid_5() {
        process_precision(&mut StrCursor::new(".€"));
    }
}
