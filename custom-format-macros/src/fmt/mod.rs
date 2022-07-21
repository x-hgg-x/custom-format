mod parse;
mod utils;

use utils::StrCursor;

use std::borrow::Cow;
use std::collections::hash_map::{Entry, HashMap};
use std::fmt::{self, Display, Write};
use std::str;

#[cfg(not(test))]
use proc_macro::{Spacing, TokenStream, TokenTree};
#[cfg(test)]
use proc_macro2::{Spacing, TokenStream, TokenTree};

const CUSTOM_SEPARATOR: &str = " :";

#[derive(Debug)]
struct Expr<'a>(&'a [TokenTree]);

impl<'a> Expr<'a> {
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Display for Expr<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.iter().try_for_each(|token| write!(f, "{}", token))
    }
}

#[derive(Debug)]
struct Argument<'a> {
    name: Option<String>,
    expr: Expr<'a>,
}

#[derive(Debug, PartialEq)]
struct Id<'a>(&'a str);

impl<'a> Id<'a> {
    fn new(name: &'a str) -> Self {
        Self::check_unicode_nfc(name);
        Self(name)
    }

    fn check_unicode_nfc(name: &str) {
        #[cfg(not(test))]
        let normalized_name = name.parse::<proc_macro::TokenStream>().unwrap().to_string();
        #[cfg(test)]
        let normalized_name = unicode_normalization::UnicodeNormalization::nfc(name).collect::<String>();

        assert_eq!(name, normalized_name, "identifiers in format string must be normalized in Unicode NFC");
    }

    fn name(&self) -> &'a str {
        self.0
    }
}

#[derive(Debug, PartialEq)]
enum ArgType<'a> {
    Positional(usize),
    Named(Id<'a>),
}

