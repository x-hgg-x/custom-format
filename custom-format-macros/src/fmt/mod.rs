//! Module containing procedural macros common code.

mod parse;
mod utils;

use utils::StrCursor;

use std::collections::hash_map::{Entry, HashMap};
use std::fmt::Write;
use std::str;

#[cfg(not(test))]
use proc_macro::TokenStream;
#[cfg(test)]
use proc_macro2::TokenStream;

/// Separator for custom format specifier
const CUSTOM_SEPARATOR: &str = " :";

/// Proc-macro argument
#[derive(Debug)]
struct Argument {
    /// Optional name
    name: Option<String>,
    /// Expression
    expr: String,
}

/// Identifier normalized in Unicode NFC
#[derive(Debug, PartialEq)]
struct Id<'a>(&'a str);

impl<'a> Id<'a> {
    /// Construct a new [`Id`] value
    fn new(name: &'a str) -> Self {
        Self::check_unicode_nfc(name);
        Self(name)
    }

    /// Check if the identifier is normalized in Unicode NFC
    fn check_unicode_nfc(name: &str) {
        #[cfg(not(test))]
        let normalized_name = name.parse::<proc_macro::TokenStream>().unwrap().to_string();
        #[cfg(test)]
        let normalized_name = unicode_normalization::UnicodeNormalization::nfc(name).collect::<String>();

        assert_eq!(name, normalized_name, "identifiers in format string must be normalized in Unicode NFC");
    }

    /// Returns the identifier value
    fn name(&self) -> &'a str {
        self.0
    }
}

/// Kind of a proc-macro argument
#[derive(Debug, PartialEq)]
enum ArgKind<'a> {
    /// Positional argument
    Positional(usize),
    /// Named argument
    Named(Id<'a>),
}

