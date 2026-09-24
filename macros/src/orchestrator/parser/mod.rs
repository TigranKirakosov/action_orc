use super::ast::*;

use proc_macro2::{Delimiter, Span, TokenStream as TokenStream2, TokenTree};
use winnow::{
    ModalResult, Parser,
    combinator::{alt, opt, preceded, repeat, separated},
    error::{ContextError, ErrMode, ParserError, StrContext, StrContextValue},
    stream::Stream,
};

use combinators::*;
use error::ParseError;

mod combinators;
mod error;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy)]
pub(super) struct SpanInfo {
    pub(super) span: Span,
    pub(super) at_call_site: bool,
}

pub(crate) fn current_span(input: &[TokenTree]) -> SpanInfo {
    match input.first().map(|tt| tt.span()) {
        Some(span) => SpanInfo {
            span,
            at_call_site: false,
        },
        None => SpanInfo {
            span: Span::call_site(),
            at_call_site: true,
        },
    }
}

pub(super) fn parse(stream: TokenStream2) -> Result<SyntaxTree, syn::Error> {
    let tokens: Vec<TokenTree> = stream.into_iter().collect();
    let mut input = tokens.as_slice();

    match ast.parse_next(&mut input) {
        Ok(root) => Ok(root),
        Err(err_mode) => {
            let parse_err = err_mode
                .into_inner()
                .expect("Parser failed or encountered incomplete input state");

            Err(syn::Error::new(
                parse_err.span_info.span,
                format!("Failed to parse pipeline: {}", parse_err.inner),
            ))
        }
    }
}

fn ast<'a>(input: &mut &'a [TokenTree]) -> ModalResult<SyntaxTree, ParseError<'a>> {
    let graphs = separated(0.., graph, punct(';')).parse_next(input)?;

    Ok(SyntaxTree { graphs })
}

/// (a: A | b: B) -> C -> [d];
fn graph<'a>(input: &mut &'a [TokenTree]) -> ModalResult<Graph, ParseError<'a>> {
    let start_span = current_span(input);
    let entry = node_expr.parse_next(input)?;
    let conns = repeat(0.., preceded(arrow, node_expr)).parse_next(input)?;
    Ok(Graph {
        entry,
        conns,
        span_info: start_span,
    })
}

fn node_expr<'a>(input: &mut &'a [TokenTree]) -> ModalResult<NodeExpr, ParseError<'a>> {
    alt((binding, group, decl, expr_block)).parse_next(input)
}

/// 1) a: A
/// 2) A
fn decl<'a>(input: &mut &'a [TokenTree]) -> ModalResult<NodeExpr, ParseError<'a>> {
    alt((
        // var: scenario::Entering<Dungeon>
        (ident, punct(':'), type_path).map(|(var, _, typ)| Declartaion {
            var: Some(var),
            typ,
        }),
        // Type tag, e.g. `Enter` (must be strictly uppercase)
        type_path
            .verify(|typ: &syn::Type| {
                let type_string = quote::quote!(#typ).to_string();
                type_string
                    .chars()
                    .next()
                    .map_or(false, |c| !c.is_lowercase())
            })
            .map(|typ| Declartaion { var: None, typ }),
    ))
    .map(NodeExpr::Declaration)
    .parse_next(input)
}

// [var]
fn binding<'a>(input: &mut &'a [TokenTree]) -> ModalResult<NodeExpr, ParseError<'a>> {
    let expr = enclosed(Delimiter::Bracket, ident, "var binding")
        .map(NodeExpr::Binding)
        .parse_next(input)?;

    Ok(expr)
}

fn expr_block<'a>(input: &mut &'a [TokenTree]) -> ModalResult<NodeExpr, ParseError<'a>> {
    punct('@').parse_next(input)?;
    // Optional bound annotation: `[Fork; Join]` or `[Fork]`
    let explicit_bounds = opt(enclosed(
        Delimiter::Bracket,
        |input| {
            let i = bound.parse_next(input)?;
            let o = match opt(punct(';')).parse_next(input)? {
                Some(()) => bound.parse_next(input)?,
                None => i,
            };
            Ok((i, o))
        },
        "bounds",
    ))
    .parse_next(input)?;

    let expr = struct_expr.parse_next(input)?;
    Ok(NodeExpr::Expression {
        expr,
        explicit_bounds,
    })
}