#[derive(Debug, PartialEq)]
enum Count<'a> {
    Argument(ArgType<'a>),
    Integer(&'a str),
}

#[derive(Debug, PartialEq)]
enum Precision<'a> {
    Asterisk,
    WithCount(Count<'a>),
}

#[derive(Debug, PartialEq)]
enum Piece<'a> {
    StdFmt { arg_type_position: ArgType<'a>, arg_type_width: Option<ArgType<'a>>, arg_type_precision: Option<ArgType<'a>> },
    CustomFmt { arg_type: ArgType<'a>, spec: &'a str },
}

fn is_valid_spec_byte(x: u8) -> bool {
    x < 0x7F && (x.is_ascii_alphanumeric() || b"!#$%*+-.<=>?@^_~".binary_search(&x).is_ok())
}

fn parse_tokens(token_trees: &[TokenTree], skip_first: bool) -> (Option<Expr>, Cow<str>, Vec<Argument>) {
    let mut args_tokens_iter = token_trees.split(|token| matches!(token, TokenTree::Punct(punct) if punct.as_char() == ',' ));

    let first_arg = if skip_first { args_tokens_iter.next().map(Expr) } else { None };

    let format_string = match args_tokens_iter.next() {
        Some([arg]) => match litrs::StringLit::parse(arg.to_string()) {
            Ok(lit) => lit.into_value(),
            Err(_) => panic!("format argument must be a string literal"),
        },
        _ => panic!("invalid format argument"),
    };

    let mut arguments: Vec<_> = args_tokens_iter
        .map(|arg_tokens| match arg_tokens {
            [TokenTree::Ident(ident), TokenTree::Punct(punct), tail @ ..] if punct.as_char() == '=' => match punct.spacing() {
                Spacing::Alone => Argument { name: Some(ident.to_string()), expr: Expr(tail) },
                Spacing::Joint => match tail.first() {
                    Some(TokenTree::Punct(next_punct)) if matches!(next_punct.as_char(), '=' | '>') => Argument { name: None, expr: Expr(arg_tokens) },
                    _ => Argument { name: Some(ident.to_string()), expr: Expr(tail) },
                },
            },
            _ => Argument { name: None, expr: Expr(arg_tokens) },
        })
        .collect();

    if let Some(last) = arguments.last() {
        if last.expr.is_empty() {
            arguments.pop();
        }
    }

    assert!(arguments.iter().all(|arg| !arg.expr.is_empty()), "invalid syntax: empty argument");

    (first_arg, format_string, arguments)
}

fn process_fmt<'a>(fmt: &'a str, current_positional_index: &mut usize, new_format_string: &mut String, new_current_index: &mut usize) -> Piece<'a> {
    let mut fmt_chars = fmt.chars();
    let inner = match (fmt_chars.next(), fmt_chars.next_back()) {
        (Some('{'), Some('}')) => fmt_chars.as_str().trim_end(),
        _ => panic!("invalid format string"),
    };

    write!(new_format_string, "{{{}", *new_current_index).unwrap();
    *new_current_index += 1;

    let piece = match inner.find(CUSTOM_SEPARATOR) {
        Some(position) => {
            let spec = &inner[position + CUSTOM_SEPARATOR.len()..];
            assert!(spec.bytes().all(is_valid_spec_byte), "invalid char in format spec");

            let mut cursor = StrCursor::new(&inner[..position]);

            let arg_type = parse::parse_argument(&mut cursor).unwrap_or_else(|| {
                let arg_type = ArgType::Positional(*current_positional_index);
                *current_positional_index += 1;
                arg_type
            });

            assert!(cursor.remaining().is_empty(), "invalid format string");

            Piece::CustomFmt { arg_type, spec }
        }
        None => {
            let mut cursor = StrCursor::new(inner);

            let mut has_arg_type = true;
            let mut arg_type_position = parse::parse_argument(&mut cursor).unwrap_or_else(|| {
                let arg_type = ArgType::Positional(*current_positional_index);
                *current_positional_index += 1;
                has_arg_type = false;
                arg_type
            });

            let mut arg_type_width = None;
            let mut arg_type_precision = None;

            match cursor.next() {
                Some(':') => {
                    new_format_string.push(':');
                    new_format_string.extend(parse::process_align(&mut cursor).into_iter().flatten());
                    new_format_string.extend(parse::process_sign(&mut cursor));
                    new_format_string.extend(parse::process_alternate(&mut cursor));
                    new_format_string.extend(parse::process_sign_aware_zero_pad(&mut cursor));

                    match parse::process_width(&mut cursor) {
                        None => (),
                        Some(Count::Integer(integer)) => *new_format_string += integer,
                        Some(Count::Argument(arg_type_for_width)) => {
                            arg_type_width = Some(arg_type_for_width);
                            write!(new_format_string, "{}$", *new_current_index).unwrap();
                            *new_current_index += 1;
                        }
                    }

                    match parse::process_precision(&mut cursor) {
                        None => (),
                        Some(Precision::Asterisk) => {
                            let new_arg_type = ArgType::Positional(*current_positional_index);
                            *current_positional_index += 1;

                            if has_arg_type {
                                arg_type_precision = Some(new_arg_type);
                            } else {
                                arg_type_precision = Some(arg_type_position);
                                arg_type_position = new_arg_type;
                            }

                            write!(new_format_string, ".{}$", *new_current_index).unwrap();
                            *new_current_index += 1;
                        }
                        Some(Precision::WithCount(Count::Integer(integer))) => write!(new_format_string, ".{}", integer).unwrap(),
                        Some(Precision::WithCount(Count::Argument(arg_type_for_precision))) => {
                            arg_type_precision = Some(arg_type_for_precision);
                            write!(new_format_string, ".{}$", *new_current_index).unwrap();
                            *new_current_index += 1;
                        }
                    };

                    *new_format_string += cursor.remaining();
                }
                None => (),
                _ => panic!("invalid format string"),
            };

            Piece::StdFmt { arg_type_position, arg_type_width, arg_type_precision }
        }
    };

    new_format_string.push('}');

    piece
}