/// Standard count format specifier
#[derive(Debug, PartialEq)]
enum Count<'a> {
    /// Count is provided by an argument
    Argument(ArgKind<'a>),
    /// Count is provided by an integer
    Integer(&'a str),
}

/// Standard precision format specifier
#[derive(Debug, PartialEq)]
enum Precision<'a> {
    /// Precision is provided by the next positional argument
    Asterisk,
    /// Precision is provided by the specified count
    WithCount(Count<'a>),
}

/// Custom format specifier
#[derive(Debug, Copy, Clone, PartialEq)]
enum Spec<'a> {
    // Format specifier checked at compile-time
    CompileTime(&'a str),
    // Format specifier checked at runtime
    Runtime(&'a str),
}

/// Piece of a format string
#[derive(Debug, PartialEq)]
enum Piece<'a> {
    /// Standard format specifier data
    StdFmt {
        /// Kind of the positional argument
        arg_kind_position: ArgKind<'a>,
        /// Optional kind of the width argument
        arg_kind_width: Option<ArgKind<'a>>,
        /// Optional kind of the precision argument
        arg_kind_precision: Option<ArgKind<'a>>,
    },
    /// Custom format specifier data
    CustomFmt {
        /// Kind of the positional argument
        arg_kind: ArgKind<'a>,
        /// Custom format specifier
        spec: Spec<'a>,
    },
}

/// Parse input tokens into a list of arguments
#[cfg(not(feature = "better-parsing"))]
fn parse_tokens_fast(input: TokenStream, skip_first: bool) -> (Option<String>, String, Vec<Argument>) {
    #[cfg(not(test))]
    use proc_macro::{Spacing, TokenTree};
    #[cfg(test)]
    use proc_macro2::{Spacing, TokenTree};

    let expr_to_string = |tokens| -> String {
        let mut output = String::new();
        for token in tokens {
            write!(output, "{}", token).unwrap();
        }
        output
    };

    let token_trees: Vec<_> = input.into_iter().collect();

    let mut args_tokens_iter = token_trees.split(|token| matches!(token, TokenTree::Punct(punct) if punct.as_char() == ',' ));

    let first_arg = if skip_first { args_tokens_iter.next().map(expr_to_string) } else { None };

    let format_string = match args_tokens_iter.next() {
        Some([arg]) => match litrs::StringLit::parse(arg.to_string()) {
            Ok(lit) => lit.into_value().into_owned(),
            Err(_) => panic!("format argument must be a string literal"),
        },
        _ => panic!("invalid format argument"),
    };

    let mut arguments: Vec<_> = args_tokens_iter
        .map(|arg_tokens| match arg_tokens {
            [TokenTree::Ident(ident), TokenTree::Punct(punct), tail @ ..] if punct.as_char() == '=' => match punct.spacing() {
                Spacing::Alone => Argument { name: Some(ident.to_string()), expr: expr_to_string(tail) },
                Spacing::Joint => match tail.first() {
                    Some(TokenTree::Punct(next_punct)) if matches!(next_punct.as_char(), '=' | '>') => {
                        Argument { name: None, expr: expr_to_string(arg_tokens) }
                    }
                    _ => Argument { name: Some(ident.to_string()), expr: expr_to_string(tail) },
                },
            },
            _ => Argument { name: None, expr: expr_to_string(arg_tokens) },
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

/// Parse input tokens into a list of arguments
#[cfg(feature = "better-parsing")]
fn parse_tokens_syn(input: TokenStream, skip_first: bool) -> (Option<String>, String, Vec<Argument>) {
    use quote::ToTokens;
    use syn::parse::Parser;
    use syn::punctuated::Punctuated;
    use syn::{Expr, ExprAssign, Token};

    #[allow(clippy::useless_conversion)]
    let input = input.into();

    let mut args_iter = Punctuated::<Expr, Token![,]>::parse_terminated.parse2(input).unwrap_or_else(|_| panic!("invalid syntax")).into_iter();

    let first_arg = if skip_first { args_iter.next().map(|expr| expr.to_token_stream().to_string()) } else { None };

    let format_string = match args_iter.next() {
        Some(expr) => match litrs::StringLit::parse(expr.to_token_stream().to_string()) {
            Ok(lit) => lit.into_value().into_owned(),
            Err(_) => panic!("format argument must be a string literal"),
        },
        _ => panic!("invalid format argument"),
    };

    let arguments: Vec<_> = args_iter
        .map(|expr| match expr {
            Expr::Assign(ExprAssign { left, right, .. }) => {
                Argument { name: Some(left.to_token_stream().to_string()), expr: right.to_token_stream().to_string() }
            }
            expr => Argument { name: None, expr: expr.to_token_stream().to_string() },
        })
        .collect();

    (first_arg, format_string, arguments)
}

/// Process formatting argument
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
            let specifier = &inner[position + CUSTOM_SEPARATOR.len()..];

            let mut spec_chars = specifier.chars();
            let spec = match (spec_chars.next(), spec_chars.next_back()) {
                (Some('<'), Some('>')) => Spec::Runtime(spec_chars.as_str()),
                _ => Spec::CompileTime(specifier),
            };

            let mut cursor = StrCursor::new(&inner[..position]);

            let arg_kind = parse::parse_argument(&mut cursor).unwrap_or_else(|| {
                let arg_kind = ArgKind::Positional(*current_positional_index);
                *current_positional_index += 1;
                arg_kind
            });

            assert!(cursor.remaining().is_empty(), "invalid format string");

            Piece::CustomFmt { arg_kind, spec }
        }
        None => {
            let mut cursor = StrCursor::new(inner);

            let mut has_arg_kind = true;
            let mut arg_kind_position = parse::parse_argument(&mut cursor).unwrap_or_else(|| {
                let arg_kind = ArgKind::Positional(*current_positional_index);
                *current_positional_index += 1;
                has_arg_kind = false;
                arg_kind
            });

            let mut arg_kind_width = None;
            let mut arg_kind_precision = None;

            match cursor.next() {
                Some(':') => {
                    new_format_string.push(':');
                    new_format_string.extend(parse::process_align(&mut cursor).iter().flatten());
                    new_format_string.extend(parse::process_sign(&mut cursor));
                    new_format_string.extend(parse::process_alternate(&mut cursor));
                    new_format_string.extend(parse::process_sign_aware_zero_pad(&mut cursor));

                    match parse::process_width(&mut cursor) {
                        None => (),
                        Some(Count::Integer(integer)) => *new_format_string += integer,
                        Some(Count::Argument(arg_kind_for_width)) => {
                            arg_kind_width = Some(arg_kind_for_width);
                            write!(new_format_string, "{}$", *new_current_index).unwrap();
                            *new_current_index += 1;
                        }
                    }

                    match parse::process_precision(&mut cursor) {
                        None => (),
                        Some(Precision::Asterisk) => {
                            let new_arg_kind = ArgKind::Positional(*current_positional_index);
                            *current_positional_index += 1;

                            if has_arg_kind {
                                arg_kind_precision = Some(new_arg_kind);
                            } else {
                                arg_kind_precision = Some(arg_kind_position);
                                arg_kind_position = new_arg_kind;
                            }

                            write!(new_format_string, ".{}$", *new_current_index).unwrap();
                            *new_current_index += 1;
                        }
                        Some(Precision::WithCount(Count::Integer(integer))) => write!(new_format_string, ".{}", integer).unwrap(),
                        Some(Precision::WithCount(Count::Argument(arg_kind_for_precision))) => {
                            arg_kind_precision = Some(arg_kind_for_precision);
                            write!(new_format_string, ".{}$", *new_current_index).unwrap();
                            *new_current_index += 1;
                        }
                    };

                    *new_format_string += cursor.remaining();
                }
                None => (),
                _ => panic!("invalid format string"),
            };

            Piece::StdFmt { arg_kind_position, arg_kind_width, arg_kind_precision }
        }
    };

    new_format_string.push('}');

    piece
}

/// Parse format string
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

/// Process list of pieces
fn process_pieces<'a>(pieces: &'a [Piece], arguments: &[Argument]) -> (Vec<(usize, Option<Spec<'a>>)>, Vec<&'a str>) {
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

    let mut process_arg_kind = |arg_kind: &_, spec| {
        let index = match *arg_kind {
            ArgKind::Positional(index) => {
                assert!(index < arguments.len(), "invalid positional argument index: {}", index);
                arg_indices.push((index, spec));
                index
            }
            ArgKind::Named(ref ident) => match named_args_positions.entry(ident.name()) {
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
            Piece::StdFmt { arg_kind_position, arg_kind_width, arg_kind_precision } => {
                for &arg_kind in [Some(arg_kind_position), arg_kind_width.as_ref(), arg_kind_precision.as_ref()].iter().flatten() {
                    process_arg_kind(arg_kind, None)
                }
            }
            Piece::CustomFmt { arg_kind, spec } => process_arg_kind(arg_kind, Some(*spec)),
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

/// Write literal string
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

/// Compute output Rust code
fn compute_output(
    root_macro: &str,
    first_arg: Option<&str>,
    new_format_string: &str,
    arguments: &[Argument],
    arg_indices: &[(usize, Option<Spec>)],
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
            Some(Spec::CompileTime(spec)) => write!(output, "::custom_format::custom_formatter!(\"{}\", arg{})", spec, index).unwrap(),
            Some(Spec::Runtime(spec)) => write!(output, "::custom_format::runtime::CustomFormatter::new(\"{}\", arg{})", spec, index).unwrap(),
            None => write!(output, "arg{}", index).unwrap(),
        }
    }

    output.push_str(") } }");

    output
}

/// Main function of the procedural macros
pub(crate) fn fmt(input: TokenStream, skip_first: bool, root_macro: &str) -> String {
    if input.is_empty() {
        return format!("{}()", root_macro).parse().unwrap();
    }

    #[cfg(not(feature = "better-parsing"))]
    let parse_tokens = parse_tokens_fast;
    #[cfg(feature = "better-parsing")]
    let parse_tokens = parse_tokens_syn;

    let (first_arg, format_string, arguments) = parse_tokens(input, skip_first);
    let (new_format_string, pieces) = parse_format_string(&format_string);
    let (arg_indices, new_args) = process_pieces(&pieces, &arguments);

    compute_output(root_macro, first_arg.as_deref(), &new_format_string, &arguments, &arg_indices, &new_args)
}

#[cfg(test)]
mod test {
    use super::*;

    #[cfg(not(feature = "better-parsing"))]
    mod fast_parsing {
        use super::*;

        #[test]
        fn test_parse_tokens_fast() {
            let s1 = r#"
                "format string", 5==3, (), Custom(1f64.abs()), std::format!("{:?},{}", (3, 4), 5),
                z=::std::f64::MAX, r = &1 + 4, b = 2, c = Custom(6), e = { g }
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

            for &(s, skip_first, result_first_arg) in &data {
                let (first_arg, format_string, arguments) = parse_tokens_fast(s.parse::<TokenStream>().unwrap(), skip_first);

                assert_eq!(first_arg.as_deref(), result_first_arg);
                assert_eq!(format_string, result_format_string);

                for ((arg, &result_name), &result_expr) in arguments.iter().zip(&result_argument_names).zip(&result_argument_exprs) {
                    assert_eq!(arg.name.as_deref(), result_name);
                    assert_eq!(arg.expr.to_string().replace(' ', ""), result_expr);
                }
            }
        }

        #[test]
        #[should_panic(expected = "format argument must be a string literal")]
        fn test_parse_tokens_fast_not_string_literal() {
            parse_tokens_fast(r#""{}", 1"#.parse::<TokenStream>().unwrap(), true);
        }

        #[test]
        #[should_panic(expected = "invalid format argument")]
        fn test_parse_tokens_fast_invalid_format_string() {
            parse_tokens_fast(",1".parse::<TokenStream>().unwrap(), false);
        }

        #[test]
        #[should_panic(expected = "invalid syntax: empty argument")]
        fn test_parse_tokens_fast_empty_argument() {
            parse_tokens_fast(r#""{}", ,"#.parse::<TokenStream>().unwrap(), false);
        }
    }

    #[cfg(feature = "better-parsing")]
    mod syn_parsing {
        use super::*;

        #[test]
        #[should_panic(expected = "invalid syntax")]
        fn test_parse_tokens_syn_invalid_format_string() {
            parse_tokens_syn(",1".parse::<TokenStream>().unwrap(), false);
        }

        #[test]
        #[should_panic(expected = "format argument must be a string literal")]
        fn test_parse_tokens_syn_not_string_literal() {
            parse_tokens_syn(r#""{}", 1"#.parse::<TokenStream>().unwrap(), true);
        }
    }

    #[test]
    fn test_process_fmt() {
        #[rustfmt::skip]
        let data = [
            ("{ :}",            "{0}",             1, 1, Piece::CustomFmt { arg_kind: ArgKind::Positional(0),        spec: Spec::CompileTime("") }),
            ("{ : \t\r\n }",    "{0}",             1, 1, Piece::CustomFmt { arg_kind: ArgKind::Positional(0),        spec: Spec::CompileTime("") }),
            ("{ :\u{2000} }",   "{0}",             1, 1, Piece::CustomFmt { arg_kind: ArgKind::Positional(0),        spec: Spec::CompileTime("") }),
            ("{ : : : }",       "{0}",             1, 1, Piece::CustomFmt { arg_kind: ArgKind::Positional(0),        spec: Spec::CompileTime(" : :") }),
            ("{ : <: :> }",     "{0}",             1, 1, Piece::CustomFmt { arg_kind: ArgKind::Positional(0),        spec: Spec::CompileTime(" <: :>") }),
            ("{ : éà }" ,       "{0}",             1, 1, Piece::CustomFmt { arg_kind: ArgKind::Positional(0),        spec: Spec::CompileTime(" éà") }),
            ("{ : <éà> }" ,     "{0}",             1, 1, Piece::CustomFmt { arg_kind: ArgKind::Positional(0),        spec: Spec::CompileTime(" <éà>") }),
            ("{3 :%a }",        "{0}",             0, 1, Piece::CustomFmt { arg_kind: ArgKind::Positional(3),        spec: Spec::CompileTime("%a") }),
            ("{éà :%a}",        "{0}",             0, 1, Piece::CustomFmt { arg_kind: ArgKind::Named(Id::new("éà")), spec: Spec::CompileTime("%a") }),
            ("{éà :<<<>>%a><}", "{0}",             0, 1, Piece::CustomFmt { arg_kind: ArgKind::Named(Id::new("éà")), spec: Spec::CompileTime("<<<>>%a><") }),
            ("{ :<>}",          "{0}",             1, 1, Piece::CustomFmt { arg_kind: ArgKind::Positional(0),        spec: Spec::Runtime("") }),
            ("{ :<> \t\r\n }",  "{0}",             1, 1, Piece::CustomFmt { arg_kind: ArgKind::Positional(0),        spec: Spec::Runtime("") }),
            ("{ :<>\u{2000} }", "{0}",             1, 1, Piece::CustomFmt { arg_kind: ArgKind::Positional(0),        spec: Spec::Runtime("") }),
            ("{ :< : :> }",     "{0}",             1, 1, Piece::CustomFmt { arg_kind: ArgKind::Positional(0),        spec: Spec::Runtime(" : :") }),
            ("{ :<%a> }",       "{0}",             1, 1, Piece::CustomFmt { arg_kind: ArgKind::Positional(0),        spec: Spec::Runtime("%a") }),
            ("{3 :<%a> }",      "{0}",             0, 1, Piece::CustomFmt { arg_kind: ArgKind::Positional(3),        spec: Spec::Runtime("%a") }),
            ("{éà :<%a>}",      "{0}",             0, 1, Piece::CustomFmt { arg_kind: ArgKind::Named(Id::new("éà")), spec: Spec::Runtime("%a") }),
            ("{éà :<<<>>%a>}",  "{0}",             0, 1, Piece::CustomFmt { arg_kind: ArgKind::Named(Id::new("éà")), spec: Spec::Runtime("<<>>%a") }),
            ("{}",              "{0}",             1, 1, Piece::StdFmt { arg_kind_position: ArgKind::Positional(0),        arg_kind_width: None,                               arg_kind_precision: None }),
            ("{:?}",            "{0:?}",           1, 1, Piece::StdFmt { arg_kind_position: ArgKind::Positional(0),        arg_kind_width: None,                               arg_kind_precision: None }),
            ("{3:? }",          "{0:?}",           0, 1, Piece::StdFmt { arg_kind_position: ArgKind::Positional(3),        arg_kind_width: None,                               arg_kind_precision: None }),
            ("{éà}",            "{0}",             0, 1, Piece::StdFmt { arg_kind_position: ArgKind::Named(Id::new("éà")), arg_kind_width: None,                               arg_kind_precision: None }),
            ("{: ^+#03.6? }",   "{0: ^+#03.6?}",   1, 1, Piece::StdFmt { arg_kind_position: ArgKind::Positional(0),        arg_kind_width: None,                               arg_kind_precision: None }),
            ("{: ^+#0a$.6? }",  "{0: ^+#01$.6?}",  1, 2, Piece::StdFmt { arg_kind_position: ArgKind::Positional(0),        arg_kind_width: Some(ArgKind::Named(Id::new("a"))), arg_kind_precision: None }),
            ("{: ^+#03.6$? }",  "{0: ^+#03.1$?}",  1, 2, Piece::StdFmt { arg_kind_position: ArgKind::Positional(0),        arg_kind_width: None,                               arg_kind_precision: Some(ArgKind::Positional(6)) }),
            ("{: ^+#03$.d$? }", "{0: ^+#01$.2$?}", 1, 3, Piece::StdFmt { arg_kind_position: ArgKind::Positional(0),        arg_kind_width: Some(ArgKind::Positional(3)),       arg_kind_precision: Some(ArgKind::Named(Id::new("d"))) }),
            ("{: ^+#0z$.*? }",  "{0: ^+#01$.2$?}", 2, 3, Piece::StdFmt { arg_kind_position: ArgKind::Positional(1),        arg_kind_width: Some(ArgKind::Named(Id::new("z"))), arg_kind_precision: Some(ArgKind::Positional(0)) }),
            ("{2: ^+#03$.*? }", "{0: ^+#01$.2$?}", 1, 3, Piece::StdFmt { arg_kind_position: ArgKind::Positional(2),        arg_kind_width: Some(ArgKind::Positional(3)),       arg_kind_precision: Some(ArgKind::Positional(0)) }),
            ("{:1$? }",         "{0:1$?}",         1, 2, Piece::StdFmt { arg_kind_position: ArgKind::Positional(0),        arg_kind_width: Some(ArgKind::Positional(1)),       arg_kind_precision: None }),
            ("{:.2$? }",        "{0:.1$?}",        1, 2, Piece::StdFmt { arg_kind_position: ArgKind::Positional(0),        arg_kind_width: None,                               arg_kind_precision: Some(ArgKind::Positional(2)) }),
            ("{:.*? }",         "{0:.1$?}",        2, 2, Piece::StdFmt { arg_kind_position: ArgKind::Positional(1),        arg_kind_width: None,                               arg_kind_precision: Some(ArgKind::Positional(0)) }),
            ("{a:.*? }",        "{0:.1$?}",        1, 2, Piece::StdFmt { arg_kind_position: ArgKind::Named(Id::new("a")),  arg_kind_width: None,                               arg_kind_precision: Some(ArgKind::Positional(0)) }),
        ];

        for &(fmt, result_new_format_string, result_current_positional_index, result_new_current_index, ref result_piece) in &data {
            let mut new_format_string = String::new();
            let mut current_positional_index = 0;
            let mut new_current_index = 0;

            let piece = process_fmt(fmt, &mut current_positional_index, &mut new_format_string, &mut new_current_index);

            assert_eq!(new_format_string, result_new_format_string);
            assert_eq!(current_positional_index, result_current_positional_index);
            assert_eq!(new_current_index, result_new_current_index);
            assert_eq!(piece, *result_piece);
        }
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
    #[should_panic(expected = "invalid argument: argument name cannot be a single underscore")]
    fn test_process_fmt_invalid_named_argument() {
        process_fmt("{_:?}", &mut 0, &mut String::new(), &mut 0);
    }

    #[test]
    fn test_parse_format_string() {
        let format_string = "aaaa }} {{}}{} {{{{ \" {:#.*} #{h :<z>} {e \u{3A}3xxx\u{47}xxxxxxx  }, {:?}, { :}, {:?}, {},,{}, {8 :<>}";

        let result_new_format_string = "aaaa }} {{}}{0} {{{{ \" {1:#.2$} #{3} {4}, {5:?}, {6}, {7:?}, {8},,{9}, {10}";

        let result_pieces = [
            Piece::StdFmt { arg_kind_position: ArgKind::Positional(0), arg_kind_width: None, arg_kind_precision: None },
            Piece::StdFmt { arg_kind_position: ArgKind::Positional(2), arg_kind_width: None, arg_kind_precision: Some(ArgKind::Positional(1)) },
            Piece::CustomFmt { arg_kind: ArgKind::Named(Id("h")), spec: Spec::Runtime("z") },
            Piece::CustomFmt { arg_kind: ArgKind::Named(Id("e")), spec: Spec::CompileTime("3xxxGxxxxxxx") },
            Piece::StdFmt { arg_kind_position: ArgKind::Positional(3), arg_kind_width: None, arg_kind_precision: None },
            Piece::CustomFmt { arg_kind: ArgKind::Positional(4), spec: Spec::CompileTime("") },
            Piece::StdFmt { arg_kind_position: ArgKind::Positional(5), arg_kind_width: None, arg_kind_precision: None },
            Piece::StdFmt { arg_kind_position: ArgKind::Positional(6), arg_kind_width: None, arg_kind_precision: None },
            Piece::StdFmt { arg_kind_position: ArgKind::Positional(7), arg_kind_width: None, arg_kind_precision: None },
            Piece::CustomFmt { arg_kind: ArgKind::Positional(8), spec: Spec::Runtime("") },
        ];

        let (new_format_string, pieces) = parse_format_string(format_string);

        assert_eq!(new_format_string, result_new_format_string);
        assert_eq!(pieces, result_pieces);
    }

    #[test]
    fn test_process_pieces() {
        let pieces = [
            Piece::StdFmt { arg_kind_position: ArgKind::Named(Id::new("h")), arg_kind_width: None, arg_kind_precision: None },
            Piece::CustomFmt { arg_kind: ArgKind::Named(Id::new("h")), spec: Spec::CompileTime("%z") },
            Piece::StdFmt { arg_kind_position: ArgKind::Positional(1), arg_kind_width: None, arg_kind_precision: None },
            Piece::StdFmt { arg_kind_position: ArgKind::Named(Id::new("a")), arg_kind_width: None, arg_kind_precision: None },
            Piece::StdFmt { arg_kind_position: ArgKind::Positional(3), arg_kind_width: None, arg_kind_precision: None },
            Piece::StdFmt { arg_kind_position: ArgKind::Named(Id::new("b")), arg_kind_width: None, arg_kind_precision: None },
            Piece::StdFmt { arg_kind_position: ArgKind::Positional(1), arg_kind_width: None, arg_kind_precision: Some(ArgKind::Positional(0)) },
            Piece::StdFmt { arg_kind_position: ArgKind::Positional(3), arg_kind_width: Some(ArgKind::Named(Id::new("g"))), arg_kind_precision: None },
        ];

        let arguments = [
            Argument { name: None, expr: "".to_owned() },
            Argument { name: Some("a".to_owned()), expr: "".to_owned() },
            Argument { name: Some("b".to_owned()), expr: "".to_owned() },
            Argument { name: Some("c".to_owned()), expr: "".to_owned() },
        ];

        let result_arg_indices =
            [(4, None), (4, Some(Spec::CompileTime("%z"))), (1, None), (1, None), (3, None), (2, None), (1, None), (0, None), (3, None), (5, None)];

        let result_new_args = ["h", "g"];

        let (arg_indices, new_args) = process_pieces(&pieces, &arguments);

        assert_eq!(arg_indices, result_arg_indices);
        assert_eq!(new_args, result_new_args);
    }

    #[test]
    #[should_panic(expected = "positional arguments cannot follow named arguments")]
    fn test_process_pieces_positional_after_named() {
        process_pieces(&[], &[Argument { name: Some("é".to_owned()), expr: "".to_owned() }, Argument { name: None, expr: "".to_owned() }]);
    }

    #[test]
    #[should_panic(expected = "duplicate argument named `a`")]
    fn test_process_pieces_duplicate_named_argument() {
        process_pieces(&[], &[Argument { name: Some("a".to_owned()), expr: "".to_owned() }, Argument { name: Some("a".to_owned()), expr: "".to_owned() }]);
    }

    #[test]
    #[should_panic(expected = "invalid positional argument index: 0")]
    fn test_process_pieces_invalid_positional_argument() {
        process_pieces(&[Piece::CustomFmt { arg_kind: ArgKind::Positional(0), spec: Spec::CompileTime("") }], &[]);
    }

    #[test]
    #[should_panic(expected = "positional argument 0 not used")]
    fn test_process_pieces_positional_argument_not_used() {
        process_pieces(&[], &[Argument { name: None, expr: "".to_owned() }]);
    }

    #[test]
    #[should_panic(expected = "named argument `a` not used")]
    fn test_process_pieces_named_argument_not_used() {
        process_pieces(&[], &[Argument { name: Some("a".to_owned()), expr: "".to_owned() }]);
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

        let arguments = [
            Argument { name: None, expr: r#""0""#.to_owned() },
            Argument { name: Some("a".to_owned()), expr: r#""1""#.to_owned() },
            Argument { name: Some("b".to_owned()), expr: r#""2""#.to_owned() },
            Argument { name: Some("c".to_owned()), expr: r#""3""#.to_owned() },
        ];

        let arg_indices = [
            (4, None),
            (4, Some(Spec::CompileTime("%z"))),
            (1, None),
            (1, None),
            (3, None),
            (2, Some(Spec::Runtime("%x"))),
            (1, None),
            (0, None),
            (3, None),
            (5, None),
        ];

        let new_args = ["h", "g"];

        let result = concat!(
            r#"match (&("0"), &("1"), &("2"), &("3"), &(h), &(g)) { (arg0, arg1, arg2, arg3, arg4, arg5) => { "#,
            r#"::std::println!("{0}, {1}, {2}, {3}, {4}, {5}, {6:.7$}, {8:9$}", arg4, "#,
            r#"::custom_format::custom_formatter!("%z", arg4), arg1, arg1, arg3, "#,
            r#"::custom_format::runtime::CustomFormatter::new("%x", arg2), arg1, arg0, arg3, arg5) } }"#
        );

        let output = compute_output("::std::println!", None, new_format_string, &arguments, &arg_indices, &new_args);

        assert_eq!(output, result);
    }

    #[test]
    fn test_compute_output_with_first_arg() {
        let output = compute_output("::std::writeln!", Some("f"), "string", &[], &[], &[]);
        assert_eq!(output, "match () { () => { ::std::writeln!(f, \"string\") } }");
    }
}
