#![deny(rust_2018_idioms)]
#![deny(broken_intra_doc_links)]

use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::quote;
use syn::{parse::Error, parse_macro_input, parse_quote, spanned::Spanned, Lit, Type};
use uuid::Uuid;

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
    enum LastItem {
        Fun,
        SubFun,
        SubGroup,
        SubGroupFun,
    }

    let mut fun = parse_macro_input!(input as CommandFun);

    let _name = if !attr.is_empty() {
        parse_macro_input!(attr as Lit).to_str()
    } else {
        fun.name.to_string()
    };

    let mut hooks: Vec<Ident> = Vec::new();
    let mut options = Options::new();
    let mut last_desc = LastItem::Fun;

    for attribute in &fun.attributes {
        let span = attribute.span();
        let values = propagate_err!(parse_values(attribute));

        let name = values.name.to_string();
        let name = &name[..];

        match name {
            "subcommand" => {
                let new_subcommand = Subcommand::new(propagate_err!(attributes::parse(values)));

                if let Some(subcommand_group) = options.subcommand_groups.last_mut() {
                    last_desc = LastItem::SubGroupFun;
                    subcommand_group.subcommands.push(new_subcommand);
                } else {
                    last_desc = LastItem::SubFun;
                    options.subcommands.push(new_subcommand);
                }
            }
            "subcommandgroup" => {
                let new_group = SubcommandGroup::new(propagate_err!(attributes::parse(values)));
                last_desc = LastItem::SubGroup;

                options.subcommand_groups.push(new_group);
            }
            "arg" => {
                let arg = propagate_err!(attributes::parse(values));

                match last_desc {
                    LastItem::Fun => {
                        options.cmd_args.push(arg);
                    }
                    LastItem::SubFun => {
                        options.subcommands.last_mut().unwrap().cmd_args.push(arg);
                    }
                    LastItem::SubGroup => {
                        panic!("Argument not expected under subcommand group");
                    }
                    LastItem::SubGroupFun => {
                        options
                            .subcommand_groups
                            .last_mut()
                            .unwrap()
                            .subcommands
                            .last_mut()
                            .unwrap()
                            .cmd_args
                            .push(arg);
                    }
                }
            }
            "example" => {
                options.examples.push(propagate_err!(attributes::parse(values)));
            }
            "description" => {
                let line: String = propagate_err!(attributes::parse(values));

                match last_desc {
                    LastItem::Fun => {
                        util::append_line(&mut options.description, line);
                    }
                    LastItem::SubFun => {
                        util::append_line(
                            &mut options.subcommands.last_mut().unwrap().description,
                            line,
                        );
                    }
                    LastItem::SubGroup => {
                        util::append_line(
                            &mut options.subcommand_groups.last_mut().unwrap().description,
                            line,
                        );
                    }
                    LastItem::SubGroupFun => {
                        util::append_line(
                            &mut options
                                .subcommand_groups
                                .last_mut()
                                .unwrap()
                                .subcommands
                                .last_mut()
                                .unwrap()
                                .description,
                            line,
                        );
                    }
                }
            }
            "hook" => {
                hooks.push(propagate_err!(attributes::parse(values)));
            }
            _ => {
                match_options!(name, values, options, span => [
                    aliases;
                    group;
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
        can_blacklist,
        supports_dm,
        mut cmd_args,
        mut subcommands,
        mut subcommand_groups,
    } = options;

    let visibility = fun.visibility;
    let name = fun.name.clone();
    let body = fun.body;

    let root_ident = name.with_suffix(COMMAND);

    let command_path = quote!(crate::framework::Command);

    populate_fut_lifetimes_on_refs(&mut fun.args);

    let mut subcommand_group_idents = subcommand_groups
        .iter()
        .map(|subcommand| {
            root_ident
                .with_suffix(subcommand.name.replace("-", "_").as_str())
                .with_suffix(SUBCOMMAND_GROUP)
        })
        .collect::<Vec<Ident>>();

    let mut subcommand_idents = subcommands
        .iter()
        .map(|subcommand| {
            root_ident
                .with_suffix(subcommand.name.replace("-", "_").as_str())
                .with_suffix(SUBCOMMAND)
        })
        .collect::<Vec<Ident>>();

    let mut arg_idents = cmd_args
        .iter()
        .map(|arg| root_ident.with_suffix(arg.name.replace("-", "_").as_str()).with_suffix(ARG))
        .collect::<Vec<Ident>>();

    let mut tokens = quote! {};

    tokens.extend(
        subcommand_groups
            .iter_mut()
            .zip(subcommand_group_idents.iter())
            .map(|(group, group_ident)| group.as_tokens(group_ident))
            .fold(quote! {}, |mut a, b| {
                a.extend(b);
                a
            }),
    );

    tokens.extend(
        subcommands
            .iter_mut()
            .zip(subcommand_idents.iter())
            .map(|(subcommand, sc_ident)| subcommand.as_tokens(sc_ident))
            .fold(quote! {}, |mut a, b| {
                a.extend(b);
                a
            }),
    );

    tokens.extend(
        cmd_args.iter_mut().zip(arg_idents.iter()).map(|(arg, ident)| arg.as_tokens(ident)).fold(
            quote! {},
            |mut a, b| {
                a.extend(b);
                a
            },
        ),
    );

    arg_idents.append(&mut subcommand_group_idents);
    arg_idents.append(&mut subcommand_idents);

    let args = fun.args;

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
        #[allow(missing_docs)]
        pub static #root_ident: #command_path = #command_path {
            fun: #variant(#name),
            names: &[#_name, #(#aliases),*],
            desc: #description,
            group: #group,
            examples: &[#(#examples),*],
            can_blacklist: #can_blacklist,
            supports_dm: #supports_dm,
            args: &[#(&#arg_idents),*],
            hooks: &[#(&#hooks),*],
        };

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

#[proc_macro_attribute]
pub fn check(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let mut fun = parse_macro_input!(input as CommandFun);

    let n = fun.name.clone();
    let name = n.with_suffix(HOOK);
    let fn_name = n.with_suffix(CHECK);
    let visibility = fun.visibility;

    let body = fun.body;
    let ret = fun.ret;
    populate_fut_lifetimes_on_refs(&mut fun.args);
    let args = fun.args;

    let hook_path = quote!(crate::framework::Hook);
    let uuid = Uuid::new_v4().as_u128();

    (quote! {
        #[allow(missing_docs)]
        #visibility fn #fn_name<'fut>(#(#args),*) -> ::serenity::futures::future::BoxFuture<'fut, #ret> {
            use ::serenity::futures::future::FutureExt;

            async move {
                let _output: #ret = { #(#body)* };
                #[allow(unreachable_code)]
                _output
            }.boxed()
        }

        #[allow(missing_docs)]
        pub static #name: #hook_path = #hook_path {
            fun: #fn_name,
            uuid: #uuid,
        };
    })
    .into()
}