fn parse_format_string(format_string: &str) -> (String, Vec<Piece>) {
    let mut cursor = StrCursor::new(format_string);
    let mut current_positional_index = 0;

    let mut pieces = Vec::new();
    let mut new_format_string = String::new();
    let mut new_current_index = 0;

    loop {
        new_format_string += cursor.read_until(|c| c == '{');

        if cursor.remaining().is_empty() {
            break;
        }

        if cursor.remaining().starts_with("{{") {
            cursor.next();
            cursor.next();
            new_format_string += "{{";
            continue;
        }

        let fmt = cursor.read_until_included(|c| c == '}');
        pieces.push(process_fmt(fmt, &mut current_positional_index, &mut new_format_string, &mut new_current_index));
    }

    (new_format_string, pieces)
}

fn process_pieces<'a>(pieces: &'a [Piece], arguments: &[Argument]) -> (Vec<(usize, Option<&'a str>)>, Vec<&'a str>) {
    let mut arguments_iter = arguments.iter();
    arguments_iter.position(|arg| arg.name.is_some());
    assert!(arguments_iter.all(|arg| arg.name.is_some()), "positional arguments cannot follow named arguments");

    let mut named_args_positions = HashMap::new();
    for (index, arg) in arguments.iter().enumerate() {
        if let Some(name) = arg.name.as_deref() {
            assert!(named_args_positions.insert(name, index).is_none(), "duplicate argument named `{}`", name);
        }
    }

    let mut arg_indices = Vec::new();
    let mut new_args = Vec::new();
    let mut used_args = vec![false; arguments.len()];

    let mut process_arg_type = |arg_type: &_, spec| {
        let index = match *arg_type {
            ArgType::Positional(index) => {
                assert!(index < arguments.len(), "invalid positional argument index: {}", index);
                arg_indices.push((index, spec));
                index
            }
            ArgType::Named(ref ident) => match named_args_positions.entry(ident.name()) {
                Entry::Occupied(entry) => {
                    let index = *entry.get();
                    arg_indices.push((index, spec));
                    index
                }
                Entry::Vacant(entry) => {
                    let new_index = arguments.len() + new_args.len();
                    entry.insert(new_index);
                    arg_indices.push((new_index, spec));
                    new_args.push(ident.name());
                    new_index
                }
            },
        };

        if let Some(used) = used_args.get_mut(index) {
            *used = true;
        }
    };

    for piece in pieces {
        match piece {
            Piece::StdFmt { arg_type_position, arg_type_width, arg_type_precision } => {
                for arg_type in [Some(arg_type_position), arg_type_width.as_ref(), arg_type_precision.as_ref()].into_iter().flatten() {
                    process_arg_type(arg_type, None)
                }
            }
            Piece::CustomFmt { arg_type, spec } => process_arg_type(arg_type, Some(*spec)),
        }
    }

    if let Some((index, (arg, _))) = arguments.iter().zip(&used_args).enumerate().find(|(_, (_, &used))| !used) {
        match arg.name.as_deref() {
            Some(name) => panic!("named argument `{}` not used", name),
            None => panic!("positional argument {} not used", index),
        }
    }

    (arg_indices, new_args)
}

fn write_literal_string(output: &mut String, s: &str) {
    output.push('\"');

    for c in s.chars() {
        match c {
            '\x20'..='\x7E' if c != '"' && c != '\\' => output.push(c),
            _ => write!(output, "\\u{{{:X}}}", c as i32).unwrap(),
        }
    }

    output.push('\"');
}

fn compute_output(
    root_macro: &str,
    first_arg: Option<Expr>,
    new_format_string: &str,
    arguments: &[Argument],
    arg_indices: &[(usize, Option<&str>)],
    new_args: &[&str],
) -> String {
    let mut output = String::new();

    let arg_count = arguments.len() + new_args.len();

    output.push_str("match (");

    for arg in arguments {
        write!(output, "&({}), ", arg.expr).unwrap();
    }

    for &name in new_args {
        write!(output, "&({}), ", name).unwrap();
    }

    if arg_count > 0 {
        output.pop();
        output.pop();
    }

    output.push_str(") { (");

    for index in 0..arg_count {
        write!(output, "arg{}, ", index).unwrap();
    }

    if arg_count > 0 {
        output.pop();
        output.pop();
    }

    write!(output, ") => {{ {}(", root_macro).unwrap();

    if let Some(first_arg) = first_arg {
        write!(output, "{}, ", first_arg).unwrap();
    }

    write_literal_string(&mut output, new_format_string);

    for &(index, spec) in arg_indices {
        output.push_str(", ");

        match spec {
            Some(spec) => write!(output, "::custom_format::CustomFormatter::new(arg{}, \"{}\")", index, spec).unwrap(),
            None => write!(output, "arg{}", index).unwrap(),
        }
    }

    output.push_str(") } }");

    output
}

pub(crate) fn fmt(input: TokenStream, skip_first: bool, root_macro: &str) -> String {
    if input.is_empty() {
        return format!("{}()", root_macro).parse().unwrap();
    }

    let token_trees: Vec<_> = input.into_iter().collect();
    let (first_arg, format_string, arguments) = parse_tokens(&token_trees, skip_first);
    let (new_format_string, pieces) = parse_format_string(&format_string);
    let (arg_indices, new_args) = process_pieces(&pieces, &arguments);

    compute_output(root_macro, first_arg, &new_format_string, &arguments, &arg_indices, &new_args)
}

#[cfg(test)]
mod test {
    use super::*;

    use proc_macro2::Literal;

    #[test]
    fn test_is_valid_spec_byte() {
        let result: Vec<_> = (0..4).map(|i| (0..64).map(|j| (is_valid_spec_byte(64 * i + j) as u64) << j).sum::<u64>()).collect();
        assert_eq!(result, [0xF3FF6C3A00000000, 0x47FFFFFEC7FFFFFF, 0, 0]);
    }

    #[test]
    fn test_parse_tokens() {
        let s1 = r#"
            "format string", 5==3, (), Custom(1f64.abs()), std::format!("{:?},{}", (3, 4), 5),
            z=::std::f64::MAX, r = &1 + 4, b = 2, c = Custom(6), e = { g },
        "#;

        let s2 = r##"
            f, r#"format string"#, 5==3, (), Custom(1f64.abs()), std::format!("{:?},{}", (3, 4), 5),
            z=::std::f64::MAX, r = &1 + 4, b = 2, c = Custom(6), e = { g },
        "##;

        let result_format_string = "format string";
        let result_argument_names = [None, None, None, None, Some("z"), Some("r"), Some("b"), Some("c"), Some("e")];
        let result_argument_exprs =
            ["5==3", "()", "Custom(1f64.abs())", r#"std::format!("{:?},{}",(3,4),5)"#, "::std::f64::MAX", "&1+4", "2", "Custom(6)", "{g}"];

        let data = [(s1, false, None), (s2, true, Some("f"))];

        for (s, skip_first, result_first_arg) in data {
            let token_trees: Vec<_> = s.parse::<TokenStream>().unwrap().into_iter().collect();
            let (first_arg, format_string, arguments) = parse_tokens(&token_trees, skip_first);

            assert_eq!(first_arg.map(|expr| expr.to_string()).as_deref(), result_first_arg);
            assert_eq!(format_string, result_format_string);

            for ((arg, result_name), result_expr) in arguments.iter().zip(result_argument_names).zip(result_argument_exprs) {
                assert_eq!(arg.name.as_deref(), result_name);
                assert_eq!(arg.expr.to_string().replace(' ', ""), result_expr);
            }
        }
    }

    #[test]
    #[should_panic(expected = "format argument must be a string literal")]
    fn test_parse_tokens_not_string_literal() {
        let token_trees: Vec<_> = r#""{}", 1"#.parse::<TokenStream>().unwrap().into_iter().collect();
        parse_tokens(&token_trees, true);
    }

    #[test]
    #[should_panic(expected = "invalid format argument")]
    fn test_parse_tokens_invalid_format_string() {
        let token_trees: Vec<_> = ",1".parse::<TokenStream>().unwrap().into_iter().collect();
        parse_tokens(&token_trees, false);
    }

    #[test]
    #[should_panic(expected = "invalid syntax: empty argument")]
    fn test_parse_tokens_empty_argument() {
        let token_trees: Vec<_> = r#""{}", ,"#.parse::<TokenStream>().unwrap().into_iter().collect();
        parse_tokens(&token_trees, false);
    }

    #[test]
    fn test_process_fmt() {
        #[rustfmt::skip]
        let data = [
            ("{ :}",            "{0}",             1, 1, Piece::CustomFmt { arg_type: ArgType::Positional(0), spec: "" }),
            ("{ : }",           "{0}",             1, 1, Piece::CustomFmt { arg_type: ArgType::Positional(0), spec: "" }),
            ("{ :%a }",         "{0}",             1, 1, Piece::CustomFmt { arg_type: ArgType::Positional(0), spec: "%a" }),
            ("{3 :%a }",        "{0}",             0, 1, Piece::CustomFmt { arg_type: ArgType::Positional(3), spec: "%a" }),
            ("{éà :%a}",        "{0}",             0, 1, Piece::CustomFmt { arg_type: ArgType::Named(Id::new("éà")), spec: "%a" }),
            ("{}",              "{0}",             1, 1, Piece::StdFmt { arg_type_position: ArgType::Positional(0),        arg_type_width: None,                               arg_type_precision: None }),
            ("{:?}",            "{0:?}",           1, 1, Piece::StdFmt { arg_type_position: ArgType::Positional(0),        arg_type_width: None,                               arg_type_precision: None }),
            ("{3:? }",          "{0:?}",           0, 1, Piece::StdFmt { arg_type_position: ArgType::Positional(3),        arg_type_width: None,                               arg_type_precision: None }),
            ("{éà}",            "{0}",             0, 1, Piece::StdFmt { arg_type_position: ArgType::Named(Id::new("éà")), arg_type_width: None,                               arg_type_precision: None }),
            ("{: ^+#03.6? }",   "{0: ^+#03.6?}",   1, 1, Piece::StdFmt { arg_type_position: ArgType::Positional(0),        arg_type_width: None,                               arg_type_precision: None }),
            ("{: ^+#0a$.6? }",  "{0: ^+#01$.6?}",  1, 2, Piece::StdFmt { arg_type_position: ArgType::Positional(0),        arg_type_width: Some(ArgType::Named(Id::new("a"))), arg_type_precision: None }),
            ("{: ^+#03.6$? }",  "{0: ^+#03.1$?}",  1, 2, Piece::StdFmt { arg_type_position: ArgType::Positional(0),        arg_type_width: None,                               arg_type_precision: Some(ArgType::Positional(6)) }),
            ("{: ^+#03$.d$? }", "{0: ^+#01$.2$?}", 1, 3, Piece::StdFmt { arg_type_position: ArgType::Positional(0),        arg_type_width: Some(ArgType::Positional(3)),       arg_type_precision: Some(ArgType::Named(Id::new("d"))) }),
            ("{: ^+#0z$.*? }",  "{0: ^+#01$.2$?}", 2, 3, Piece::StdFmt { arg_type_position: ArgType::Positional(1),        arg_type_width: Some(ArgType::Named(Id::new("z"))), arg_type_precision: Some(ArgType::Positional(0)) }),
            ("{2: ^+#03$.*? }", "{0: ^+#01$.2$?}", 1, 3, Piece::StdFmt { arg_type_position: ArgType::Positional(2),        arg_type_width: Some(ArgType::Positional(3)),       arg_type_precision: Some(ArgType::Positional(0)) }),
            ("{:1$? }",         "{0:1$?}",         1, 2, Piece::StdFmt { arg_type_position: ArgType::Positional(0),        arg_type_width: Some(ArgType::Positional(1)),       arg_type_precision: None }),
            ("{:.2$? }",        "{0:.1$?}",        1, 2, Piece::StdFmt { arg_type_position: ArgType::Positional(0),        arg_type_width: None,                               arg_type_precision: Some(ArgType::Positional(2)) }),
            ("{:.*? }",         "{0:.1$?}",        2, 2, Piece::StdFmt { arg_type_position: ArgType::Positional(1),        arg_type_width: None,                               arg_type_precision: Some(ArgType::Positional(0)) }),
            ("{a:.*? }",        "{0:.1$?}",        1, 2, Piece::StdFmt { arg_type_position: ArgType::Named(Id::new("a")),  arg_type_width: None,                               arg_type_precision: Some(ArgType::Positional(0)) }),
        ];

        for (fmt, result_new_format_string, result_current_positional_index, result_new_current_index, result_piece) in data {
            let mut new_format_string = String::new();
            let mut current_positional_index = 0;
            let mut new_current_index = 0;

            let piece = process_fmt(fmt, &mut current_positional_index, &mut new_format_string, &mut new_current_index);

            assert_eq!(new_format_string, result_new_format_string);
            assert_eq!(current_positional_index, result_current_positional_index);
            assert_eq!(new_current_index, result_new_current_index);
            assert_eq!(piece, result_piece);
        }
    }

    #[test]
    #[should_panic(expected = "invalid char in format spec")]
    fn test_process_fmt_invalid_char_format_spec() {
        process_fmt("{ :%ù }", &mut 0, &mut String::new(), &mut 0);
    }

    #[test]
    #[should_panic(expected = "invalid format string")]
    fn test_process_fmt_invalid_format_string_1() {
        process_fmt("{: ", &mut 0, &mut String::new(), &mut 0);
    }

    #[test]
    #[should_panic(expected = "invalid format string")]
    fn test_process_fmt_invalid_format_string_2() {
        process_fmt("{0éà0 :%a}", &mut 0, &mut String::new(), &mut 0);
    }

    #[test]
    #[should_panic(expected = "invalid format string")]
    fn test_process_fmt_invalid_format_string_3() {
        process_fmt("{0éà0}", &mut 0, &mut String::new(), &mut 0);
    }

    #[test]
    #[should_panic(expected = "invalid count in format string")]
    fn test_process_fmt_invalid_count_format_string() {
        process_fmt("{0:.}", &mut 0, &mut String::new(), &mut 0);
    }

    #[test]
    fn test_parse_format_string() {
        let format_string = "aaaa }} {{}}{} {{{{ \" {:#.*} #{h : } {e \u{3A}3xxx\u{47}xxxxxxx  }, {:?}, { :}, {:?}, {},,{}, {8 :}";

        let result_new_format_string = "aaaa }} {{}}{0} {{{{ \" {1:#.2$} #{3} {4}, {5:?}, {6}, {7:?}, {8},,{9}, {10}";

        let result_pieces = [
            Piece::StdFmt { arg_type_position: ArgType::Positional(0), arg_type_width: None, arg_type_precision: None },
            Piece::StdFmt { arg_type_position: ArgType::Positional(2), arg_type_width: None, arg_type_precision: Some(ArgType::Positional(1)) },
            Piece::CustomFmt { arg_type: ArgType::Named(Id("h")), spec: "" },
            Piece::CustomFmt { arg_type: ArgType::Named(Id("e")), spec: "3xxxGxxxxxxx" },
            Piece::StdFmt { arg_type_position: ArgType::Positional(3), arg_type_width: None, arg_type_precision: None },
            Piece::CustomFmt { arg_type: ArgType::Positional(4), spec: "" },
            Piece::StdFmt { arg_type_position: ArgType::Positional(5), arg_type_width: None, arg_type_precision: None },
            Piece::StdFmt { arg_type_position: ArgType::Positional(6), arg_type_width: None, arg_type_precision: None },
            Piece::StdFmt { arg_type_position: ArgType::Positional(7), arg_type_width: None, arg_type_precision: None },
            Piece::CustomFmt { arg_type: ArgType::Positional(8), spec: "" },
        ];

        let (new_format_string, pieces) = parse_format_string(format_string);

        assert_eq!(new_format_string, result_new_format_string);
        assert_eq!(pieces, result_pieces);
    }

    #[test]
    fn test_process_pieces() {
        let pieces = [
            Piece::StdFmt { arg_type_position: ArgType::Named(Id::new("h")), arg_type_width: None, arg_type_precision: None },
            Piece::CustomFmt { arg_type: ArgType::Named(Id::new("h")), spec: "%z" },
            Piece::StdFmt { arg_type_position: ArgType::Positional(1), arg_type_width: None, arg_type_precision: None },
            Piece::StdFmt { arg_type_position: ArgType::Named(Id::new("a")), arg_type_width: None, arg_type_precision: None },
            Piece::StdFmt { arg_type_position: ArgType::Positional(3), arg_type_width: None, arg_type_precision: None },
            Piece::StdFmt { arg_type_position: ArgType::Named(Id::new("b")), arg_type_width: None, arg_type_precision: None },
            Piece::StdFmt { arg_type_position: ArgType::Positional(1), arg_type_width: None, arg_type_precision: Some(ArgType::Positional(0)) },
            Piece::StdFmt { arg_type_position: ArgType::Positional(3), arg_type_width: Some(ArgType::Named(Id::new("g"))), arg_type_precision: None },
        ];

        let arguments = [
            Argument { name: None, expr: Expr(&[]) },
            Argument { name: Some("a".to_owned()), expr: Expr(&[]) },
            Argument { name: Some("b".to_owned()), expr: Expr(&[]) },
            Argument { name: Some("c".to_owned()), expr: Expr(&[]) },
        ];

        let result_arg_indices = [(4, None), (4, Some("%z")), (1, None), (1, None), (3, None), (2, None), (1, None), (0, None), (3, None), (5, None)];
        let result_new_args = ["h", "g"];

        let (arg_indices, new_args) = process_pieces(&pieces, &arguments);

        assert_eq!(arg_indices, result_arg_indices);
        assert_eq!(new_args, result_new_args);
    }

    #[test]
    #[should_panic(expected = "positional arguments cannot follow named arguments")]
    fn test_process_pieces_positional_after_named() {
        process_pieces(&[], &[Argument { name: Some("é".to_owned()), expr: Expr(&[]) }, Argument { name: None, expr: Expr(&[]) }]);
    }

    #[test]
    #[should_panic(expected = "duplicate argument named `a`")]
    fn test_process_pieces_duplicate_named_argument() {
        process_pieces(&[], &[Argument { name: Some("a".to_owned()), expr: Expr(&[]) }, Argument { name: Some("a".to_owned()), expr: Expr(&[]) }]);
    }

    #[test]
    #[should_panic(expected = "invalid positional argument index: 0")]
    fn test_process_pieces_invalid_positional_argument() {
        process_pieces(&[Piece::CustomFmt { arg_type: ArgType::Positional(0), spec: "" }], &[]);
    }

    #[test]
    #[should_panic(expected = "positional argument 0 not used")]
    fn test_process_pieces_positional_argument_not_used() {
        process_pieces(&[], &[Argument { name: None, expr: Expr(&[]) }]);
    }

    #[test]
    #[should_panic(expected = "named argument `a` not used")]
    fn test_process_pieces_named_argument_not_used() {
        process_pieces(&[], &[Argument { name: Some("a".to_owned()), expr: Expr(&[]) }]);
    }

    #[test]
    fn test_write_literal_string() {
        let mut output = String::new();

        write_literal_string(
            &mut output,
            "\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0A\x0B\x0C\x0D\x0E\x0F\
             \x10\x11\x12\x13\x14\x15\x16\x17\x18\x19\x1A\x1B\x1C\x1D\x1E\x1F\
             \x20\x21\x22\x23\x24\x25\x26\x27\x28\x29\x2A\x2B\x2C\x2D\x2E\x2F\
             \x30\x31\x32\x33\x34\x35\x36\x37\x38\x39\x3A\x3B\x3C\x3D\x3E\x3F\
             \x40\x41\x42\x43\x44\x45\x46\x47\x48\x49\x4A\x4B\x4C\x4D\x4E\x4F\
             \x50\x51\x52\x53\x54\x55\x56\x57\x58\x59\x5A\x5B\x5C\x5D\x5E\x5F\
             \x60\x61\x62\x63\x64\x65\x66\x67\x68\x69\x6A\x6B\x6C\x6D\x6E\x6F\
             \x70\x71\x72\x73\x74\x75\x76\x77\x78\x79\x7A\x7B\x7B\x7C\x7D\x7D\
             \x7E\x7F\u{e9}\u{211D}",
        );

        let result = concat!(
            r#""\u{0}\u{1}\u{2}\u{3}\u{4}\u{5}\u{6}\u{7}\u{8}\u{9}\u{A}\u{B}\u{C}\u{D}"#,
            r#"\u{E}\u{F}\u{10}\u{11}\u{12}\u{13}\u{14}\u{15}\u{16}\u{17}\u{18}\u{19}"#,
            r#"\u{1A}\u{1B}\u{1C}\u{1D}\u{1E}\u{1F} !\u{22}#$%&'()*+,-./0123456789:;"#,
            r#"<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\u{5C}]^_`abcdefghijklmnopqrstuvwxyz"#,
            r#"{{|}}~\u{7F}\u{E9}\u{211D}""#,
        );

        assert_eq!(output, result);
    }

    #[test]
    fn test_compute_output() {
        let new_format_string = "{0}, {1}, {2}, {3}, {4}, {5}, {6:.7$}, {8:9$}";

        let tokens1 = &[TokenTree::from(Literal::string("0"))];
        let tokens2 = &[TokenTree::from(Literal::string("1"))];
        let tokens3 = &[TokenTree::from(Literal::string("2"))];
        let tokens4 = &[TokenTree::from(Literal::string("3"))];

        let arguments = [
            Argument { name: None, expr: Expr(tokens1) },
            Argument { name: Some("a".to_owned()), expr: Expr(tokens2) },
            Argument { name: Some("b".to_owned()), expr: Expr(tokens3) },
            Argument { name: Some("c".to_owned()), expr: Expr(tokens4) },
        ];

        let arg_indices = [(4, None), (4, Some("%z")), (1, None), (1, None), (3, None), (2, Some("%x")), (1, None), (0, None), (3, None), (5, None)];
        let new_args = ["h", "g"];

        let result = concat!(
            r#"match (&("0"), &("1"), &("2"), &("3"), &(h), &(g)) { (arg0, arg1, arg2, arg3, arg4, arg5) => { "#,
            r#"::std::println!("{0}, {1}, {2}, {3}, {4}, {5}, {6:.7$}, {8:9$}", arg4, "#,
            r#"::custom_format::CustomFormatter::new(arg4, "%z"), arg1, arg1, arg3, "#,
            r#"::custom_format::CustomFormatter::new(arg2, "%x"), arg1, arg0, arg3, arg5) } }"#
        );

        let output = compute_output("::std::println!", None, new_format_string, &arguments, &arg_indices, &new_args);

        assert_eq!(output, result);
    }
}
