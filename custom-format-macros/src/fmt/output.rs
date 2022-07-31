//! Functions used for computing output tokens.

use super::*;

/// Push `::` to the list of token trees
fn push_two_colons(v: &mut Vec<TokenTree>) {
    v.push(Punct::new(':', Spacing::Joint).into());
    v.push(Punct::new(':', Spacing::Alone).into());
}

/// Push `$crate::custom_formatter!` to the list of token trees
fn push_compile_time_formatter(v: &mut Vec<TokenTree>, crate_ident: &Ident) {
    v.push(crate_ident.clone().into());
    push_two_colons(v);
    v.push(Ident::new("custom_formatter", Span::call_site()).into());
    v.push(Punct::new('!', Spacing::Alone).into());
}

/// Push `$crate::runtime::CustomFormatter::new` to the list of token trees
fn push_runtime_formatter(v: &mut Vec<TokenTree>, crate_ident: &Ident) {
    v.push(crate_ident.clone().into());
    push_two_colons(v);
    v.push(Ident::new("runtime", Span::call_site()).into());
    push_two_colons(v);
    v.push(Ident::new("CustomFormatter", Span::call_site()).into());
    push_two_colons(v);
    v.push(Ident::new("new", Span::call_site()).into());
}

/// Push the whole macro call to the list of token trees
fn push_macro_call(
    v: &mut Vec<TokenTree>,
    crate_ident: Ident,
    root_macro: TokenStream,
    first_arg: Option<TokenStream>,
    new_format_string: &str,
    arg_indices: Vec<(usize, Option<Spec>)>,
    args: &[TokenStream],
) {
    v.extend(root_macro);

    v.push(TokenTree::from(Group::new(Delimiter::Parenthesis, {
        let mut fmt_args = Vec::<TokenTree>::new();

        if let Some(first_arg) = first_arg {
            fmt_args.extend(first_arg);
            fmt_args.push(Punct::new(',', Spacing::Alone).into());
        }

        fmt_args.push(TokenTree::from(Literal::string(new_format_string)));

        for (index, spec) in arg_indices {
            fmt_args.push(Punct::new(',', Spacing::Alone).into());

            match spec {
                None => fmt_args.extend(args[index].clone()),
                Some(spec) => {
                    let spec_literal = match spec {
                        Spec::CompileTime(spec) => {
                            push_compile_time_formatter(&mut fmt_args, &crate_ident);
                            Literal::string(spec)
                        }
                        Spec::Runtime(spec) => {
                            push_runtime_formatter(&mut fmt_args, &crate_ident);
                            Literal::string(spec)
                        }
                    };

                    fmt_args.push(TokenTree::from(Group::new(Delimiter::Parenthesis, {
                        let mut stream = vec![spec_literal.into(), Punct::new(',', Spacing::Alone).into()];
                        stream.extend(args[index].clone());
                        stream.into_iter().collect()
                    })));
                }
            }
        }

        fmt_args.into_iter().collect()
    })));
}

