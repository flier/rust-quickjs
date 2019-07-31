#![recursion_limit = "128"]

#[macro_use]
extern crate log;
#[macro_use]
extern crate if_chain;

#[cfg(test)]
#[macro_use]
extern crate matches;

use std::fmt;

use proc_macro2::{Delimiter, Group, Ident, Spacing, Span, TokenStream, TokenTree};
use quote::quote;
use syn::{
    braced, bracketed, parenthesized,
    parse::{Parse, ParseStream},
    parse_quote,
    punctuated::Punctuated,
    token::{Brace, Bracket, Comma, FatArrow, Paren, RArrow},
    Expr, FnArg, Result, ReturnType, Type,
};

pub fn qjs(input: TokenStream) -> Result<TokenStream> {
    match syn::parse2(input)? {
        Item::Eval(Eval { context, script }) => {
            trace!(
                "eval script with {} context: {}",
                context
                    .as_ref()
                    .map_or("anonymous".to_owned(), |WithContext { ident, .. }| ident
                        .to_string()),
                script.to_string(),
            );

            let context = context.map_or_else(
                || {
                    quote! {
                        let rt = qjs::Runtime::new();
                        let ctxt = qjs::Context::new(&rt);
                    }
                },
                |WithContext { ident, .. }| {
                    quote! {
                        let ctxt = #ident;
                    }
                },
            );
            let mut vars = vec![];
            let interpolated_script = interpolate(script, &mut vars)?.to_string();

            trace!("found {} variables: {:?}", vars.len(), vars);
            trace!("interpolated script: {}", interpolated_script.to_string());

            let global = if vars.is_empty() {
                None
            } else {
                Some(quote! {
                    let global = ctxt.global_object();
                })
            };
            let captures = vars.into_iter().enumerate().map(|(i, var)| match var {
                Variable::Ident(ident) => {
                    let name = ident.to_string();

                    quote! {
                        global.set_property(#name, #ident);
                    }
                }
                Variable::Expr(expr) => {
                    let name = format!("var{}", i);

                    quote! {
                        global.set_property(#name, #expr);
                    }
                }
            });

            let expanded = quote! {{
                #context
                #global
                #(#captures)*

                ctxt.eval(#interpolated_script, qjs::Eval::GLOBAL)
            }};

            trace!("expandedscript:\n{}", expanded.to_string());

            Ok(expanded)
        }
        Item::Closure(Closure {
            captures,
            params,
            output,
            script,
            ..
        }) => {
            let param_names = params
                .iter()
                .flat_map(|param| match param {
                    syn::FnArg::Captured(syn::ArgCaptured {
                        pat: syn::Pat::Ident(syn::PatIdent { ident, .. }),
                        ..
                    }) => Some(ident.to_string()),
                    _ => {
                        warn!("ignore param: {:?}", param);

                        None
                    }
                })
                .collect::<Vec<_>>();

            let mut vars = vec![];
            let interpolated_script = interpolate(script, &mut vars)?;
            let script = format!(
                "({}) => {{ {} }}",
                param_names.join(", "),
                interpolated_script.to_string()
            );
            let global = if vars.is_empty() {
                None
            } else {
                Some(quote! {
                    let global = ctxt.global_object();
                })
            };
            let (output, output_ty) = if_chain! {
                if let Some(output) = output;
                if let ReturnType::Type(rarrow, output_ty) = output;
                then {
                    (
                        ReturnType::Type(
                            rarrow,
                            Box::new(Type::Path(parse_quote! {
                                Result<Option<#output_ty>, failure::Error>
                            })),
                        ),
                        output_ty,
                    )
                } else {
                    (ReturnType::Default, parse_quote! { () })
                }
            };

            let args = param_names
                .into_iter()
                .map(|name| Ident::new(&name, Span::call_site()))
                .collect::<Vec<_>>();
            let args = args.as_slice();

            let captures = vars.into_iter().enumerate().map(|(i, var)| match var {
                Variable::Ident(name) => {
                    quote! { global.set_property(stringify!(#name), #name); }
                }
                Variable::Expr(expr) => {
                    let var = Ident::new(&format!("var{}", i), Span::call_site());

                    quote! { global.set_property(#var, #expr); }
                }
            });

            let expanded = quote! {
                move | #(#args),* | #output {
                    let rt = qjs::Runtime::new();
                    let ctxt = qjs::Context::new(&rt);
                    #global
                    #(#captures)*

                    let func = ctxt.eval_script(#script, "<evalScript>", qjs::Eval::GLOBAL)?;

                    func.call(None, (#(#args),*))
                        .map(|v| if v.is_undefined() {
                            None
                        } else {
                            <#output_ty as qjs::ExtractValue>::extract_value(&v)
                        })
                }
            };

            trace!("expanded script:\n{}", expanded.to_string());

            Ok(expanded)
        }
    }
}

enum Item {
    Eval(Eval),
    Closure(Closure),
}

impl Parse for Item {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.fork().parse::<Closure>().is_ok() {
            input.parse().map(Item::Closure)
        } else {
            input.parse().map(Item::Eval)
        }
    }
}

struct Eval {
    pub context: Option<WithContext>,
    pub script: TokenStream,
}

impl Parse for Eval {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Eval {
            context: if input.peek(syn::Ident) && input.peek2(FatArrow) {
                Some(input.parse()?)
            } else {
                None
            },
            script: input.parse()?,
        })
    }
}

struct WithContext {
    pub ident: Ident,
    pub fat_arrow_token: FatArrow,
}

impl Parse for WithContext {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(WithContext {
            ident: input.parse()?,
            fat_arrow_token: input.parse()?,
        })
    }
}

struct Closure {
    pub captures: Option<Captures>,
    pub paren_token: Paren,
    pub params: Punctuated<FnArg, Comma>,
    pub output: Option<ReturnType>,
    pub fat_arrow_token: FatArrow,
    pub brace_token: Option<Brace>,
    pub script: TokenStream,
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

struct Captures {
    pub bracket_token: Bracket,
    pub inputs: Punctuated<Ident, Comma>,
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

enum Variable {
    Ident(Ident),
    Expr(Expr),
}

impl fmt::Debug for Variable {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Variable::Ident(ident) => f
                .debug_tuple("Variable::Ident")
                .field(&ident.to_string())
                .finish(),
            Variable::Expr(expr) => f
                .debug_tuple("Variable::Expr")
                .field(&quote! { #expr }.to_string())
                .finish(),
        }
    }
}

fn interpolate(input: TokenStream, vars: &mut Vec<Variable>) -> Result<TokenStream> {
    let mut output = TokenStream::new();
    let mut interpolating = None;

    for token in input {
        match token {
            TokenTree::Punct(ref punct)
                if punct.as_char() == '#' && punct.spacing() == Spacing::Alone =>
            {
                interpolating = Some(punct.clone())
            }
            TokenTree::Ident(ref name) if interpolating.is_some() => {
                let _ = interpolating.take();

                output.extend(quote! { #name });

                vars.push(Variable::Ident(name.clone()));
            }
            TokenTree::Group(ref group)
                if interpolating.is_some() && group.delimiter() == Delimiter::Parenthesis =>
            {
                let _ = interpolating.take();
                let var = Ident::new(&format!("var{}", vars.len()), Span::call_site());

                output.extend(quote! { #var });

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

    use super::*;

    #[test]
    fn javascript() {
        assert_eq!(
            js(quote! { 1+2 }).unwrap().to_string(),
            quote! { qjs::eval("1 + 2") }.to_string(),
        );
        assert_eq!(
            js(quote! { ctxt => 1+2 }).unwrap().to_string(),
            quote! { ctxt.eval("1 + 2") }.to_string(),
        );

        assert_eq!(
            js(quote! { () => 1+2 }).unwrap().to_string(),
            quote! { | | { qjs::eval("function() { 1 + 2 }") } }.to_string()
        );

        assert_eq!(
            js(quote! { (n: usize) -> usize => { n+1 } })
                .unwrap()
                .to_string(),
            quote! { |n: usize| -> usize { qjs::eval("function(n) { n + 1 }") } }.to_string()
        );
    }

    #[test]
    fn eval() {
        let e: Eval = parse_quote! { 1+2 };

        assert!(e.context.is_none());

        let e: Eval = parse_quote! { ctxt => 1+2 };

        assert_eq!(e.context.unwrap().ident.to_string(), "ctxt");
        assert_eq!(e.script.to_string(), "1 + 2");
    }

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
        let c: Closure = parse_quote! { [print] (n: usize) -> usize => { print(n); n } };

        let inputs = c.captures.unwrap().inputs;

        assert_eq!(inputs.len(), 1);
        assert_matches!(
            inputs.first().unwrap().value().to_string().as_str(),
            "print"
        );

        assert_eq!(c.params.len(), 1);
        assert_matches!(
            c.params.first().unwrap().value(),
            FnArg::Captured(syn::ArgCaptured {
                pat: syn::Pat::Ident(syn::PatIdent { ident, .. }),
                ty: syn::Type::Path(syn::TypePath { path, ..}),..
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
            quote! { print("hello world") }.to_string()
        );

        assert_eq!(
            interpolate(TokenStream::from_str("print(#name)").unwrap(), &mut vars)
                .unwrap()
                .to_string(),
            quote! { print(var0) }.to_string()
        );

        assert_eq!(
            interpolate(
                TokenStream::from_str("print(#(person.name))").unwrap(),
                &mut vars
            )
            .unwrap()
            .to_string(),
            quote! { print(var1) }.to_string()
        );
    }
}
