use proc_macro::TokenStream;
use proc_macro2::Span;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, ToTokens};
use syn::{
    braced, bracketed, parenthesized,
    parse::{Error, Parse, ParseStream, Result as SynResult},
    punctuated::Punctuated,
    token::{Comma, Mut},
    Ident, Lifetime, Lit, Type,
};

pub trait LitExt {
    fn to_str(&self) -> String;
    fn to_bool(&self) -> bool;
    fn to_ident(&self) -> Ident;
}

impl LitExt for Lit {
    fn to_str(&self) -> String {
        match self {
            Lit::Str(s) => s.value(),
            Lit::ByteStr(s) => unsafe { String::from_utf8_unchecked(s.value()) },
            Lit::Char(c) => c.value().to_string(),
            Lit::Byte(b) => (b.value() as char).to_string(),
            _ => panic!("values must be a (byte)string or a char"),
        }
    }

    fn to_bool(&self) -> bool {
        if let Lit::Bool(b) = self {
            b.value
        } else {
            self.to_str()
                .parse()
                .unwrap_or_else(|_| panic!("expected bool from {:?}", self))
        }
    }

    #[inline]
    fn to_ident(&self) -> Ident {
        Ident::new(&self.to_str(), self.span())
    }
}

pub trait IdentExt2: Sized {
    fn to_uppercase(&self) -> Self;
    fn with_suffix(&self, suf: &str) -> Ident;
}

impl IdentExt2 for Ident {
    #[inline]
    fn to_uppercase(&self) -> Self {
        format_ident!("{}", self.to_string().to_uppercase())
    }

    #[inline]
    fn with_suffix(&self, suffix: &str) -> Ident {
        format_ident!("{}_{}", self.to_string().to_uppercase(), suffix)
    }
}

#[inline]
pub fn into_stream(e: Error) -> TokenStream {
    e.to_compile_error().into()
}

macro_rules! propagate_err {
    ($res:expr) => {{
        match $res {
            Ok(v) => v,
            Err(e) => return $crate::util::into_stream(e),
        }
    }};
}

#[derive(Debug)]
pub struct Bracketed<T>(pub Punctuated<T, Comma>);

impl<T: Parse> Parse for Bracketed<T> {
    fn parse(input: ParseStream<'_>) -> SynResult<Self> {
        let content;
        bracketed!(content in input);

        Ok(Bracketed(content.parse_terminated(T::parse)?))
    }
}

#[derive(Debug)]
pub struct Braced<T>(pub Punctuated<T, Comma>);

impl<T: Parse> Parse for Braced<T> {
    fn parse(input: ParseStream<'_>) -> SynResult<Self> {
        let content;
        braced!(content in input);

        Ok(Braced(content.parse_terminated(T::parse)?))
    }
}

#[derive(Debug)]
pub struct Parenthesised<T>(pub Punctuated<T, Comma>);

impl<T: Parse> Parse for Parenthesised<T> {
    fn parse(input: ParseStream<'_>) -> SynResult<Self> {
        let content;
        parenthesized!(content in input);

        Ok(Parenthesised(content.parse_terminated(T::parse)?))
    }
}

#[derive(Debug)]
pub struct AsOption<T>(pub Option<T>);

impl<T: ToTokens> ToTokens for AsOption<T> {
    fn to_tokens(&self, stream: &mut TokenStream2) {
        match &self.0 {
            Some(o) => stream.extend(quote!(Some(#o))),
            None => stream.extend(quote!(None)),
        }
    }
}

impl<T> Default for AsOption<T> {
    #[inline]
    fn default() -> Self {
        AsOption(None)
    }
}

#[derive(Debug)]
pub struct Argument {
    pub mutable: Option<Mut>,
    pub name: Ident,
    pub kind: Type,
}

impl ToTokens for Argument {
    fn to_tokens(&self, stream: &mut TokenStream2) {
        let Argument {
            mutable,
            name,
            kind,
        } = self;

        stream.extend(quote! {
            #mutable #name: #kind
        });
    }
}

#[inline]
pub fn populate_fut_lifetimes_on_refs(args: &mut Vec<Argument>) {
    for arg in args {
        if let Type::Reference(reference) = &mut arg.kind {
            reference.lifetime = Some(Lifetime::new("'fut", Span::call_site()));
        }
    }
}