/// Compute output Rust code
pub(super) fn compute_output(parsed_input: ParsedInput, new_format_string: &str, processed_pieces: ProcessedPieces) -> TokenStream {
    let ParsedInput { crate_ident, root_macro, first_arg, arguments, span } = parsed_input;
    let ProcessedPieces { arg_indices, new_args } = processed_pieces;

    let arg_exprs: Vec<TokenStream> = arguments
        .into_iter()
        .map(|arg| arg.expr.into())
        .chain(new_args.into_iter().map(|name| Ident::new(name, span).into()))
        .map(|tt| vec![TokenTree::from(Punct::new('&', Spacing::Alone)), tt].into_iter().collect())
        .collect();

    let arg_idents: Vec<TokenStream> =
        (0..arg_exprs.len()).map(|index| TokenTree::from(Ident::new(&format!("arg{}", index), Span::call_site())).into()).collect();

    // Don't use a `match` for the `format_args!` macro because it creates temporary values
    if let Some(TokenTree::Ident(ident)) = root_macro.clone().into_iter().nth(5) {
        if &ident.to_string() == "format_args" {
            let mut output = Vec::new();
            push_macro_call(&mut output, crate_ident, root_macro, first_arg, new_format_string, arg_indices, &arg_exprs);
            return output.into_iter().collect();
        }
    }

    let mut output = vec![Ident::new("match", Span::call_site()).into()];

    output.push(TokenTree::from(Group::new(Delimiter::Parenthesis, {
        let mut exprs = Vec::new();

        for arg in arg_exprs {
            exprs.extend(arg);
            exprs.push(Punct::new(',', Spacing::Alone).into());
        }

        exprs.pop();
        exprs.into_iter().collect()
    })));

    output.push(TokenTree::from(Group::new(Delimiter::Brace, {
        let mut block = Vec::new();

        block.push(TokenTree::from(Group::new(Delimiter::Parenthesis, {
            let mut arm_pat = Vec::new();

            for arg_ident in &arg_idents {
                arm_pat.extend(arg_ident.clone());
                arm_pat.push(Punct::new(',', Spacing::Alone).into());
            }

            arm_pat.pop();
            arm_pat.into_iter().collect()
        })));

        block.push(Punct::new('=', Spacing::Joint).into());
        block.push(Punct::new('>', Spacing::Alone).into());

        push_macro_call(&mut block, crate_ident, root_macro, first_arg, new_format_string, arg_indices, &arg_idents);

        block.push(Punct::new(',', Spacing::Alone).into());

        block.into_iter().collect()
    })));

    output.into_iter().collect()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_compute_output() -> Result<(), Box<dyn std::error::Error>> {
        let create_argument = |name: Option<&str>, s| {
            let expr = Group::new(Delimiter::Parenthesis, TokenTree::from(Literal::string(s)).into());
            Argument { ident: name.map(|x| x.to_owned()), expr }
        };

        let data = [
            (
                "::std::println!",
                concat!(
                    r#"match (&("0"), &("1"), &("2"), &("3"), &h, &g) { (arg0, arg1, arg2, arg3, arg4, arg5) => "#,
                    r#"::std::println!("{0}, {1}, {2}, {3}, {4}, {5}, {6:.7$}, {8:9$}", arg4, "#,
                    r#"crate::custom_formatter!("%z", arg4), arg1, arg1, arg3, "#,
                    r#"crate::runtime::CustomFormatter::new("%x", arg2), arg1, arg0, arg3, arg5), }"#
                ),
            ),
            (
                "::core::format_args!",
                concat!(
                    r#"::core::format_args!("{0}, {1}, {2}, {3}, {4}, {5}, {6:.7$}, {8:9$}", &h, "#,
                    r#"crate::custom_formatter!("%z", &h), &("1"), &("1"), &("3"), "#,
                    r#"crate::runtime::CustomFormatter::new("%x", &("2")), "#,
                    r#"&("1"), &("0"), &("3"), &g)"#,
                ),
            ),
        ];

        for &(root_macro, result) in &data {
            let new_format_string = "{0}, {1}, {2}, {3}, {4}, {5}, {6:.7$}, {8:9$}";

            let arguments = vec![create_argument(None, "0"), create_argument(Some("a"), "1"), create_argument(Some("b"), "2"), create_argument(Some("c"), "3")];

            let arg_indices = vec![
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

            let new_args = vec!["h", "g"];

            let output = compute_output(
                ParsedInput {
                    crate_ident: Ident::new("crate", Span::call_site()),
                    root_macro: root_macro.parse()?,
                    first_arg: None,
                    arguments,
                    span: Span::call_site(),
                },
                new_format_string,
                ProcessedPieces { arg_indices, new_args },
            );

            assert_eq!(output.to_string(), result.parse::<TokenStream>()?.to_string());
        }

        Ok(())
    }

    #[test]
    fn test_compute_output_with_first_arg() -> Result<(), Box<dyn std::error::Error>> {
        let output = compute_output(
            ParsedInput {
                crate_ident: Ident::new("crate", Span::call_site()),
                root_macro: "::std::writeln!".parse()?,
                first_arg: Some("f".parse()?),
                arguments: vec![],
                span: Span::call_site(),
            },
            "string",
            ProcessedPieces { arg_indices: vec![], new_args: vec![] },
        );

        assert_eq!(output.to_string(), "match () { () => ::std::writeln!(f, \"string\"), }".parse::<TokenStream>()?.to_string());

        Ok(())
    }
}
