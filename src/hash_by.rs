use crate::parsing::{parse_input, ParsedFields, ParsedInput, ParsingError};
use proc_macro2::{Literal, TokenStream};
use quote::{quote, quote_spanned, ToTokens};
use syn::spanned::Spanned;
use syn::{DeriveInput, Error};

pub fn impl_hash_by_derive(input: DeriveInput) -> TokenStream {
    let input_span = input.span();
    let struct_name = input.ident.clone();

    let ParsedInput {
        expressions: sortable_expressions,
        fields: sortable_fields,
        generics,
        generic_arguments: generics_params,
    } = match parse_input(input, "hash_by") {
        Ok(value) => value,
        Err(err) => {
            return match err {
                ParsingError::Error(err) => err,
                ParsingError::NoField(span) => Error::new(
                    span,
                    "HashBy: no field to compare on. Mark fields to compare on with #[hash_by]",
                ),
            }
            .into_compile_error()
        }
    };

    let expr_hash_statements = {
        let mut hash_exprs = sortable_expressions
            .iter()
            .map(|expr| quote_spanned!(expr.span() => self.#expr.hash(state)))
            .peekable();
        if hash_exprs.peek().is_some() {
            Some(quote!(#(#hash_exprs);*;))
        } else {
            None
        }
    };

    let field_hash_expressions = match &sortable_fields {
        ParsedFields::Struct(sortable_expr) => {
            let mut hash_exprs = sortable_expr
                .iter()
                .map(|expr| quote_spanned!(expr.span() => self.#expr.hash(state)))
                .peekable();
            if hash_exprs.peek().is_some() {
                Some(quote! { #(#hash_exprs);*; })
            } else {
                None
            }
        }
        ParsedFields::Enum(sortable_variants) => {
            let mut hash_statements = sortable_variants
                .iter()
                .enumerate()
                .filter(|(_, (_, sortable_expr))| sortable_expr.len() != 0)
                .map(|(i, (variant, sortable_expr))| {
                    let hash_pattern = quote_spanned! {variant.span() => self @ #variant};
                    let variant_num = Literal::usize_unsuffixed(i).to_token_stream();
                    let variant_hash_statement = quote! {state.write_u8(#variant_num)};
                    let hash_statement = sortable_expr
                        .iter()
                        .map(|expr| quote_spanned!(expr.span() => self.#expr.hash(state)));
                    quote! {
                        #hash_pattern => {
                            #variant_hash_statement;
                            #(#hash_statement);*
                        }
                    }
                })
                .peekable();
            if hash_statements.peek().is_some() {
                Some(quote! {
                    match self {
                        #(#hash_statements),*
                    }
                })
            } else {
                None
            }
        }
    };

    let hash_expr = match (expr_hash_statements, field_hash_expressions) {
        (Some(exprs), Some(fields)) => {
            quote! {
                #exprs
                #fields
            }
        }
        (Some(stmts), None) | (None, Some(stmts)) => stmts,
        (None, None) => unreachable!("Missing fields to hash by should have errored earlier."),
    };

    let where_clause = &generics.where_clause;

    quote_spanned! {input_span =>
        impl #generics ::core::hash::Hash for #struct_name <#(#generics_params),*> #where_clause {
            fn hash<H: ::core::hash::Hasher>(&self, state: &mut H) {
                #hash_expr
            }
        }

    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::assert_rust_eq;

    #[test]
    fn test_struct() {
        let input = syn::parse_quote! {
            #[hash_by(embed.otherfield)]
            struct Toto {
                #[hash_by]
                a: u16,
                #[hash_by]
                c: u32,
                b: f32,
                embed: EmbedStruct
            }
        };

        let output = impl_hash_by_derive(syn::parse2(input).unwrap());
        assert_rust_eq!(
            output.to_string(),
            r#"impl ::core::hash::Hash for Toto {
    fn hash<H: ::core::hash::Hasher>(&self, state: &mut H) {
        self.embed.otherfield.hash(state);
        self.a.hash(state);
        self.c.hash(state);
    }
}
"#
        );
    }

    #[test]
    fn test_enum() {
        let input = syn::parse_quote! {
            #[hash_by(this, this.that, get_something(), something.do_this())]
            enum Toto {
                A(u32),
                B,
                G { doesnotmatter: String, anyway: usize }
            }
        };

        let output = impl_hash_by_derive(syn::parse2(input).unwrap());
        assert_rust_eq!(
            output.to_string(),
            r#"impl ::core::hash::Hash for Toto {
    fn hash<H: ::core::hash::Hasher>(&self, state: &mut H) {
        self.this.hash(state);
        self.this.that.hash(state);
        self.get_something().hash(state);
        self.something.do_this().hash(state);
    }
}
"#
        );
    }

    #[test]
    fn test_singlecall() {
        let input = syn::parse_quote! {
            #[hash_by(get_something())]
            enum Toto {
                A(u32),
                B,
                G { doesnotmatter: String, anyway: usize }
            }
        };

        let output = impl_hash_by_derive(syn::parse2(input).unwrap());
        assert_rust_eq!(
            output.to_string(),
            r#"impl ::core::hash::Hash for Toto {
    fn hash<H: ::core::hash::Hasher>(&self, state: &mut H) {
        self.get_something().hash(state);
    }
}
"#
        );
    }

    #[test]
    fn test_lifetime() {
        let input = syn::parse_quote! {
            #[derive(HashBy)]
            pub struct ContextWrapper<'a, T>
            where T: Ctx,
            {
                ctx: Cow<'a, T>,
                #[hash_by]
                elapsed: i32,
            }
        };

        let output = impl_hash_by_derive(syn::parse2(input).unwrap());
        assert_rust_eq!(
            output.to_string(),
            r#"impl<'a, T> ::core::hash::Hash for ContextWrapper<'a, T>
where
    T: Ctx,
{
    fn hash<H: ::core::hash::Hasher>(&self, state: &mut H) {
        self.elapsed.hash(state);
    }
}
"#
        );
    }

    #[test]
    fn test_tuple_struct() {
        let input = syn::parse_quote! {
            #[hash_by(somemethod(), literal, some.path)]
            struct Something (
              #[hash_by]
              u16,
              #[hash_by]
              u32,
              f32,
            );
        };

        let output = impl_hash_by_derive(syn::parse2(input).unwrap());
        assert_rust_eq!(
            output.to_string(),
            r#"impl ::core::hash::Hash for Something {
    fn hash<H: ::core::hash::Hasher>(&self, state: &mut H) {
        self.somemethod().hash(state);
        self.literal.hash(state);
        self.some.path.hash(state);
        self.0.hash(state);
        self.1.hash(state);
    }
}
"#
        );
    }
}
