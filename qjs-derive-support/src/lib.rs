#[macro_use]
extern crate log;

#[cfg(test)]
#[macro_use]
extern crate matches;
#[cfg(test)]
#[macro_use]
extern crate if_chain;

use proc_macro2::{Delimiter, Group, Ident, Spacing, Span, TokenStream, TokenTree};
use quote::quote;
use syn::{
    braced, bracketed, parenthesized,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    token::{Brace, Bracket, Comma, FatArrow, Paren, RArrow},
    Expr, FnArg, Result, ReturnType,
};

pub struct Closure {
    captures: Option<Captures>,
    paren_token: Paren,
    params: Punctuated<FnArg, Comma>,
    output: Option<ReturnType>,
    fat_arrow_token: FatArrow,
    brace_token: Option<Brace>,
    script: TokenStream,
}

impl Parse for Closure {
    fn parse(input: ParseStream) -> Result<Self> {
        let captures = if input.peek(Bracket) {
            Some(input.parse()?)
        } else {
            None
        };
        let content;
        let paren_token = parenthesized!(content in input);
        let params = content.parse_terminated(FnArg::parse)?;
        let output = if input.peek(RArrow) {
            Some(input.parse()?)
        } else {
            None
        };
        let fat_arrow_token = input.parse()?;
        let (brace_token, script) = if input.peek(Brace) {
            let content;

            (Some(braced!(content in input)), content.parse()?)
        } else {
            (None, input.parse()?)
        };

        Ok(Closure {
            captures,
            paren_token,
            params,
            output,
            fat_arrow_token,
            brace_token,
            script,
        })
    }
}

pub struct Captures {
    bracket_token: Bracket,
    inputs: Punctuated<Ident, Comma>,
}

impl Parse for Captures {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;

        Ok(Captures {
            bracket_token: bracketed!(content in input),
            inputs: content.parse_terminated(Ident::parse)?,
        })
    }
}

pub fn closure(input: TokenStream) -> Result<TokenStream> {
    let c: Closure = syn::parse2(input)?;

    let expanded = quote! {};

    Ok(expanded)
}

pub enum Variable {
    Ident(Ident),
    Expr(Expr),
}

pub fn interpolate(input: TokenStream, vars: &mut Vec<Variable>) -> Result<TokenStream> {
    let mut output = TokenStream::new();
    let mut interpolating = None;

    for token in input {
        trace!("token: {:?}", token);

        match token {
            TokenTree::Punct(ref punct)
                if punct.as_char() == '#' && punct.spacing() == Spacing::Alone =>
            {
                interpolating = Some(punct.clone())
            }
            TokenTree::Ident(ref ident) if interpolating.is_some() => {
                let var = Ident::new(&format!("var{}", vars.len()), Span::call_site());

                output.extend(quote! {
                    __scope.#var
                });

                vars.push(Variable::Ident(ident.clone()));
            }
            TokenTree::Group(ref group)
                if interpolating.is_some() && group.delimiter() == Delimiter::Parenthesis =>
            {
                let var = Ident::new(&format!("var{}", vars.len()), Span::call_site());

                output.extend(quote! {
                    __scope.#var
                });

                vars.push(Variable::Expr(syn::parse2(group.stream())?));
            }
            TokenTree::Group(ref group) => output.extend(Some(TokenTree::Group(Group::new(
                group.delimiter(),
                interpolate(group.stream(), vars)?,
            )))),
            _ => {
                if let Some(punct) = interpolating.take() {
                    output.extend(Some(TokenTree::Punct(punct)))
                }

                output.extend(Some(token))
            }
        }
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use syn::parse_quote;

    use super::*;

    #[test]
    fn empty_closure() {
        let c: Closure = parse_quote! { () => {} };

        assert!(c.captures.is_none());
        assert!(c.params.is_empty());
        assert!(c.output.is_none());
        assert!(c.script.is_empty());
    }

    #[test]
    fn simple_closure() {
        let c: Closure = parse_quote! { (n: usize) -> usize => { print(n); n } };

        assert!(c.captures.is_none());
        assert_eq!(c.params.len(), 1);
        assert_matches!(
            c.params.first().unwrap().value(),
            FnArg::Captured(syn::ArgCaptured {
                pat: syn::Pat::Ident(syn::PatIdent { ident, .. }),
                colon_token,
                ty: syn::Type::Path(syn::TypePath { path, ..}),
            }) if ident == "n" && path.is_ident("usize")
        );

        let ty = c.output.unwrap();
        if_chain! {
            if let syn::ReturnType::Type(_, ty) = ty;
            if let syn::Type::Path(syn::TypePath { path, .. }) = ty.as_ref();
            if path.is_ident("usize");
            then {

            } else {
                panic!("unexpected output type: {:?}", ty);
            }
        }
        assert_eq!(c.script.to_string(), "print ( n ) ; n");
    }

    #[test]
    fn closure_to_expr() {
        let c: Closure = parse_quote! { () => print(n) };

        assert_eq!(c.script.to_string(), "print ( n )");
    }

    #[test]
    fn interpolating() {
        let _ = pretty_env_logger::try_init();
        let mut vars = vec![];

        assert_eq!(
            interpolate(
                TokenStream::from_str("print(\"hello world\")").unwrap(),
                &mut vars
            )
            .unwrap()
            .to_string(),
            "print ( \"hello world\" )"
        );

        assert_eq!(
            interpolate(TokenStream::from_str("print(#name)").unwrap(), &mut vars)
                .unwrap()
                .to_string(),
            "print ( __scope . var0 )"
        );

        assert_eq!(
            interpolate(
                TokenStream::from_str("print(#(person.name))").unwrap(),
                &mut vars
            )
            .unwrap()
            .to_string(),
            "print ( __scope . var1 )"
        );
    }
}
