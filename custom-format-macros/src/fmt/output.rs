//! Functions used for computing output tokens.

use super::*;

use std::iter::FromIterator;

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

/// Compute output Rust code
pub(super) fn compute_output(parsed_input: ParsedInput, new_format_string: &str, processed_pieces: ProcessedPieces) -> TokenStream {
    let crate_ident = parsed_input.crate_ident;

    let arg_count = parsed_input.arguments.len() + processed_pieces.new_args.len();
    let arg_idents: Vec<TokenTree> = (0..arg_count).map(|index| Ident::new(&format!("arg{}", index), Span::call_site()).into()).collect();

    let mut output = Vec::<TokenTree>::new();

    output.push(Ident::new("match", Span::call_site()).into());

    output.push(TokenTree::from(Group::new(Delimiter::Parenthesis, {
        let mut exprs = Vec::<TokenTree>::new();

        for arg in parsed_input.arguments {
            exprs.push(Punct::new('&', Spacing::Alone).into());
            exprs.push(arg.expr.clone().into());
            exprs.push(Punct::new(',', Spacing::Alone).into());
        }

        for name in processed_pieces.new_args {
            exprs.push(Punct::new('&', Spacing::Alone).into());
            exprs.push(Ident::new(name, parsed_input.span).into());
            exprs.push(Punct::new(',', Spacing::Alone).into());
        }

        exprs.pop();
        exprs.into_iter().collect()
    })));

    output.push(TokenTree::from(Group::new(Delimiter::Brace, {
        let mut block = Vec::<TokenTree>::new();

        block.push(TokenTree::from(Group::new(Delimiter::Parenthesis, {
            let mut arm_pat = Vec::<TokenTree>::new();

            for arg_ident in &arg_idents {
                arm_pat.push(arg_ident.clone());
                arm_pat.push(Punct::new(',', Spacing::Alone).into());
            }

            arm_pat.pop();
            arm_pat.into_iter().collect()
        })));

        block.push(Punct::new('=', Spacing::Joint).into());
        block.push(Punct::new('>', Spacing::Alone).into());

        block.extend(parsed_input.root_macro);

        block.push(TokenTree::from(Group::new(Delimiter::Parenthesis, {
            let mut fmt_args = Vec::<TokenTree>::new();

            if let Some(first_arg) = parsed_input.first_arg {
                fmt_args.extend(first_arg);
                fmt_args.push(Punct::new(',', Spacing::Alone).into());
            }

            fmt_args.push(TokenTree::from(Literal::string(new_format_string)));

            for (index, spec) in processed_pieces.arg_indices {
                fmt_args.push(Punct::new(',', Spacing::Alone).into());

                match spec {
                    None => fmt_args.push(arg_idents[index].clone()),
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

                        fmt_args.push(TokenTree::from(Group::new(
                            Delimiter::Parenthesis,
                            TokenStream::from_iter(vec![spec_literal.into(), Punct::new(',', Spacing::Alone).into(), arg_idents[index].clone()]),
                        )));
                    }
                }
            }

            fmt_args.into_iter().collect()
        })));

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

        let result = concat!(
            r#"match (&("0"), &("1"), &("2"), &("3"), &h, &g) { (arg0, arg1, arg2, arg3, arg4, arg5) => "#,
            r#"::std::println!("{0}, {1}, {2}, {3}, {4}, {5}, {6:.7$}, {8:9$}", arg4, "#,
            r#"crate::custom_formatter!("%z", arg4), arg1, arg1, arg3, "#,
            r#"crate::runtime::CustomFormatter::new("%x", arg2), arg1, arg0, arg3, arg5), }"#
        );

        let output = compute_output(
            ParsedInput {
                crate_ident: Ident::new("crate", Span::call_site()),
                root_macro: "::std::println!".parse()?,
                first_arg: None,
                arguments,
                span: Span::call_site(),
            },
            new_format_string,
            ProcessedPieces { arg_indices, new_args },
        );

        assert_eq!(output.to_string(), result.parse::<TokenStream>()?.to_string());

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
