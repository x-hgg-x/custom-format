//! Module containing procedural macros common code.

mod output;
mod parse;
mod process;
mod utils;

use output::*;
use process::*;

#[cfg(not(test))]
use proc_macro::{Delimiter, Group, Ident, Literal, Punct, Spacing, Span, TokenStream, TokenTree};
#[cfg(test)]
use proc_macro2::{Delimiter, Group, Ident, Literal, Punct, Spacing, Span, TokenStream, TokenTree};

/// Error type for the procedural macro
type Error = std::borrow::Cow<'static, str>;

/// Separator for custom format specifier
const CUSTOM_SEPARATOR: &str = " :";

/// Proc-macro argument
#[derive(Debug)]
struct Argument {
    /// Optional identifier name
    ident: Option<String>,
    /// Expression
    expr: Group,
}

/// Parsed input elements
#[derive(Debug)]
struct ParsedInput {
    /// Crate identifier (`$crate`)
    crate_ident: Ident,
    /// Root macro tokens
    root_macro: TokenStream,
    /// First argument tokens
    first_arg: Option<TokenStream>,
    /// List of proc-macro arguments
    arguments: Vec<Argument>,
    /// Span of the format string
    span: Span,
}

/// Identifier normalized in Unicode NFC
#[derive(Debug, PartialEq)]
struct Id<'a>(&'a str);

impl<'a> Id<'a> {
    /// Construct a new [`Id`] value
    fn new(name: &'a str) -> Result<Self, String> {
        #[cfg(not(test))]
        let normalized_name = Ident::new(name, Span::call_site()).to_string();
        #[cfg(test)]
        let normalized_name = unicode_normalization::UnicodeNormalization::nfc(name).collect::<String>();

        if name == normalized_name {
            Ok(Self(name))
        } else {
            Err(format!("identifiers in format string must be normalized in Unicode NFC (`{:?}` != `{:?}`)", name, normalized_name))
        }
    }

    /// Return the identifier value
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
    /// Format specifier checked at compile-time
    CompileTime(&'a str),
    /// Format specifier checked at runtime
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

/// Processed elements of the format string pieces
#[derive(Debug)]
struct ProcessedPieces<'a> {
    /// Argument indices associated to the format string pieces, with custom format specifiers if applicable
    arg_indices: Vec<(usize, Option<Spec<'a>>)>,
    /// List of new arguments to be added from captured identifiers in the format string, if not already existing
    new_args: Vec<&'a str>,
}

/// Create tokens representing a compilation error
fn compile_error(msg: &str, span: Span) -> TokenStream {
    let mut tokens = vec![
        TokenTree::from(Ident::new("compile_error", span)),
        TokenTree::from(Punct::new('!', Spacing::Alone)),
        TokenTree::from(Group::new(Delimiter::Parenthesis, TokenTree::from(Literal::string(msg)).into())),
    ];

    for t in &mut tokens {
        t.set_span(span);
    }

    tokens.into_iter().collect()
}

/// Main function, working with both [`proc_macro::TokenStream`] and `proc_macro2::TokenStream`
pub(crate) fn fmt(input: TokenStream) -> TokenStream {
    let (format_string, parsed_input) = match parse_tokens(input) {
        Err(compile_error) => return compile_error,
        Ok(x) => x,
    };

    let (new_format_string, pieces) = match parse_format_string(&format_string) {
        Err(error) => return compile_error(&error, parsed_input.span),
        Ok(x) => x,
    };

    let processed_pieces = match process_pieces(pieces, &parsed_input.arguments) {
        Err(error) => return compile_error(&error, parsed_input.span),
        Ok(x) => x,
    };

    compute_output(parsed_input, &new_format_string, processed_pieces)
}
