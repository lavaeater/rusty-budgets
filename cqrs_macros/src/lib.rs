use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, DeriveInput, Data, Fields, Ident,
    parse::{Parse, ParseStream},
    punctuated::Punctuated, Token, Path,
};


/// Holds the parsed arguments from `#[event(...)]`
struct EventArgs {
    pub aggregate: Path,
}

impl EventArgs {
    pub fn from_attrs(attrs: &[syn::Attribute]) -> syn::Result<Self> {
        let mut aggregate: Option<Path> = None;

        for attr in attrs {
            if attr.path().is_ident("event") {
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("aggregate") {
                        let value: Path = meta.value()?.parse()?;
                        aggregate = Some(value);
                        Ok(())
                    } else {
                        Err(meta.error("unsupported attribute inside #[event(..)]"))
                    }
                })?;
            }
        }

        Ok(EventArgs {
            aggregate: aggregate.ok_or_else(|| {
                syn::Error::new(proc_macro2::Span::call_site(), "missing `aggregate = ...`")
            })?,
        })
    }
}

#[proc_macro_derive(Event, attributes(event))]
pub fn derive_event(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    let name = input.ident;

    let args = EventArgs::from_attrs(&input.attrs)
        .expect("Failed to parse #[event(...)] attributes");

    let aggregate = args.aggregate;

    let expanded = quote! {
        impl DomainEvent<#aggregate> for #name {
            fn aggregate_id(&self) -> <#aggregate as Aggregate>::Id {
                todo!("Return the aggregate id for {}", stringify!(#name));
            }

            fn apply(&self, state: &mut #aggregate) {
                todo!("Apply {} to the aggregate", stringify!(#name));
            }
        }
    };

    expanded.into()
}



#[proc_macro_derive(EventEnum)]
pub fn derive_event_enum(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let syn::Data::Enum(data_enum) = &input.data else {
        panic!("#[derive(EventEnum)] only works on enums");
    };

    let arms_agg_id = data_enum.variants.iter().map(|v| {
        let vname = &v.ident;
        quote! { #name::#vname(e) => e.aggregate_id(), }
    });

    let arms_apply = data_enum.variants.iter().map(|v| {
        let vname = &v.ident;
        quote! { #name::#vname(e) => e.apply(state), }
    });

    let expanded = quote! {
        impl DomainEvent<Budget> for #name {
            fn aggregate_id(&self) -> <Budget as Aggregate>::Id {
                match self {
                    #(#arms_agg_id)*
                }
            }
            fn apply(&self, state: &mut Budget) {
                match self {
                    #(#arms_apply)*
                }
            }
        }
    };

    TokenStream::from(expanded)
}

/// Holds the parsed arguments from `#[command(...)]`
struct CommandArgs {
    pub aggregate: Path,
    pub event: Path,
}

impl CommandArgs {
    pub fn from_attrs(attrs: &[syn::Attribute]) -> syn::Result<Self> {
        let mut aggregate: Option<Path> = None;
        let mut event: Option<Path> = None;

        for attr in attrs {
            if attr.path().is_ident("command") {
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("aggregate") {
                        let value: Path = meta.value()?.parse()?;
                        aggregate = Some(value);
                        Ok(())
                    } else if meta.path.is_ident("event") {
                        let value: Path = meta.value()?.parse()?;
                        event = Some(value);
                        Ok(())
                    } else {
                        Err(meta.error("unsupported attribute inside #[command(..)]"))
                    }
                })?;
            }
        }

        Ok(CommandArgs {
            aggregate: aggregate.ok_or_else(|| {
                syn::Error::new(proc_macro2::Span::call_site(), "missing `aggregate = ...`")
            })?,
            event: event.ok_or_else(|| {
                syn::Error::new(proc_macro2::Span::call_site(), "missing `event = ...`")
            })?,
        })
    }
}

#[proc_macro_derive(Command, attributes(command))]
pub fn derive_command(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    let name = input.ident;

    let args = CommandArgs::from_attrs(&input.attrs)
        .expect("Failed to parse #[command(...)] attributes");

    let aggregate = args.aggregate;
    let event = args.event;

    let expanded = quote! {
        impl Decision<#aggregate, #event> for #name {
            fn decide(self, state: Option<&#aggregate>) -> Result<#event, CommandError> {
                todo!("Implement decision logic for {}", stringify!(#name));
            }
        }
    };

    expanded.into()
}

