// cqrs_macros/src/lib.rs
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Meta, Lit};

#[proc_macro_derive(DomainEvent, attributes(domain_event))]
pub fn derive_domain_event(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    // Defaults
    let mut aggregate_name = None;
    let mut command_fn_name = None;
    let mut command_error_name = None;

    // Iterate over attributes
    for attr in input.attrs.iter().filter(|a| a.path().is_ident("domain_event")) {
        if let Meta::List(meta_list) = &attr.meta {
            let args: syn::Result<syn::punctuated::Punctuated<syn::Meta, syn::Token![,]>> =
                meta_list.parse_args_with(syn::punctuated::Punctuated::parse_terminated);

            if let Ok(args) = args {
                for arg in args {
                    if let Meta::NameValue(name_value) = arg {
                        if name_value.path.is_ident("aggregate") {
                            if let syn::Expr::Lit(syn::ExprLit { lit: Lit::Str(s), .. }) =
                                &name_value.value
                            {
                                aggregate_name = Some(s.value());
                            }
                        } else if name_value.path.is_ident("command_fn") {
                            if let syn::Expr::Lit(syn::ExprLit { lit: Lit::Str(s), .. }) =
                                &name_value.value
                            {
                                command_fn_name = Some(s.value());
                            }
                        } else if name_value.path.is_ident("command_error") {
                            if let syn::Expr::Lit(syn::ExprLit { lit: Lit::Str(s), .. }) =
                                &name_value.value
                            {
                                command_error_name = Some(s.value());
                            }
                        }
                    }
                }
            }
        }
    }

    let aggregate_name =
        aggregate_name.expect("domain_event attribute must specify aggregate = \"...\"");
    let command_fn_name =
        command_fn_name.expect("domain_event attribute must specify command_fn = \"...\"");
    let command_error_name =
        command_error_name.expect("domain_event attribute must specify command_error type = \"...\"");

    let aggregate_ident: syn::Ident = syn::parse_str(&aggregate_name).unwrap();
    let command_fn_ident: syn::Ident = syn::parse_str(&command_fn_name).unwrap();
    let command_error_ident: syn::Ident = syn::parse_str(&command_error_name).unwrap();

    // Generate working stubs
    let expanded = quote! {
        impl DomainEvent<#aggregate_ident> for #name {
            fn aggregate_id(&self) -> <#aggregate_ident as Aggregate>::Id {
                todo!("implement aggregate_id for {}", stringify!(#name));
            }

            fn apply(&self, state: &mut #aggregate_ident) {
                todo!("implement apply for {}", stringify!(#name));
            }
        }

        impl #aggregate_ident {
            pub fn #command_fn_ident(&mut self, args: impl Into<#name>) -> Result<#name, #command_error_ident> {
                let event: #name = args.into();

                // Here you can perform business-rule validation:
                // if some_condition {
                //     return Err(CommandError::ValidationFailed("reason".into()));
                // }

                Ok(event)
            }
        }
    };

    TokenStream::from(expanded)
}