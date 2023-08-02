use crate::parsing::{parse_input, ParsedFields, ParsedInput, ParsingError};
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned, ToTokens};
use syn::{parse2, parse_quote_spanned, spanned::Spanned, DeriveInput, Error, Expr};

pub fn impl_cmp_by_derive(input: DeriveInput) -> TokenStream {
    println!("Entered impl_cmp_by_derive");
    let input_span = input.span();
    let struct_name = input.ident.clone();

    let ParsedInput {
        expressions: sortable_expressions,
        fields: sortable_fields,
        generics,
        generic_arguments: generics_params,
    } = match parse_input(input, "cmp_by") {
        Ok(value) => value,
        Err(err) => {
            return match err {
                ParsingError::Error(err) => err,
                ParsingError::NoField(span) => Error::new(
                    span,
                    "CmpBy: no field to compare on. Mark fields to compare on with #[cmp_by]",
                ),
            }
            .into_compile_error()
        }
    };
    println!("Successfully parsed input");

    let field_ord_statement = match &sortable_fields {
        ParsedFields::Struct(sortable_expr) => gen_cmp_exprs(sortable_expr),
        ParsedFields::Enum(sortable_variants) => {
            let ord_statements = sortable_variants
                .iter()
                .filter(|(_, sortable_expr)| sortable_expr.len() != 0)
                .map(|(variant, sortable_expr)| {
                    let ord_pattern =
                        quote_spanned! {variant.span() => (#variant @ this, #variant @ other)};
                    let ord_statement = gen_cmp_exprs(sortable_expr);
                    quote! {#ord_pattern => #ord_statement}
                });
            // TODO: What do we compare when we have different variants?
            // TODO: And what about variants that have no fields marked to cmp?
            let stream = quote_spanned! { input_span =>
                match (self, other) {
                    #(#ord_statements,)*
                    (_this, _other) => ::core::cmp::Ordering::Equal
                }
            };
            Some(match parse2(stream.clone()) {
                Ok(ts) => ts,
                Err(err) => {
                    println!("{stream}");
                    panic!("{err}");
                }
            })
        }
    };
    println!("Successfully generated field cmps");

    println!("Entering gen_cmp_expr");
    let expr_ord_statements = sortable_expressions
        .iter()
        .map(|expr| {
            if expr.to_token_stream().to_string() == "_fields" {
                parse_quote_spanned! { expr.span() =>
                    #field_ord_statement
                }
            } else {
                parse_quote_spanned! { expr.span() =>
                    self.#expr.cmp(&other.#expr)
                }
            }
        })
        .reduce(|ord_expr: Expr, expr| {
            // println!("Combining {} with {}", quote!(#ord_expr), quote!(#expr));
            parse_quote_spanned! {expr.span() =>
                #ord_expr.then_with(|| #expr)
            }
        });
    println!("Successfully generated preceding expressions cmps");

    let ord_expression = match (expr_ord_statements, field_ord_statement) {
        (Some(exprs), Some(fields)) => {
            parse_quote_spanned! {input_span =>
                #exprs.then_with(|| #fields)
            }
        }
        (None, Some(ts)) | (Some(ts), None) => ts,
        (None, None) => {
            unreachable!("Error of no fields to compare on should be handled in the parsing stage.")
        }
    };
    println!("Successfully combined preceding expressions with fields cmps");

    let where_clause = &generics.where_clause;
    let generics_params = &generics_params;

    quote_spanned! {input_span =>
        impl #generics ::core::cmp::Eq for #struct_name <#(#generics_params),*> #where_clause {}

        impl #generics ::core::cmp::PartialEq<Self> for #struct_name <#(#generics_params),*> #where_clause {
            #[inline]
            fn eq(&self, other: &Self) -> bool {
                self.cmp(other).is_eq()
            }
        }

        impl #generics ::core::cmp::PartialOrd<Self> for #struct_name <#(#generics_params),*> #where_clause {
            #[inline]
            fn partial_cmp(&self, other: &Self) -> ::core::option::Option<::core::cmp::Ordering> {
                ::core::option::Option::Some(self.cmp(other))
            }
        }

        impl #generics ::core::cmp::Ord for #struct_name <#(#generics_params),*> #where_clause {
            #[inline]
            fn cmp(&self, other: &Self) -> ::core::cmp::Ordering {
                #ord_expression
            }
        }
    }
}

