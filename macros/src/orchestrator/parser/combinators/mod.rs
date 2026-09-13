use proc_macro2::{Delimiter, TokenStream as TokenStream2, TokenTree};
use syn::{Ident, Type};

use winnow::{
    ModalResult, Parser,
    error::{ContextError, ErrMode, ParserError, StrContext, StrContextValue},
    token::any,
};

use super::{ParseError, SpanInfo, TokenStreamParseExt, current_span};

/// Matches [TokenTree::Group] with [Delimiter],
/// parses its inner stream completely using `parser`, and handles span mapping
pub(super) fn enclosed<'a, O, F>(
    delim: Delimiter,
    mut parser: F,
    description: &'static str,
) -> impl FnMut(&mut &'a [TokenTree]) -> ModalResult<O, ParseError<'a>>
where
    F: for<'b> FnMut(&mut &'b [TokenTree]) -> ModalResult<O, ParseError<'b>>,
{
    move |input: &mut &'a [TokenTree]| {
        let (inner_stream, group_span) = any
            .verify_map(|tt| {
                let group_span = current_span(std::slice::from_ref(&tt));
                match tt {
                    TokenTree::Group(g) if g.delimiter() == delim => Some((g.stream(), group_span)),
                    _ => None,
                }
            })
            .context(StrContext::Expected(StrContextValue::Description(
                description,
            )))
            .parse_next(input)?;

        inner_stream.parse_nested(group_span, input, &mut parser)
    }
}

const TYPE_PATH_TERMINATORS: [char; 4] = [':', ';', '|', ','];
/// Parses any fully qualified Rust type with generics (e.g., `my_mod::Entering<Main>`)
/// Will stop on punctuation within [TYPE_PATH_TERMINATORS] or on arrow `->`
pub(super) fn type_path<'a>(input: &mut &'a [TokenTree]) -> ModalResult<Type, ParseError<'a>> {
    let mut type_len = 0;
    for tt in input.iter() {
        match tt {
            TokenTree::Punct(p) => {
                let c = p.as_char();
                if TYPE_PATH_TERMINATORS.contains(&c) {
                    break;
                }
                // Check arrow
                if c == '-' {
                    if let Some(TokenTree::Punct(next_p)) = input.get(type_len + 1) {
                        if next_p.as_char() == '>' {
                            break;
                        }
                    }
                }
            }
            _ => {}
        }
        type_len += 1;
    }

    if type_len == 0 {
        return Err(ErrMode::Backtrack(ParseError::from_input(input)));
    }

    let type_tokens = &input[..type_len];

    let stream: TokenStream2 = type_tokens.iter().cloned().collect();

    let typ = syn::parse2(stream).map_err(|syn_err| {
        ErrMode::Backtrack(ParseError {
            span_info: SpanInfo {
                span: syn_err.span(),
                at_call_site: false,
            },
            inner: ContextError::from_input(input),
            input: *input,
        })
    })?;

    *input = &input[type_len..];

    Ok(typ)
}

pub(super) fn struct_expr<'a>(
    input: &mut &'a [TokenTree],
) -> ModalResult<syn::Expr, ParseError<'a>> {
    let start_input = *input;

    // Scan until hitting a terminal delimiter or an arrow `->`
    let mut struct_len = 0;
    for tt in input.iter() {
        if let TokenTree::Punct(p) = tt {
            let c = p.as_char();
            if TYPE_PATH_TERMINATORS.contains(&c) {
                break;
            }
            if c == '-' {
                if let Some(TokenTree::Punct(next_p)) = input.get(struct_len + 1) {
                    if next_p.as_char() == '>' {
                        break;
                    }
                }
            }
        }
        struct_len += 1;
    }

    // Terminated instantly
    if struct_len == 0 {
        return Err(ErrMode::Backtrack(ParseError::from_input(input)));
    }

    *input = &input[struct_len..];

    let mut constructor_len = 0;
    if let Some(TokenTree::Group(g)) = input.first()
        && g.delimiter() == Delimiter::Brace
    {
        *input = &input[1..];
        constructor_len += 1;
    }

    // Hand over parsing to syn to preserve token spans
    let total_len = struct_len + constructor_len;
    let expr_tokens = &start_input[..total_len];
    let stream: TokenStream2 = expr_tokens.iter().cloned().collect();

    let parsed_expr = syn::parse2::<syn::Expr>(stream).map_err(|syn_err| {
        ErrMode::Backtrack(ParseError {
            span_info: SpanInfo {
                span: syn_err.span(),
                at_call_site: false,
            },
            inner: ContextError::from_input(input),
            input: *input,
        })
    })?;

    Ok(parsed_expr)
}

pub(super) fn punct<'a>(
    expected: char,
) -> impl FnMut(&mut &'a [TokenTree]) -> ModalResult<(), ParseError<'a>> {
    move |input: &mut &'a [TokenTree]| {
        any.verify(|tt| match tt {
            TokenTree::Punct(p) => p.as_char() == expected,
            _ => false,
        })
        .context(StrContext::Expected(StrContextValue::CharLiteral(expected)))
        .void()
        .parse_next(input)
    }
}

pub(super) fn ident<'a>(input: &mut &'a [TokenTree]) -> ModalResult<Ident, ParseError<'a>> {
    any.verify_map(|tt| match tt {
        TokenTree::Ident(ident) => Some(ident.clone()),
        _ => None,
    })
    .context(StrContext::Expected(StrContextValue::Description(
        "identifier",
    )))
    .parse_next(input)
}
