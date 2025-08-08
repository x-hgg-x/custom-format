//! Functions used for processing input.

use super::utils::StrCursor;
use super::*;

use std::collections::hash_map::{Entry, HashMap};
use std::fmt::Write;

/// Parse input tokens
pub(super) fn parse_tokens(input: TokenStream) -> Result<(String, ParsedInput), TokenStream> {
    let token_trees: Vec<_> = input.into_iter().collect();

    let mut args_iter = token_trees.split(|token| matches!(token, TokenTree::Punct(punct) if punct.as_char() == ',' ));

    let crate_ident = match args_iter.next() {
        Some([TokenTree::Ident(ident)]) => ident.clone(),
        _ => return Err(compile_error("invalid tokens", Span::call_site())),
    };

    // A `$crate` identifier is impossible to construct with `proc_macro2::Ident`
    #[cfg(not(test))]
    if &crate_ident.to_string() != "$crate" {
        return Err(compile_error("invalid tokens", Span::call_site()));
    }

    let root_macro = match args_iter.next() {
        Some([TokenTree::Group(group)]) => group.stream(),
        _ => return Err(compile_error("invalid tokens", Span::call_site())),
    };

    let first_arg = match args_iter.next() {
        Some([TokenTree::Group(group)]) => match group.stream() {
            stream if !stream.is_empty() => Some(stream),
            _ => None,
        },
        _ => return Err(compile_error("invalid tokens", Span::call_site())),
    };

    let remaining: Vec<_> = match args_iter.next() {
        Some([TokenTree::Group(group)]) => group.stream().into_iter().collect(),
        _ => return Err(compile_error("invalid tokens", Span::call_site())),
    };

    let mut remaining_iter = remaining.split(|token| matches!(token, TokenTree::Punct(punct) if punct.as_char() == ',' ));

    let (format_string, span) = match remaining_iter.next() {
        Some([TokenTree::Group(group)]) => {
            let mut stream_iter = group.stream().into_iter();
            match (stream_iter.next(), stream_iter.next()) {
                (Some(tt), None) => {
                    let span = tt.span();
                    match litrs::StringLit::parse(tt.to_string()) {
                        Ok(lit) => (lit.into_value(), span),
                        Err(e) => return Err(compile_error(&e.to_string(), span)),
                    }
                }
                _ => return Err(compile_error("invalid tokens", Span::call_site())),
            }
        }
        _ => return Err(compile_error("invalid tokens", Span::call_site())),
    };

    let arguments = remaining_iter
        .map(|x| match x {
            [TokenTree::Group(group)] => {
                let mut ident = None;
                let mut stream = group.stream();

                let mut stream_iter = stream.clone().into_iter();
                let (tt1, tt2, tt3, tt4) = (stream_iter.next(), stream_iter.next(), stream_iter.next(), stream_iter.next());

                if let Some(TokenTree::Group(g1)) = tt1 {
                    let g1_inner = g1.stream().to_string();

                    // Since Rust 1.61: Proc macros no longer see ident matchers wrapped in groups (#92472)
                    let mut g1_iter = g1_inner.parse::<TokenStream>().ok().into_iter().flat_map(|x| x.into_iter());

                    if let (Some(TokenTree::Ident(_)), None) = (g1_iter.next(), g1_iter.next()) {
                        if let (Some(TokenTree::Punct(punct)), Some(TokenTree::Group(inner_group)), None) = (tt2, tt3, tt4) {
                            if punct.as_char() == '=' && punct.spacing() == Spacing::Alone {
                                ident = Some(g1_inner);
                                stream = inner_group.stream();
                            }
                        }
                    }
                }

                Ok(Argument { ident, expr: Group::new(Delimiter::Parenthesis, stream) })
            }
            _ => Err(compile_error("invalid tokens", span)),
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok((format_string, ParsedInput { crate_ident, root_macro, first_arg, arguments, span }))
}

/// Process formatting argument
fn process_fmt<'a>(
    fmt: &'a str,
    current_positional_index: &mut usize,
    new_format_string: &mut String,
    new_current_index: &mut usize,
) -> Result<Piece<'a>, Error> {
    let mut fmt_chars = fmt.chars();
    let inner = match (fmt_chars.next(), fmt_chars.next_back()) {
        (Some('{'), Some('}')) => fmt_chars.as_str().trim_end(),
        _ => return Err("invalid format string".into()),
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

            let arg_kind = parse::parse_argument(&mut cursor)?.unwrap_or_else(|| {
                let arg_kind = ArgKind::Positional(*current_positional_index);
                *current_positional_index += 1;
                arg_kind
            });

            if !cursor.remaining().is_empty() {
                return Err("invalid format string".into());
            }

            Piece::CustomFmt { arg_kind, spec }
        }
        None => {
            let mut cursor = StrCursor::new(inner);

            let mut has_arg_kind = true;
            let mut arg_kind_position = parse::parse_argument(&mut cursor)?.unwrap_or_else(|| {
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

                    match parse::process_width(&mut cursor)? {
                        None => (),
                        Some(Count::Integer(integer)) => *new_format_string += integer,
                        Some(Count::Argument(arg_kind_for_width)) => {
                            arg_kind_width = Some(arg_kind_for_width);
                            write!(new_format_string, "{}$", *new_current_index).unwrap();
                            *new_current_index += 1;
                        }
                    }

                    match parse::process_precision(&mut cursor)? {
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
                _ => return Err("invalid format string".into()),
            };

            Piece::StdFmt { arg_kind_position, arg_kind_width, arg_kind_precision }
        }
    };

    new_format_string.push('}');

    Ok(piece)
}

/// Parse format string
pub(super) fn parse_format_string(format_string: &str) -> Result<(String, Vec<Piece<'_>>), Error> {
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
        pieces.push(process_fmt(fmt, &mut current_positional_index, &mut new_format_string, &mut new_current_index)?);
    }

    Ok((new_format_string, pieces))
}

