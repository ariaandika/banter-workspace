use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{self, Data, DataStruct, DeriveInput, Fields};

macro_rules! decv {
    ($i:tt,$d:tt,$f:ident) => {
        #[proc_macro_derive($i)]
        pub fn $d(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
            $f(&syn::parse_macro_input!(input as DeriveInput)).unwrap_or_else(|err|err.to_compile_error()).into()
        }
    };
}

decv!(EnumExt,ex,enum_ext_impl);
decv!(EnumDecode,ed,enum_decode_impl);
decv!(IdDecode,id,id_decode_impl);
decv!(FromRow,fr,from_row_impl);

fn v<T>(i: usize) -> Vec<T> { Vec::with_capacity(i) }

fn enum_ext_impl(ast: &DeriveInput) -> syn::Result<TokenStream> {
    let Data::Enum(en) = &ast.data else {
        return Err(syn::Error::new_spanned(ast, "EnumExt only support enum"))
    };

    let name = &ast.ident;
    let l = en.variants.len();
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
    let vl = syn::Lit::Int(syn::LitInt::new(&en.variants.len().to_string(), Span::call_site()));

    let (from_str,as_str,variants) = en.variants.iter()
        .fold((v(l),v(l),v(l)),|(mut fs,mut ar,mut vs),v|{
            let name = &v.ident;
            fs.push(quote! { stringify!(#name) => Ok(Self::#name), });
            ar.push(quote! { Self::#name => stringify!(#name), });
            vs.push(quote! { stringify!(#name), });
            (fs,ar,vs)
        });

    Ok(quote! {
        impl #impl_generics #name #ty_generics #where_clause {
            pub const VARIANTS: [&'static str;#vl] = [#(#variants)*];
            pub fn from_str<'r>(input: &'r str) -> Result<Self, &'r str> {
                match input { #(#from_str)* _ => Err(input) }
            }
            pub fn as_str(&self) -> &'static str {
                match self { #(#as_str)* }
            }
        }
    })
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

        impl<'q> ::sqlx::Encode<'q,::sqlx::Postgres> for #name #ty_generics #where_clause {
            fn encode_by_ref(
                &self,
                buf: &mut <::sqlx::Postgres as ::sqlx::Database>::ArgumentBuffer<'q>,
            ) -> Result<::sqlx::encode::IsNull, ::sqlx::error::BoxDynError> {
                <i32 as ::sqlx::Encode<::sqlx::Postgres>>::encode_by_ref(&self.0, buf)
            }
        }

        impl<'r> ::sqlx::Decode<'r,::sqlx::Postgres> for #name #ty_generics #where_clause {
            fn decode(value: <::sqlx::Postgres as ::sqlx::Database>::ValueRef<'r>) -> Result<Self, ::sqlx::error::BoxDynError> {
                Ok(Self(<i32 as ::sqlx::Decode<::sqlx::Postgres>>::decode(value)?))
            }
        }
    })
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


