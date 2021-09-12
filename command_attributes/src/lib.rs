#![deny(rust_2018_idioms)]
#![deny(broken_intra_doc_links)]

use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::quote;
use syn::{parse::Error, parse_macro_input, parse_quote, spanned::Spanned, Lit, Type};

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

    let _name = if !attr.is_empty() {
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

        match name {
            "subcommand" => {
                options
                    .subcommands
                    .push(Subcommand::new(propagate_err!(attributes::parse(values))));
            }
            "arg" => {
                if let Some(subcommand) = options.subcommands.last_mut() {
                    subcommand.cmd_args.push(propagate_err!(attributes::parse(values)));
                } else {
                    options.cmd_args.push(propagate_err!(attributes::parse(values)));
                }
            }
            "example" => {
                options.examples.push(propagate_err!(attributes::parse(values)));
            }
            "description" => {
                let line: String = propagate_err!(attributes::parse(values));
                if let Some(subcommand) = options.subcommands.last_mut() {
                    util::append_line(&mut subcommand.description, line);
                } else {
                    util::append_line(&mut options.description, line);
                }
            }
            _ => {
                match_options!(name, values, options, span => [
                    aliases;
                    group;
                    required_permissions;
                    can_blacklist;
                    supports_dm
                ]);
            }
        }
    }

    let Options {
        aliases,
        description,
        group,
        examples,
        required_permissions,
        can_blacklist,
        supports_dm,
        mut cmd_args,
        mut subcommands,
    } = options;

    let visibility = fun.visibility;
    let name = fun.name.clone();
    let body = fun.body;

    let n = name.with_suffix(COMMAND);

    let cooked = fun.cooked.clone();

    let command_path = quote!(crate::framework::Command);
    let arg_path = quote!(crate::framework::Arg);
    let subcommand_path = ApplicationCommandOptionType::SubCommand;

    populate_fut_lifetimes_on_refs(&mut fun.args);
    let args = fun.args;

    let mut subcommand_idents = subcommands
        .iter()
        .map(|subcommand| {
            n.with_suffix(subcommand.name.replace("-", "_").as_str()).with_suffix(SUBCOMMAND)
        })
        .collect::<Vec<Ident>>();

    let mut tokens = subcommands
        .iter_mut()
        .zip(subcommand_idents.iter())
        .map(|(subcommand, sc_ident)| {
            let arg_idents = subcommand
                .cmd_args
                .iter()
                .map(|arg| {
                    n.with_suffix(subcommand.name.as_str())
                        .with_suffix(arg.name.as_str())
                        .with_suffix(ARG)
                })
                .collect::<Vec<Ident>>();

            let mut tokens = subcommand
                .cmd_args
                .iter_mut()
                .zip(arg_idents.iter())
                .map(|(arg, ident)| {
                    let Arg { name, description, kind, required } = arg;

                    quote! {
                        #(#cooked)*
                        #[allow(missing_docs)]
                        pub static #ident: #arg_path = #arg_path {
                            name: #name,
                            description: #description,
                            kind: #kind,
                            required: #required,
                            options: &[]
                        };
                    }
                })
                .fold(quote! {}, |mut a, b| {
                    a.extend(b);
                    a
                });

            let Subcommand { name, description, .. } = subcommand;

            tokens.extend(quote! {
                #(#cooked)*
                #[allow(missing_docs)]
                pub static #sc_ident: #arg_path = #arg_path {
                    name: #name,
                    description: #description,
                    kind: #subcommand_path,
                    required: false,
                    options: &[#(&#arg_idents),*],
                };
            });

            tokens
        })
        .fold(quote! {}, |mut a, b| {
            a.extend(b);
            a
        });

    let mut arg_idents = cmd_args
        .iter()
        .map(|arg| n.with_suffix(arg.name.replace("-", "_").as_str()).with_suffix(ARG))
        .collect::<Vec<Ident>>();

    let arg_tokens = cmd_args
        .iter_mut()
        .zip(arg_idents.iter())
        .map(|(arg, ident)| {
            let Arg { name, description, kind, required } = arg;

            quote! {
                #(#cooked)*
                #[allow(missing_docs)]
                pub static #ident: #arg_path = #arg_path {
                    name: #name,
                    description: #description,
                    kind: #kind,
                    required: #required,
                    options: &[],
                };
            }
        })
        .fold(quote! {}, |mut a, b| {
            a.extend(b);
            a
        });

    tokens.extend(arg_tokens);
    arg_idents.append(&mut subcommand_idents);

    let variant = if args.len() == 2 {
        quote!(crate::framework::CommandFnType::Multi)
    } else {
        let string: Type = parse_quote!(String);

        let final_arg = args.get(2).unwrap();

        if final_arg.kind == string {
            quote!(crate::framework::CommandFnType::Text)
        } else {
            quote!(crate::framework::CommandFnType::Slash)
        }
    };

    tokens.extend(quote! {
        #(#cooked)*
        #[allow(missing_docs)]
        pub static #n: #command_path = #command_path {
            fun: #variant(#name),
            names: &[#_name, #(#aliases),*],
            desc: #description,
            group: #group,
            examples: &[#(#examples),*],
            required_permissions: #required_permissions,
            can_blacklist: #can_blacklist,
            supports_dm: #supports_dm,
            args: &[#(&#arg_idents),*],
        };
    });

    tokens.extend(quote! {
        #(#cooked)*
        #[allow(missing_docs)]
        #visibility fn #name<'fut> (#(#args),*) -> ::serenity::futures::future::BoxFuture<'fut, ()> {
            use ::serenity::futures::future::FutureExt;

            async move {
                #(#body)*;
            }.boxed()
        }
    });

    tokens.into()
}