fn gen_cmp_exprs(sortable_expr: &[Expr]) -> Option<Expr> {
    println!("Entering gen_cmp_expr");
    sortable_expr
        .iter()
        .map(|expr| {
            parse_quote_spanned! { expr.span() =>
                self.#expr.cmp(&other.#expr)
            }
        })
        .reduce(|ord_expr: Expr, expr| {
            // println!("Combining {} with {}", quote!(#ord_expr), quote!(#expr));
            parse_quote_spanned! {expr.span() =>
                #ord_expr.then_with(|| #expr)
            }
        })
}

#[cfg(test)]
pub(crate) mod test {
    use quote::quote;

    #[macro_export]
    macro_rules! assert_rust_eq {
        ($actual: expr, $expected: expr$(, $msg: literal)?) => {
            use rust_format::Formatter;
            let rust_fmt = rust_format::RustFmt::default();
            assert_eq!(
                rust_fmt
                    .format_str($actual)
                    .expect("Actual value could not be formatted :"),
                rust_fmt
                    .format_str($expected)
                    .expect("Expected value could not be formatted :"),
                $($msg)?
            );
        };
    }

    #[test]
    fn test_struct() {
        let input = syn::parse_quote! {
            #[cmp_by(embed.otherfield)]
            struct Toto {
                #[cmp_by]
                a: u16,
                #[cmp_by]
                c: u32,
                b: f32,
                embed: EmbedStruct
            }
        };

        let output = crate::cmp_by::impl_cmp_by_derive(syn::parse2(input).unwrap());
        assert_rust_eq!(
            output.to_string(),
            r#"impl ::core::cmp::Eq for Toto {}
impl ::core::cmp::PartialEq<Self> for Toto {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}
impl ::core::cmp::PartialOrd<Self> for Toto {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> ::core::option::Option<::core::cmp::Ordering> {
        ::core::option::Option::Some(self.cmp(other))
    }
}
impl ::core::cmp::Ord for Toto {
    #[inline]
    fn cmp(&self, other: &Self) -> ::core::cmp::Ordering {
        self.embed
            .otherfield
            .cmp(&other.embed.otherfield)
            .then_with(|| self.a.cmp(&other.a)
            .then_with(|| self.c.cmp(&other.c)))
    }
}
"#
        );
    }

