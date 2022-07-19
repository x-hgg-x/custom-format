use super::utils::StrCursor;
use super::{ArgType, Count, Precision};

use proc_macro::TokenStream;
use std::str::FromStr;

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

    assert_eq!(identifier, TokenStream::from_str(identifier).unwrap().to_string(), "identifiers in format string must be normalized in Unicode NFC");

    Some(ArgType::Named(identifier))
}
