#![allow(clippy::manual_try_fold)]

use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{
    parse2, parse_quote, punctuated::Punctuated, spanned::Spanned, ConstParam, Data, DataEnum,
    DataStruct, DeriveInput, Error, Expr, Fields, FieldsNamed, FieldsUnnamed, GenericArgument,
    GenericParam, Generics, Index, LifetimeParam, Token, TypeParam,
};

pub enum ParsedFields {
    Struct(Vec<Expr>),
    Enum(Vec<(TokenStream, Vec<Expr>)>),
}

pub struct ParsedInput {
    pub expressions: Vec<Expr>,
    pub fields: ParsedFields,
    pub generics: Generics,
    pub generic_arguments: Vec<GenericArgument>,
}

pub(crate) fn parse_input(input: DeriveInput, attr: &str) -> Result<ParsedInput, ParsingError> {
    // println!("Entered parse_input()");
    let expressions = input
        .attrs
        .iter()
        .filter(|i| i.path().get_ident().map_or(false, |i| {
            i == attr
        }))
        .map(|attr| {
            attr.parse_args_with(Punctuated::<Expr, Token![,]>::parse_terminated)?.into_iter().map(|elem| {
                match elem {
                    Expr::Call(_) | Expr::Field(_) | Expr::Path(_) | Expr::MethodCall(_) => {
                        // TODO: test paths with lengths other than 1
                        Ok(elem)
                    }
                    _ => {
                        Err(ParsingError::Error(Error::new(elem.span(), format!("Invalid form: `{}`.\nAllowed forms: `field`, `method()`, `inner.field`, `inner.method()", elem.to_token_stream()))))
                    }
                }
            }).fold(Ok(vec![]), fold_token_errors)
        }).fold(Ok(vec![]), |acc, res| {
        match (acc, res) {
            (Ok(mut acc), Ok(res)) => {
                acc.extend(res);
                Ok(acc)
            }
            (Err(mut acc), Err(err)) => {
                acc.extend(err);
                Err(acc)
            }
            (Ok(_), Err(err)) | (Err(err), Ok(_)) => Err(err),
        }
    })?;
    // println!("Successfully parsed expressions");

    let fields = match input.data {
        Data::Struct(DataStruct {
            fields: fields @ (Fields::Unnamed(..) | Fields::Named(..)),
            ..
        }) => {
            // println!("Parsing struct fields");
            ParsedFields::Struct(parse_fields(&fields, attr)?)
        }
        Data::Enum(DataEnum { variants, .. }) => {
            // println!("Parsing enum fields");
            ParsedFields::Enum(
                variants
                    .into_iter()
                    .map(|variant| -> Result<_, ParsingError> {
                        let result = match parse_fields(&variant.fields, attr) {
                            Ok(f) => f,
                            Err(ParsingError::NoField(_)) => Vec::new(),
                            Err(e) => return Err(e),
                        };
                        let field_pat = match variant.fields {
                            Fields::Named(_) => {
                                quote!({ .. })
                            }
                            Fields::Unnamed(_) => {
                                quote!((..))
                            }
                            Fields::Unit => {
                                quote!()
                            }
                        };

                        let ident = variant.ident;
                        Ok((quote!(Self::#ident #field_pat), result))
                    })
                    .fold(Ok(vec![]), fold_token_errors)?,
            )
        }
        _ => {
            return Err(ParsingError::Error(Error::new(
                input.span(),
                r#"expected an enum or a non-unit struct"#,
            )));
        }
    };
    // println!("Successfully parsed fields");
    let generic_arguments = input
        .generics
        .params
        .iter()
        .flat_map(|p| match p {
            GenericParam::Type(TypeParam { ident, .. })
            | GenericParam::Const(ConstParam { ident, .. }) => Some(parse_quote!(#ident)),
            GenericParam::Lifetime(LifetimeParam { lifetime, .. }) => Some(parse_quote!(#lifetime)),
        })
        .collect::<Vec<_>>();
    let generics = input.generics;
    Ok(ParsedInput {
        expressions,
        fields,
        generics,
        generic_arguments,
    })
}

fn fold_token_errors<T, E>(acc: Result<Vec<T>, E>, res: Result<T, E>) -> Result<Vec<T>, E>
where
    E: Extend<Error> + IntoIterator<Item = Error>,
{
    match (acc, res) {
        (Ok(mut acc), Ok(ts)) => {
            acc.push(ts);
            Ok(acc)
        }
        (Err(mut acc), Err(err)) => {
            acc.extend(err);
            Err(acc)
        }
        (Ok(_), Err(err)) | (Err(err), Ok(_)) => Err(err),
    }
}

#[derive(Debug)]
pub(crate) enum ParsingError {
    NoField(Span),
    Error(Error),
}

impl Extend<Error> for ParsingError {
    fn extend<T: IntoIterator<Item = Error>>(&mut self, iter: T) {
        match self {
            ParsingError::NoField(_) => {
                unreachable!("When there are no fields, it should never be extended.")
            }
            ParsingError::Error(err) => {
                err.extend(iter);
            }
        }
    }
}

impl IntoIterator for ParsingError {
    type Item = Error;
    type IntoIter = <Error as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            ParsingError::NoField(_) => {
                unreachable!("When there are no fields, there are no more errors that can originate from parse_fields().")
            }
            ParsingError::Error(err) => err.into_iter(),
        }
    }
}

impl From<Error> for ParsingError {
    fn from(value: Error) -> Self {
        Self::Error(value)
    }
}

fn parse_fields(fields: &Fields, attr: &str) -> Result<Vec<Expr>, ParsingError> {
    // println!("Entered parse_fields");
    match fields {
        Fields::Named(FieldsNamed { named: fields, .. })
        | Fields::Unnamed(FieldsUnnamed {
            unnamed: fields, ..
        }) => {
            let field_span = fields.span();
            let mut cmp_fields = fields
                .into_iter()
                .enumerate()
                .filter_map(|(i, field)| -> Option<Result<Expr, ParsingError>> {
                    let span = field.span();
                    let mut attrs = field
                        .attrs
                        .iter()
                        .filter(|i| i.path().get_ident().map_or(false, |i| i == attr));
                    attrs.next()?;
                    if attrs.next().is_some() {
                        return Some(Err(ParsingError::Error(Error::new(
                            span,
                            format!(r#"expected at most one `{attr}` attribute"#),
                        ))));
                    }
                    // println!("Attempting to generate field exprs");
                    Some(
                        parse2(if let Some(ident) = &field.ident {
                            // println!("Generating named field");
                            ident.to_token_stream()
                        } else {
                            // println!("Generating unnamed field");
                            Index::from(i).to_token_stream()
                        })
                        .map_err(ParsingError::Error),
                    )
                })
                .peekable();
            if cmp_fields.peek().is_none() {
                return Err(ParsingError::NoField(field_span));
            }
            cmp_fields.fold(Ok(vec![]), fold_token_errors)
        }
        Fields::Unit => {
            // println!("Parsed unit field");
            Ok(Vec::new())
        }
    }
}
