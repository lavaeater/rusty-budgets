use proc_macro::{TokenStream};
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Path};

/// Holds the parsed arguments from `#[command(...)]`
struct CommandArgs {
    aggregate: Path,
    event: Path,
}

impl CommandArgs {
    fn from_attrs(attrs: &[syn::Attribute]) -> syn::Result<Self> {
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


/// Automatically implement `Decision` for a command struct.
///
/// This macro is useful when you want to create a command that, given the current state of the
/// aggregate, produces an event to be stored.
///
/// # Example
///
///
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

/// Holds the parsed arguments from `#[event(...)]`
struct EventArgs {
    aggregate: Path,
}

impl EventArgs {
    fn from_attrs(attrs: &[syn::Attribute]) -> syn::Result<Self> {
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

/// Automatically implement `DomainEvent` for an enum.
///
/// This macro is useful when you have multiple events that can happen to the same aggregate.
/// Instead of having to write `impl DomainEvent<Budget> for Event1`, `impl DomainEvent<Budget> for Event2`,
/// you can just put `#[derive(DomainEvents)]` on your enum and all variants will implement `DomainEvent`.
///
/// # Example
///#[derive(Event)]
/// #[event(aggregate = Budget)]
/// pub struct BudgetCreated {
///     pub budget_id: Uuid,
///     pub name: String,
///     pub user_id: Uuid,
///     pub default: bool,
/// }
///
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


    /// Automatically implement `DomainEvent` for an enum.
    ///
    /// This macro is useful when you have multiple events that can happen to the same aggregate.
    /// Instead of having to write `impl DomainEvent<Budget> for Event1`, `impl DomainEvent<Budget> for Event2`,
    /// you can just put `#[derive(DomainEvents)]` on your enum and all variants will implement `DomainEvent`.
    ///
    /// # Example
    ///
    ///
#[proc_macro_derive(DomainEvents)]
pub fn derive_domain_events(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident; // e.g. BudgetEvent

    let enum_data = match input.data {
        Data::Enum(data_enum) => data_enum,
        _ => {
            return syn::Error::new_spanned(
                name,
                "DomainEvents can only be derived on enums"
            ).to_compile_error().into();
        }
    };

    let variants: Vec<_> = enum_data
        .variants
        .iter()
        .map(|v| {
            let vname = &v.ident;
            quote! {
                #name::#vname(e) => e
            }
        })
        .collect();

    let expanded = quote! {
        impl DomainEvent<Budget> for #name {
            fn aggregate_id(&self) -> <Budget as Aggregate>::Id {
                match self {
                    #( #variants.aggregate_id(), )*
                }
            }

            fn apply(&self, state: &mut Budget) {
                match self {
                    #( #variants.apply(state), )*
                }
            }
        }
    };

    TokenStream::from(expanded)
}