fn bound<'a>(input: &mut &'a [TokenTree]) -> ModalResult<Bound, ParseError<'a>> {
    let checkpoint = input.checkpoint();
    match input.first() {
        Some(TokenTree::Ident(id)) if id == "Fork" => {
            *input = &input[1..];
            Ok(Bound::Fork)
        }
        Some(TokenTree::Ident(id)) if id == "Join" => {
            *input = &input[1..];
            Ok(Bound::Join)
        }
        _ => {
            input.reset(&checkpoint);
            Err(ErrMode::Backtrack(ParseError {
                span_info: current_span(input),
                inner: ContextError::from_input(input),
                input: *input,
            }))
        }
    }
}

/// 1) (A | B | C)
/// 2) (A, B, C)
/// 3) (A ? B ? C)
fn group<'a>(input: &mut &'a [TokenTree]) -> ModalResult<NodeExpr, ParseError<'a>> {
    let group_span = current_span(input);
    let expr = enclosed(
        Delimiter::Parenthesis,
        move |i| {
            alt((
                fork_block(group_span.clone()),
                sequence_block(group_span.clone()),
                selection_block(group_span.clone()),
            ))
            .parse_next(i)
        },
        "node group",
    )
    .map(NodeExpr::Group)
    .parse_next(input)?;

    Ok(expr)
}

fn fork_block<'a>(
    span_info: SpanInfo,
) -> impl FnMut(&mut &'a [TokenTree]) -> ModalResult<GroupBlock, ParseError<'a>> {
    move |input: &mut &'a [TokenTree]| {
        let checkpoint = input.checkpoint();
        match separated(2.., graph, punct('|')).parse_next(input) {
            Ok(graphs) => {
                let block = GroupBlock {
                    mode: SchedulingMode::Fork,
                    graphs,
                    span_info,
                };

                Ok(block)
            }
            Err(err) => {
                input.reset(&checkpoint);
                Err(err)
            }
        }
    }
}

fn sequence_block<'a>(
    span_info: SpanInfo,
) -> impl FnMut(&mut &'a [TokenTree]) -> ModalResult<GroupBlock, ParseError<'a>> {
    move |input: &mut &'a [TokenTree]| {
        let checkpoint = input.checkpoint();

        match (separated(2.., graph, punct(',')), opt(punct(',')))
            .map(|(graphs, _)| graphs)
            .parse_next(input)
        {
            Ok(graphs) => {
                let block = GroupBlock {
                    mode: SchedulingMode::Sequence,
                    graphs,
                    span_info,
                };

                Ok(block)
            }
            Err(err) => {
                input.reset(&checkpoint);
                Err(err)
            }
        }
    }
}

fn selection_block<'a>(
    span_info: SpanInfo,
) -> impl FnMut(&mut &'a [TokenTree]) -> ModalResult<GroupBlock, ParseError<'a>> {
    move |input: &mut &'a [TokenTree]| {
        let checkpoint = input.checkpoint();

        match separated(2.., graph, punct('?')).parse_next(input) {
            Ok(graphs) => Ok(GroupBlock {
                mode: SchedulingMode::Selection,
                graphs,
                span_info,
            }),
            Err(err) => {
                input.reset(&checkpoint);
                Err(err)
            }
        }
    }
}

/// ->
fn arrow<'a>(input: &mut &'a [TokenTree]) -> ModalResult<(), ParseError<'a>> {
    (punct('-'), punct('>'))
        .context(StrContext::Expected(StrContextValue::Description(
            "right arrow (->)",
        )))
        .void()
        .parse_next(input)
}

impl PartialEq for SpanInfo {
    fn eq(&self, other: &Self) -> bool {
        self.at_call_site == other.at_call_site
    }
}
