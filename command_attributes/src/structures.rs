use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::{
    braced,
    parse::{Error, Parse, ParseStream, Result},
    spanned::Spanned,
    Attribute, Block, FnArg, Ident, Pat, ReturnType, Stmt, Token, Type, Visibility,
};

use crate::{
    consts::{ARG, SUBCOMMAND},
    util::{Argument, IdentExt2, Parenthesised},
};

fn parse_argument(arg: FnArg) -> Result<Argument> {
    match arg {
        FnArg::Typed(typed) => {
            let pat = typed.pat;
            let kind = typed.ty;

            match *pat {
                Pat::Ident(id) => {
                    let name = id.ident;
                    let mutable = id.mutability;

                    Ok(Argument { mutable, name, kind: *kind })
                }
                Pat::Wild(wild) => {
                    let token = wild.underscore_token;

                    let name = Ident::new("_", token.spans[0]);

                    Ok(Argument { mutable: None, name, kind: *kind })
                }
                _ => Err(Error::new(pat.span(), format_args!("unsupported pattern: {:?}", pat))),
            }
        }
        FnArg::Receiver(_) => {
            Err(Error::new(arg.span(), format_args!("`self` arguments are prohibited: {:?}", arg)))
        }
    }
}

#[derive(Debug)]
pub struct CommandFun {
    /// `#[...]`-style attributes.
    pub attributes: Vec<Attribute>,
    /// Populated cooked attributes. These are attributes outside of the realm of this crate's procedural macros
    /// and will appear in generated output.
    pub visibility: Visibility,
    pub name: Ident,
    pub args: Vec<Argument>,
    pub ret: Type,
    pub body: Vec<Stmt>,
}

impl Parse for CommandFun {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let attributes = input.call(Attribute::parse_outer)?;

        let visibility = input.parse::<Visibility>()?;

        input.parse::<Token![async]>()?;

        input.parse::<Token![fn]>()?;
        let name = input.parse()?;

        // (...)
        let Parenthesised(args) = input.parse::<Parenthesised<FnArg>>()?;

        let ret = match input.parse::<ReturnType>()? {
            ReturnType::Type(_, t) => (*t).clone(),
            ReturnType::Default => Type::Verbatim(quote!(())),
        };

        // { ... }
        let bcont;
        braced!(bcont in input);
        let body = bcont.call(Block::parse_within)?;

        let args = args.into_iter().map(parse_argument).collect::<Result<Vec<_>>>()?;

        Ok(Self { attributes, visibility, name, args, ret, body })
    }
}

impl ToTokens for CommandFun {
    fn to_tokens(&self, stream: &mut TokenStream2) {
        let Self { attributes: _, visibility, name, args, ret, body } = self;

        stream.extend(quote! {
            #visibility async fn #name (#(#args),*) -> #ret {
                #(#body)*
            }
        });
    }
}

#[derive(Debug)]
pub(crate) enum ApplicationCommandOptionType {
    SubCommand,
    SubCommandGroup,
    String,
    Integer,
    Boolean,
    User,
    Channel,
    Role,
    Mentionable,
    Number,
    Unknown,
}

impl ApplicationCommandOptionType {
    pub fn from_str(s: String) -> Self {
        match s.as_str() {
            "SubCommand" => Self::SubCommand,
            "SubCommandGroup" => Self::SubCommandGroup,
            "String" => Self::String,
            "Integer" => Self::Integer,
            "Boolean" => Self::Boolean,
            "User" => Self::User,
            "Channel" => Self::Channel,
            "Role" => Self::Role,
            "Mentionable" => Self::Mentionable,
            "Number" => Self::Number,
            _ => Self::Unknown,
        }
    }
}

impl ToTokens for ApplicationCommandOptionType {
    fn to_tokens(&self, stream: &mut TokenStream2) {
        let path = quote!(
            serenity::model::interactions::application_command::ApplicationCommandOptionType
        );
        let variant = match self {
            ApplicationCommandOptionType::SubCommand => quote!(SubCommand),
            ApplicationCommandOptionType::SubCommandGroup => quote!(SubCommandGroup),
            ApplicationCommandOptionType::String => quote!(String),
            ApplicationCommandOptionType::Integer => quote!(Integer),
            ApplicationCommandOptionType::Boolean => quote!(Boolean),
            ApplicationCommandOptionType::User => quote!(User),
            ApplicationCommandOptionType::Channel => quote!(Channel),
            ApplicationCommandOptionType::Role => quote!(Role),
            ApplicationCommandOptionType::Mentionable => quote!(Mentionable),
            ApplicationCommandOptionType::Number => quote!(Number),
            ApplicationCommandOptionType::Unknown => quote!(Unknown),
        };

        stream.extend(quote! {
            #path::#variant
        });
    }
}

