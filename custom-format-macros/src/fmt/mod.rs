mod parse;
mod utils;

use utils::StrCursor;

use proc_macro::{Spacing, TokenStream, TokenTree};
use std::collections::hash_map::{Entry, HashMap};
use std::fmt::{self, Display, Write};
use std::str;

const CUSTOM_SEPARATOR: &str = " :";

struct Expr<'a>(&'a [TokenTree]);

impl Display for Expr<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.iter().try_for_each(|token| write!(f, "{}", token))
    }
}

struct Argument<'a> {
    expr: Expr<'a>,
    name: Option<String>,
}

enum ArgType<'a> {
    Positional(usize),
    Named(&'a str),
}

enum Count<'a> {
    Argument(ArgType<'a>),
    Integer(&'a str),
}

enum Precision<'a> {
    Asterisk,
    WithCount(Count<'a>),
}

enum Piece<'a> {
    StdFmt { arg_type_position: ArgType<'a>, arg_type_width: Option<ArgType<'a>>, arg_type_precision: Option<ArgType<'a>> },
    CustomFmt { arg_type: ArgType<'a>, spec: &'a str },
}

fn is_arg_separator(token: &TokenTree) -> bool {
    matches!(token, TokenTree::Punct(punct) if punct.as_char() == ',' )
}

fn is_valid_spec_char(x: u8) -> bool {
    matches!(x, b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z'| b'_'| b'!'| b'#'| b'$'| b'%'| b'*' | b'+'| b'-'| b'.'| b'<'| b'='| b'>'| b'?'| b'@'| b'^'| b'~')
}

fn process_fmt<'a>(fmt: &'a str, current_positional_index: &mut usize, new_format_string: &mut String, new_current_index: &mut usize) -> Piece<'a> {
    match fmt.find(CUSTOM_SEPARATOR) {
        Some(position) => {
            let spec = &fmt[position + CUSTOM_SEPARATOR.len()..];
            assert!(spec.bytes().all(is_valid_spec_char), "invalid char in format spec");

            write!(new_format_string, "{}", *new_current_index).unwrap();
            *new_current_index += 1;

            let arg_type = parse::parse_argument(&mut StrCursor::new(&fmt[..position])).unwrap_or_else(|| {
                let arg_type = ArgType::Positional(*current_positional_index);
                *current_positional_index += 1;
                arg_type
            });

            Piece::CustomFmt { arg_type, spec }
        }
        None => {
            write!(new_format_string, "{}", *new_current_index).unwrap();
            *new_current_index += 1;

            let mut cursor = StrCursor::new(fmt);

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
                        Some(Precision::WithCount(Count::Integer(integer))) => *new_format_string += integer,
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
    }
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
        assert!(matches!(fmt.as_bytes(), [b'{', .., b'}']), "invalid format string");

        new_format_string.push('{');
        let piece = process_fmt(fmt[1..fmt.len() - 1].trim_end(), &mut current_positional_index, &mut new_format_string, &mut new_current_index);
        new_format_string.push('}');

        pieces.push(piece);
    }

    (new_format_string, pieces)
}

fn process_pieces<'a>(pieces: &'a [Piece], arguments: &[Argument]) -> (Vec<(usize, Option<&'a str>)>, Vec<&'a str>) {
    let mut named_args_positions = HashMap::new();
    for (index, arg) in arguments.iter().enumerate() {
        if let Some(name) = arg.name.as_deref() {
            assert!(named_args_positions.insert(name, index).is_none(), "duplicate argument named `{}`", name);
        }
    }

    let mut arg_indices = Vec::new();
    let mut new_args = Vec::new();

    let mut process_arg_type = |arg_type: &_, spec| match *arg_type {
        ArgType::Positional(index) => {
            assert!(index < arguments.len(), "invalid positional argument index: {}", index);
            arg_indices.push((index, spec));
        }
        ArgType::Named(name) => match named_args_positions.entry(name) {
            Entry::Occupied(entry) => arg_indices.push((*entry.get(), spec)),
            Entry::Vacant(entry) => {
                let new_index = arguments.len() + new_args.len();
                entry.insert(new_index);
                arg_indices.push((new_index, spec));
                new_args.push(name);
            }
        },
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

pub(crate) fn fmt(input: TokenStream, skip_first: bool, root_macro: &str) -> TokenStream {
    let mut token_trees: Vec<_> = input.into_iter().collect();

    if token_trees.is_empty() {
        return format!("{}()", root_macro).parse().unwrap();
    }

    if token_trees.last().map(is_arg_separator) == Some(true) {
        token_trees.pop();
    }

    let mut args_tokens_iter = token_trees.split(is_arg_separator);

    let first_arg = if skip_first { args_tokens_iter.next().map(Expr) } else { None };

    let format_string = match args_tokens_iter.next() {
        Some([arg]) => match litrs::StringLit::try_from(arg) {
            Ok(lit) => lit.into_value(),
            Err(_) => panic!("format argument must be a string literal"),
        },
        _ => panic!("invalid format argument"),
    };

    let arguments: Vec<_> = args_tokens_iter
        .map(|arg_tokens| match arg_tokens {
            [TokenTree::Ident(ident), TokenTree::Punct(punct), tail @ ..] if punct.as_char() == '=' && punct.spacing() == Spacing::Alone => {
                Argument { expr: Expr(tail), name: Some(ident.to_string()) }
            }
            _ => Argument { expr: Expr(arg_tokens), name: None },
        })
        .collect();

    let (new_format_string, pieces) = parse_format_string(&format_string);
    let (arg_indices, new_args) = process_pieces(&pieces, &arguments);

    let output = compute_output(root_macro, first_arg, &new_format_string, &arguments, &arg_indices, &new_args);

    output.parse().unwrap()
}
