#![deny(rust_2018_idioms)]
// FIXME: Remove this in a foreseeable future.
// Currently exists for backwards compatibility to previous Rust versions.
#![recursion_limit = "128"]

#[allow(unused_extern_crates)]
extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse::Error,
    parse_macro_input, parse_quote,
    spanned::Spanned,
    Lit,
};

pub(crate) mod attributes;
pub(crate) mod consts;
pub(crate) mod structures;

#[macro_use]
pub(crate) mod util;

use attributes::*;
use consts::*;
use structures::*;
use util::*;

macro_rules! match_options {
    ($v:expr, $values:ident, $options:ident, $span:expr => [$($name:ident);*]) => {
        match $v {
            $(
                stringify!($name) => $options.$name = propagate_err!($crate::attributes::parse($values)),
            )*
            _ => {
                return Error::new($span, format_args!("invalid attribute: {:?}", $v))
                    .to_compile_error()
                    .into();
            },
        }
    };
}

#[proc_macro_attribute]
pub fn command(attr: TokenStream, input: TokenStream) -> TokenStream {
    let mut fun = parse_macro_input!(input as CommandFun);

    let lit_name = if !attr.is_empty() {
        parse_macro_input!(attr as Lit).to_str()
    } else {
        fun.name.to_string()
    };

    let mut options = Options::new();

    for attribute in &fun.attributes {
        let span = attribute.span();
        let values = propagate_err!(parse_values(attribute));

        let name = values.name.to_string();
        let name = &name[..];

        match_options!(name, values, options, span => [
            permission_level;
            supports_dm;
            can_blacklist
        ]);
    }

    let Options {
        permission_level,
        supports_dm,
        can_blacklist,
    } = options;

    propagate_err!(create_declaration_validations(&mut fun, DeclarFor::Command));

    let res = parse_quote!(serenity::framework::standard::CommandResult);
    create_return_type_validation(&mut fun, res);

    let visibility = fun.visibility;
    let name = fun.name.clone();
    let body = fun.body;
    let ret = fun.ret;

    let n = name.with_suffix(COMMAND);

    let cooked = fun.cooked.clone();

    let command_path = quote!(crate::framework::Command);

    populate_fut_lifetimes_on_refs(&mut fun.args);
    let args = fun.args;

    (quote! {
        #(#cooked)*
        pub static #n: #command_path = #command_path {
            func: #name,
            name: #lit_name,
            required_perms: #permission_level,
            supports_dm: #supports_dm,
            can_blacklist: #can_blacklist,
        };

        #visibility fn #name<'fut> (#(#args),*) -> ::serenity::futures::future::BoxFuture<'fut, #ret> {
            use ::serenity::futures::future::FutureExt;

            async move { #(#body)* }.boxed()
        }
    })
    .into()
}