#[derive(Debug)]
pub(crate) struct Arg {
    pub name: String,
    pub description: String,
    pub kind: ApplicationCommandOptionType,
    pub required: bool,
}

impl Arg {
    pub fn as_tokens(&self, ident: &Ident) -> TokenStream2 {
        let arg_path = quote!(crate::framework::Arg);
        let Arg { name, description, kind, required } = self;

        quote! {
            #[allow(missing_docs)]
            pub static #ident: #arg_path = #arg_path {
                name: #name,
                description: #description,
                kind: #kind,
                required: #required,
                options: &[]
            };
        }
    }
}

impl Default for Arg {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            kind: ApplicationCommandOptionType::String,
            required: false,
        }
    }
}

#[derive(Debug)]
pub(crate) struct Subcommand {
    pub name: String,
    pub description: String,
    pub cmd_args: Vec<Arg>,
}

impl Subcommand {
    pub fn as_tokens(&mut self, ident: &Ident) -> TokenStream2 {
        let arg_path = quote!(crate::framework::Arg);
        let subcommand_path = ApplicationCommandOptionType::SubCommand;

        let arg_idents = self
            .cmd_args
            .iter()
            .map(|arg| ident.with_suffix(arg.name.as_str()).with_suffix(ARG))
            .collect::<Vec<Ident>>();

        let mut tokens = self
            .cmd_args
            .iter_mut()
            .zip(arg_idents.iter())
            .map(|(arg, ident)| arg.as_tokens(ident))
            .fold(quote! {}, |mut a, b| {
                a.extend(b);
                a
            });

        let Subcommand { name, description, .. } = self;

        tokens.extend(quote! {
            #[allow(missing_docs)]
            pub static #ident: #arg_path = #arg_path {
                name: #name,
                description: #description,
                kind: #subcommand_path,
                required: false,
                options: &[#(&#arg_idents),*],
            };
        });

        tokens
    }
}

impl Default for Subcommand {
    fn default() -> Self {
        Self { name: String::new(), description: String::new(), cmd_args: vec![] }
    }
}

impl Subcommand {
    pub(crate) fn new(name: String) -> Self {
        Self { name, ..Default::default() }
    }
}

#[derive(Debug)]
pub(crate) struct SubcommandGroup {
    pub name: String,
    pub description: String,
    pub subcommands: Vec<Subcommand>,
}

impl SubcommandGroup {
    pub fn as_tokens(&mut self, ident: &Ident) -> TokenStream2 {
        let arg_path = quote!(crate::framework::Arg);
        let subcommand_group_path = ApplicationCommandOptionType::SubCommandGroup;

        let arg_idents = self
            .subcommands
            .iter()
            .map(|arg| {
                ident
                    .with_suffix(self.name.as_str())
                    .with_suffix(arg.name.as_str())
                    .with_suffix(SUBCOMMAND)
            })
            .collect::<Vec<Ident>>();

        let mut tokens = self
            .subcommands
            .iter_mut()
            .zip(arg_idents.iter())
            .map(|(subcommand, ident)| subcommand.as_tokens(ident))
            .fold(quote! {}, |mut a, b| {
                a.extend(b);
                a
            });

        let SubcommandGroup { name, description, .. } = self;

        tokens.extend(quote! {
            #[allow(missing_docs)]
            pub static #ident: #arg_path = #arg_path {
                name: #name,
                description: #description,
                kind: #subcommand_group_path,
                required: false,
                options: &[#(&#arg_idents),*],
            };
        });

        tokens
    }
}

impl Default for SubcommandGroup {
    fn default() -> Self {
        Self { name: String::new(), description: String::new(), subcommands: vec![] }
    }
}

impl SubcommandGroup {
    pub(crate) fn new(name: String) -> Self {
        Self { name, ..Default::default() }
    }
}

#[derive(Debug, Default)]
pub(crate) struct Options {
    pub aliases: Vec<String>,
    pub description: String,
    pub group: String,
    pub examples: Vec<String>,
    pub can_blacklist: bool,
    pub supports_dm: bool,
    pub cmd_args: Vec<Arg>,
    pub subcommands: Vec<Subcommand>,
    pub subcommand_groups: Vec<SubcommandGroup>,
}

impl Options {
    #[inline]
    pub fn new() -> Self {
        Self { group: "None".to_string(), ..Default::default() }
    }
}