/// Process list of pieces
pub(super) fn process_pieces<'a>(pieces: Vec<Piece<'a>>, arguments: &[Argument]) -> Result<ProcessedPieces<'a>, Error> {
    let mut arguments_iter = arguments.iter();
    arguments_iter.position(|arg| arg.ident.is_some());

    if !arguments_iter.all(|arg| arg.ident.is_some()) {
        return Err("positional arguments cannot follow named arguments".into());
    }

    let mut named_args_positions = HashMap::new();
    for (index, arg) in arguments.iter().enumerate() {
        if let Some(ident) = &arg.ident {
            if named_args_positions.insert(ident.clone(), index).is_some() {
                return Err(format!("duplicate argument named `{}`", ident).into());
            }
        }
    }

    let mut arg_indices = Vec::new();
    let mut new_args = Vec::new();
    let mut used_args = vec![false; arguments.len()];

    let mut process_arg_kind = |arg_kind: &_, spec| {
        let index = match *arg_kind {
            ArgKind::Positional(index) => {
                if index >= arguments.len() {
                    return Err(format!("invalid positional argument index: {}", index));
                }

                arg_indices.push((index, spec));
                index
            }
            ArgKind::Named(ref ident) => match named_args_positions.entry(ident.name().to_owned()) {
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

        Ok(())
    };

    for piece in pieces {
        match piece {
            Piece::StdFmt { arg_kind_position, arg_kind_width, arg_kind_precision } => {
                for arg_kind in [Some(arg_kind_position), arg_kind_width, arg_kind_precision].iter().flatten() {
                    process_arg_kind(arg_kind, None)?;
                }
            }
            Piece::CustomFmt { arg_kind, spec } => process_arg_kind(&arg_kind, Some(spec))?,
        }
    }

    if let Some((index, (arg, _))) = arguments.iter().zip(&used_args).enumerate().find(|(_, (_, &used))| !used) {
        return match &arg.ident {
            Some(name) => Err(format!("named argument `{}` not used", name).into()),
            None => Err(format!("positional argument {} not used", index).into()),
        };
    }

    Ok(ProcessedPieces { arg_indices, new_args })
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_tokens() -> Result<(), Box<dyn std::error::Error>> {
        let s1 = r#"
            crate,
            [::std::format!], [],
            [("format string"), (5==3), (()), (Custom(1f64.abs())), (std::format!("{:?}, {}", (3, 4), 5)),
            ((z) = (::std::f64::MAX)), ((r) = (&1 + 4)), ((b) = (2)), ((c) = (Custom(6))), ((e) = ({ g }))]
        "#;

        let s2 = r##"
            crate,
            [::std::format!], [std::io::stdout().lock()],
            [(r#"format string"#), (5==3), (()), (Custom(1f64.abs())), (std::format!("{:?}, {}", (3, 4), 5)),
            ((z) = (::std::f64::MAX)), ((r) = (&1 + 4)), ((b) = (2)), ((c) = (Custom(6))), ((e) = ({ g }))]
        "##;

        let result_format_string = "format string";
        let result_crate_ident = "crate";
        let result_root_macro = "::std::format!".parse::<TokenStream>()?.to_string();
        let results_first_arg = [None, Some("std::io::stdout().lock()".parse::<TokenStream>()?.to_string())];
        let result_argument_names = [None, None, None, None, Some("z"), Some("r"), Some("b"), Some("c"), Some("e")];

        let result_argument_exprs = [
            "(5==3)",
            "(())",
            "(Custom(1f64.abs()))",
            r#"(std::format!("{:?}, {}", (3, 4), 5))"#,
            "(::std::f64::MAX)",
            "(&1 + 4)",
            "(2)",
            "(Custom(6))",
            "({g})",
        ];

        for (s, result_first_arg) in [s1, s2].iter().zip(&results_first_arg) {
            let (format_string, parsed_input) = parse_tokens(s.parse()?).unwrap();

            assert_eq!(format_string, result_format_string);
            assert_eq!(parsed_input.crate_ident.to_string(), result_crate_ident);
            assert_eq!(parsed_input.root_macro.to_string(), result_root_macro);
            assert_eq!(parsed_input.first_arg.map(|x| x.to_string()), *result_first_arg);

            for ((arg, &result_name), &result_expr) in parsed_input.arguments.iter().zip(&result_argument_names).zip(&result_argument_exprs) {
                assert_eq!(arg.ident.as_ref().map(|x| x.to_string()), result_name.map(|x| x.to_string()));
                assert_eq!(arg.expr.to_string(), result_expr.parse::<TokenStream>()?.to_string());
            }
        }

        let err = parse_tokens("crate, [::std::format!], [], [(42)]".parse()?).unwrap_err();
        assert!(err.to_string().starts_with("compile_error"));
        assert_ne!(err.into_iter().last().unwrap().to_string(), "(\"invalid tokens\")");

        let err = parse_tokens(TokenStream::new()).unwrap_err();
        assert!(err.to_string().starts_with("compile_error"));
        assert_eq!(err.into_iter().last().unwrap().to_string(), "(\"invalid tokens\")");

        Ok(())
    }

    #[test]
    fn test_process_fmt() -> Result<(), Error> {
        #[rustfmt::skip]
        let data = [
            ("{ :}",            "{0}",             1, 1, Piece::CustomFmt { arg_kind: ArgKind::Positional(0),         spec: Spec::CompileTime("") }),
            ("{ : \t\r\n }",    "{0}",             1, 1, Piece::CustomFmt { arg_kind: ArgKind::Positional(0),         spec: Spec::CompileTime("") }),
            ("{ :\u{2000} }",   "{0}",             1, 1, Piece::CustomFmt { arg_kind: ArgKind::Positional(0),         spec: Spec::CompileTime("") }),
            ("{ : : : }",       "{0}",             1, 1, Piece::CustomFmt { arg_kind: ArgKind::Positional(0),         spec: Spec::CompileTime(" : :") }),
            ("{ : <: :> }",     "{0}",             1, 1, Piece::CustomFmt { arg_kind: ArgKind::Positional(0),         spec: Spec::CompileTime(" <: :>") }),
            ("{ : éà }" ,       "{0}",             1, 1, Piece::CustomFmt { arg_kind: ArgKind::Positional(0),         spec: Spec::CompileTime(" éà") }),
            ("{ : <éà> }" ,     "{0}",             1, 1, Piece::CustomFmt { arg_kind: ArgKind::Positional(0),         spec: Spec::CompileTime(" <éà>") }),
            ("{3 :%a }",        "{0}",             0, 1, Piece::CustomFmt { arg_kind: ArgKind::Positional(3),         spec: Spec::CompileTime("%a") }),
            ("{éà :%a}",        "{0}",             0, 1, Piece::CustomFmt { arg_kind: ArgKind::Named(Id::new("éà")?), spec: Spec::CompileTime("%a") }),
            ("{éà :<<<>>%a><}", "{0}",             0, 1, Piece::CustomFmt { arg_kind: ArgKind::Named(Id::new("éà")?), spec: Spec::CompileTime("<<<>>%a><") }),
            ("{ :<>}",          "{0}",             1, 1, Piece::CustomFmt { arg_kind: ArgKind::Positional(0),         spec: Spec::Runtime("") }),
            ("{ :<> \t\r\n }",  "{0}",             1, 1, Piece::CustomFmt { arg_kind: ArgKind::Positional(0),         spec: Spec::Runtime("") }),
            ("{ :<>\u{2000} }", "{0}",             1, 1, Piece::CustomFmt { arg_kind: ArgKind::Positional(0),         spec: Spec::Runtime("") }),
            ("{ :< : :> }",     "{0}",             1, 1, Piece::CustomFmt { arg_kind: ArgKind::Positional(0),         spec: Spec::Runtime(" : :") }),
            ("{ :<%a> }",       "{0}",             1, 1, Piece::CustomFmt { arg_kind: ArgKind::Positional(0),         spec: Spec::Runtime("%a") }),
            ("{3 :<%a> }",      "{0}",             0, 1, Piece::CustomFmt { arg_kind: ArgKind::Positional(3),         spec: Spec::Runtime("%a") }),
            ("{éà :<%a>}",      "{0}",             0, 1, Piece::CustomFmt { arg_kind: ArgKind::Named(Id::new("éà")?), spec: Spec::Runtime("%a") }),
            ("{éà :<<<>>%a>}",  "{0}",             0, 1, Piece::CustomFmt { arg_kind: ArgKind::Named(Id::new("éà")?), spec: Spec::Runtime("<<>>%a") }),
            ("{}",              "{0}",             1, 1, Piece::StdFmt { arg_kind_position: ArgKind::Positional(0),         arg_kind_width: None,                                arg_kind_precision: None }),
            ("{:?}",            "{0:?}",           1, 1, Piece::StdFmt { arg_kind_position: ArgKind::Positional(0),         arg_kind_width: None,                                arg_kind_precision: None }),
            ("{3:? }",          "{0:?}",           0, 1, Piece::StdFmt { arg_kind_position: ArgKind::Positional(3),         arg_kind_width: None,                                arg_kind_precision: None }),
            ("{éà}",            "{0}",             0, 1, Piece::StdFmt { arg_kind_position: ArgKind::Named(Id::new("éà")?), arg_kind_width: None,                                arg_kind_precision: None }),
            ("{: ^+#03.6? }",   "{0: ^+#03.6?}",   1, 1, Piece::StdFmt { arg_kind_position: ArgKind::Positional(0),         arg_kind_width: None,                                arg_kind_precision: None }),
            ("{: ^+#0a$.6? }",  "{0: ^+#01$.6?}",  1, 2, Piece::StdFmt { arg_kind_position: ArgKind::Positional(0),         arg_kind_width: Some(ArgKind::Named(Id::new("a")?)), arg_kind_precision: None }),
            ("{: ^+#03.6$? }",  "{0: ^+#03.1$?}",  1, 2, Piece::StdFmt { arg_kind_position: ArgKind::Positional(0),         arg_kind_width: None,                                arg_kind_precision: Some(ArgKind::Positional(6)) }),
            ("{: ^+#03$.d$? }", "{0: ^+#01$.2$?}", 1, 3, Piece::StdFmt { arg_kind_position: ArgKind::Positional(0),         arg_kind_width: Some(ArgKind::Positional(3)),        arg_kind_precision: Some(ArgKind::Named(Id::new("d")?)) }),
            ("{: ^+#0z$.*? }",  "{0: ^+#01$.2$?}", 2, 3, Piece::StdFmt { arg_kind_position: ArgKind::Positional(1),         arg_kind_width: Some(ArgKind::Named(Id::new("z")?)), arg_kind_precision: Some(ArgKind::Positional(0)) }),
            ("{2: ^+#03$.*? }", "{0: ^+#01$.2$?}", 1, 3, Piece::StdFmt { arg_kind_position: ArgKind::Positional(2),         arg_kind_width: Some(ArgKind::Positional(3)),        arg_kind_precision: Some(ArgKind::Positional(0)) }),
            ("{:1$? }",         "{0:1$?}",         1, 2, Piece::StdFmt { arg_kind_position: ArgKind::Positional(0),         arg_kind_width: Some(ArgKind::Positional(1)),        arg_kind_precision: None }),
            ("{:.2$? }",        "{0:.1$?}",        1, 2, Piece::StdFmt { arg_kind_position: ArgKind::Positional(0),         arg_kind_width: None,                                arg_kind_precision: Some(ArgKind::Positional(2)) }),
            ("{:.*? }",         "{0:.1$?}",        2, 2, Piece::StdFmt { arg_kind_position: ArgKind::Positional(1),         arg_kind_width: None,                                arg_kind_precision: Some(ArgKind::Positional(0)) }),
            ("{a:.*? }",        "{0:.1$?}",        1, 2, Piece::StdFmt { arg_kind_position: ArgKind::Named(Id::new("a")?),  arg_kind_width: None,                                arg_kind_precision: Some(ArgKind::Positional(0)) }),
        ];

        for &(fmt, result_new_format_string, result_current_positional_index, result_new_current_index, ref result_piece) in &data {
            let mut new_format_string = String::new();
            let mut current_positional_index = 0;
            let mut new_current_index = 0;

            let piece = process_fmt(fmt, &mut current_positional_index, &mut new_format_string, &mut new_current_index)?;

            assert_eq!(new_format_string, result_new_format_string);
            assert_eq!(current_positional_index, result_current_positional_index);
            assert_eq!(new_current_index, result_new_current_index);
            assert_eq!(piece, *result_piece);
        }

        assert_eq!(process_fmt("{: ", &mut 0, &mut String::new(), &mut 0).unwrap_err(), "invalid format string");
        assert_eq!(process_fmt("{0éà0 :%a}", &mut 0, &mut String::new(), &mut 0).unwrap_err(), "invalid format string");
        assert_eq!(process_fmt("{0éà0}", &mut 0, &mut String::new(), &mut 0).unwrap_err(), "invalid format string");
        assert_eq!(process_fmt("{0:.}", &mut 0, &mut String::new(), &mut 0).unwrap_err(), "invalid count in format string");
        assert_eq!(process_fmt("{_:?}", &mut 0, &mut String::new(), &mut 0).unwrap_err(), "invalid argument: argument name cannot be a single underscore");

        Ok(())
    }

    #[test]
    fn test_parse_format_string() -> Result<(), Error> {
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

        let (new_format_string, pieces) = parse_format_string(format_string)?;

        assert_eq!(new_format_string, result_new_format_string);
        assert_eq!(pieces, result_pieces);

        Ok(())
    }

    #[test]
    fn test_process_pieces() -> Result<(), Error> {
        let create_argument = |name: Option<&str>| {
            let expr = Group::new(Delimiter::Parenthesis, TokenStream::new());
            Argument { ident: name.map(|x| x.to_owned()), expr }
        };

        let pieces = vec![
            Piece::StdFmt { arg_kind_position: ArgKind::Named(Id::new("h")?), arg_kind_width: None, arg_kind_precision: None },
            Piece::CustomFmt { arg_kind: ArgKind::Named(Id::new("h")?), spec: Spec::CompileTime("%z") },
            Piece::StdFmt { arg_kind_position: ArgKind::Positional(1), arg_kind_width: None, arg_kind_precision: None },
            Piece::StdFmt { arg_kind_position: ArgKind::Named(Id::new("a")?), arg_kind_width: None, arg_kind_precision: None },
            Piece::StdFmt { arg_kind_position: ArgKind::Positional(3), arg_kind_width: None, arg_kind_precision: None },
            Piece::StdFmt { arg_kind_position: ArgKind::Named(Id::new("b")?), arg_kind_width: None, arg_kind_precision: None },
            Piece::StdFmt { arg_kind_position: ArgKind::Positional(1), arg_kind_width: None, arg_kind_precision: Some(ArgKind::Positional(0)) },
            Piece::StdFmt { arg_kind_position: ArgKind::Positional(3), arg_kind_width: Some(ArgKind::Named(Id::new("g")?)), arg_kind_precision: None },
        ];

        let arguments = [create_argument(None), create_argument(Some("a")), create_argument(Some("b")), create_argument(Some("c"))];

        let result_arg_indices =
            [(4, None), (4, Some(Spec::CompileTime("%z"))), (1, None), (1, None), (3, None), (2, None), (1, None), (0, None), (3, None), (5, None)];

        let result_new_args = ["h", "g"];

        let processed_pieces = process_pieces(pieces, &arguments)?;
        assert_eq!(processed_pieces.arg_indices, result_arg_indices);
        assert_eq!(processed_pieces.new_args, result_new_args);

        assert_eq!(process_pieces(vec![], &[create_argument(Some("a")), create_argument(Some("a"))]).unwrap_err(), "duplicate argument named `a`");
        assert_eq!(process_pieces(vec![], &[create_argument(None)]).unwrap_err(), "positional argument 0 not used");
        assert_eq!(process_pieces(vec![], &[create_argument(Some("a"))]).unwrap_err(), "named argument `a` not used");

        assert_eq!(
            process_pieces(vec![], &[create_argument(Some("é")), create_argument(None)]).unwrap_err(),
            "positional arguments cannot follow named arguments"
        );

        assert_eq!(
            process_pieces(vec![Piece::CustomFmt { arg_kind: ArgKind::Positional(0), spec: Spec::CompileTime("") }], &[]).unwrap_err(),
            "invalid positional argument index: 0"
        );

        Ok(())
    }
}
