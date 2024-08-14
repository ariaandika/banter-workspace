use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{self, Data, DataStruct, DeriveInput, Fields};

#[proc_macro_derive(EnumExt)]
pub fn enum_str_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = syn::parse_macro_input!(input as DeriveInput);
    enum_ext_impl(&ast).unwrap_or_else(|err|err.to_compile_error()).into()
}

fn enum_ext_impl(ast: &DeriveInput) -> syn::Result<TokenStream> {
    let Data::Enum(en) = &ast.data else {
        return Err(syn::Error::new_spanned(ast, "EnumExt only support enum"))
    };

    let name = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    let mut quotes = Vec::with_capacity(en.variants.len());

    for variant in &en.variants {
        let name = &variant.ident;
        quotes.push(quote! { stringify!(#name) => Ok(Self::#name), });
    }

    let mut quotes2 = Vec::with_capacity(en.variants.len());

    for variant in &en.variants {
        let name = &variant.ident;
        quotes2.push(quote! { Self::#name => stringify!(#name), });
    }

    let np = syn::Lit::Int(syn::LitInt::new(&en.variants.len().to_string(), Span::call_site()));
    let mut quotes3 = Vec::with_capacity(en.variants.len());

    for variant in &en.variants {
        let name = &variant.ident;
        quotes3.push(quote! { stringify!(#name), });
    }

    Ok(quote! {
        impl #impl_generics #name #ty_generics #where_clause {
            pub const VARIANTS: [&'static str;#np] = [#(#quotes3)*];
            pub fn from_str<'r>(input: &'r str) -> Result<Self, &'r str> {
                match input { #(#quotes)* _ => Err(input) }
            }
            pub fn as_str(&self) -> &'static str {
                match self { #(#quotes2)* }
            }
        }
    })
}

#[proc_macro_derive(EnumDecode)]
pub fn enum_decode_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = syn::parse_macro_input!(input as DeriveInput);
    enum_decode_impl(&ast).unwrap_or_else(|err|err.to_compile_error()).into()
}

fn enum_decode_impl(ast: &DeriveInput) -> syn::Result<TokenStream> {
    let Data::Enum(_) = &ast.data else {
        return Err(syn::Error::new_spanned(ast, "EnumDecode not supported"))
    };

    let name = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics ::sqlx::Type<::sqlx::Postgres> for #name #ty_generics #where_clause {
            fn type_info() -> <::sqlx::Postgres as ::sqlx::Database>::TypeInfo {
                <str as ::sqlx::Type<::sqlx::Postgres>>::type_info()
            }
        }

        impl<'r> ::sqlx::Decode<'r,::sqlx::Postgres> for #name #ty_generics #where_clause {
            fn decode(value: <::sqlx::Postgres as ::sqlx::Database>::ValueRef<'r>) -> Result<Self, ::sqlx::error::BoxDynError> {
                Self::from_str(<&str as ::sqlx::Decode<::sqlx::Postgres>>::decode(value)?)
                    .map_err(|er|::sqlx::error::Error::Decode(Box::new(<::serde_json::error::Error as ::serde::de::Error>::unknown_variant(er, &Self::VARIANTS))).into())
            }
        }
    })
}



#[proc_macro_derive(IdDecode)]
pub fn id_decode_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = syn::parse_macro_input!(input as DeriveInput);
    id_decode_impl(&ast).unwrap_or_else(|err|err.to_compile_error()).into()
}

fn id_decode_impl(ast: &DeriveInput) -> syn::Result<TokenStream> {
    let Data::Struct(DataStruct { fields: Fields::Unnamed(_), .. }) = &ast.data else {
        return Err(syn::Error::new_spanned(ast, "IdDecode not supported"))
    };

    let name = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics ::sqlx::Type<::sqlx::Postgres> for #name #ty_generics #where_clause {
            fn type_info() -> <::sqlx::Postgres as ::sqlx::Database>::TypeInfo {
                <i32 as ::sqlx::Type<::sqlx::Postgres>>::type_info()
            }
        }

        impl<'r> ::sqlx::Decode<'r,::sqlx::Postgres> for #name #ty_generics #where_clause {
            fn decode(value: <::sqlx::Postgres as ::sqlx::Database>::ValueRef<'r>) -> Result<Self, ::sqlx::error::BoxDynError> {
                Ok(Self(<i32 as ::sqlx::Decode<::sqlx::Postgres>>::decode(value)?))
            }
        }
    })
}


#[proc_macro_derive(FromRow)]
pub fn from_row_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = syn::parse_macro_input!(input as DeriveInput);
    from_row_impl(&ast).unwrap_or_else(|err|err.to_compile_error()).into()
}

fn from_row_impl(ast: &DeriveInput) -> syn::Result<TokenStream> {
    let Data::Struct(st) = &ast.data else {
        return Err(syn::Error::new_spanned(ast, "FromRow not supported"))
    };

    let name = &ast.ident;
    let (_, ty_generics, where_clause) = ast.generics.split_for_impl();

    let mut quotes = Vec::with_capacity(st.fields.len());

    for field in &st.fields {
        let name = &field.ident;
        quotes.push(quote! { #name: <::sqlx::postgres::PgRow as ::sqlx::Row>::try_get(row, stringify!(#name))?, });
    }

    Ok(quote! {
        impl<'r> ::sqlx::FromRow<'r,::sqlx::postgres::PgRow> for #name #ty_generics #where_clause {
            fn from_row(row: &'r ::sqlx::postgres::PgRow) -> Result<Self, ::sqlx::Error> {
                Ok(Self { #(#quotes)* })
            }
        }
    })
}