    #[test]
    fn test_enum() {
        let input = syn::parse_quote! {
            #[cmp_by(this, this.that, get_something(), something.do_this())]
            enum Toto {
                A(#[cmp_by] u32),
                B,
                G { doesnotmatter: String, anyway: usize }
            }
        };

        let output = crate::cmp_by::impl_cmp_by_derive(syn::parse2(input).unwrap());
        assert_rust_eq!(
            output.to_string(),
            r#"impl ::core::cmp::Eq for Toto {}
impl ::core::cmp::PartialEq<Self> for Toto {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}
impl ::core::cmp::PartialOrd<Self> for Toto {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> ::core::option::Option<::core::cmp::Ordering> {
        ::core::option::Option::Some(self.cmp(other))
    }
}
impl ::core::cmp::Ord for Toto {
    #[inline]
    fn cmp(&self, other: &Self) -> ::core::cmp::Ordering {
        self.this
            .cmp(&other.this)
            .then_with(|| self.this.that.cmp(&other.this.that))
            .then_with(|| self.get_something().cmp(&other.get_something()))
            .then_with(|| self.something.do_this().cmp(&other.something.do_this()))
            .then_with(|| match (self, other) {
                (A @ this, A @ other) => self.0.cmp(&other.0),
                (_this, _other) => ::core::cmp::Ordering::Equal,
            })
    }
}
"#
        );
    }

    #[test]
    fn test_singlecall() {
        let input = syn::parse_quote! {
            #[cmp_by(get_something())]
            #[accessor(global_time: usize)]
            enum Toto {
                A(u32),
                B,
                G { doesnotmatter: String, anyway: usize }
            }
        };

        let output = crate::cmp_by::impl_cmp_by_derive(syn::parse2(input).unwrap());
        assert_rust_eq!(
            output.to_string(),
            r#"impl ::core::cmp::Eq for Toto {}
impl ::core::cmp::PartialEq<Self> for Toto {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}
impl ::core::cmp::PartialOrd<Self> for Toto {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> ::core::option::Option<::core::cmp::Ordering> {
        ::core::option::Option::Some(self.cmp(other))
    }
}
impl ::core::cmp::Ord for Toto {
    #[inline]
    fn cmp(&self, other: &Self) -> ::core::cmp::Ordering {
        self.get_something()
            .cmp(&other.get_something())
            .then_with(|| match (self, other) {
                (_this, _other) => ::core::cmp::Ordering::Equal,
            })
    }
}
"#
        );
    }

    #[test]
    fn test_lifetime() {
        let input = quote! {
            #[derive(CmpBy)]
            pub struct ContextWrapper<'a, T>
            where T: Ctx,
            {
                ctx: Cow<'a, T>,
                #[cmp_by]
                elapsed: i32,
            }
        };

        let output = crate::cmp_by::impl_cmp_by_derive(syn::parse2(input).unwrap());
        assert_rust_eq!(
            output.to_string(),
            r#"impl<'a, T> ::core::cmp::Eq for ContextWrapper<'a, T> where T: Ctx {}
impl<'a, T> ::core::cmp::PartialEq<Self> for ContextWrapper<'a, T>
where
    T: Ctx,
{
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}
impl<'a, T> ::core::cmp::PartialOrd<Self> for ContextWrapper<'a, T>
where
    T: Ctx,
{
    #[inline]
    fn partial_cmp(&self, other: &Self) -> ::core::option::Option<::core::cmp::Ordering> {
        ::core::option::Option::Some(self.cmp(other))
    }
}
impl<'a, T> ::core::cmp::Ord for ContextWrapper<'a, T>
where
    T: Ctx,
{
    #[inline]
    fn cmp(&self, other: &Self) -> ::core::cmp::Ordering {
        self.elapsed.cmp(&other.elapsed)
    }
}
"#
        );
    }

    #[test]
    fn test_tuple_struct() {
        let input = syn::parse_quote! {
            #[cmp_by(somemethod(), literal, some.path)]
            struct Something (
              #[cmp_by]
              u16,
              #[cmp_by]
              u32,
              f32,
            );
        };

        let output = crate::cmp_by::impl_cmp_by_derive(syn::parse2(input).unwrap());
        assert_rust_eq!(
            output.to_string(),
            r#"impl ::core::cmp::Eq for Something {}
impl ::core::cmp::PartialEq<Self> for Something {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}
impl ::core::cmp::PartialOrd<Self> for Something {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> ::core::option::Option<::core::cmp::Ordering> {
        ::core::option::Option::Some(self.cmp(other))
    }
}
impl ::core::cmp::Ord for Something {
    #[inline]
    fn cmp(&self, other: &Self) -> ::core::cmp::Ordering {
        self.somemethod()
            .cmp(&other.somemethod())
            .then_with(|| self.literal.cmp(&other.literal))
            .then_with(|| self.some.path.cmp(&other.some.path))
            .then_with(|| self.0.cmp(&other.0)
            .then_with(|| self.1.cmp(&other.1)))
    }
}
"#
        );
    }
}
