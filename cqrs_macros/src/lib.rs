// cqrs_macros/src/lib.rs

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Meta, Lit, Data, Fields};

fn derive_command_fn_name(struct_name: &str) -> String {
    // Known verb mappings
    let mut verb_map = std::collections::HashMap::new();
    verb_map.insert("Created", "create");
    verb_map.insert("Added", "add");
    verb_map.insert("Removed", "remove");
    verb_map.insert("Updated", "update");
    verb_map.insert("Connected", "connect");
    verb_map.insert("Reallocated", "reallocate");
    verb_map.insert("Adjusted", "adjust");

    // Split CamelCase into words
    let mut words = Vec::new();
    let mut current = String::new();
    for c in struct_name.chars() {
        if c.is_uppercase() && !current.is_empty() {
            words.push(current);
            current = String::new();
        }
        current.push(c.to_ascii_lowercase());
    }
    if !current.is_empty() {
        words.push(current);
    }

    if let Some(last) = words.last() {
        for (suffix, replacement) in &verb_map {
            if last.eq_ignore_ascii_case(&suffix.to_lowercase()) {
                let mut fn_parts = vec![replacement.to_string()];
                fn_parts.extend(words[..words.len() - 1].iter().cloned());
                return fn_parts.join("_");
            }
        }
    }

    format!("do_{}", words.join("_"))
}

#[proc_macro_derive(DomainEvent, attributes(domain_event, event_id))]
pub fn derive_domain_event(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    // Defaults
    let mut aggregate_name = None;
    let mut command_fn_name = None;
    let mut id_field_override = None;

    // Parse #[domain_event(...)] attributes
    for attr in input.attrs.iter().filter(|a| a.path().is_ident("domain_event")) {
        if let Meta::List(meta_list) = &attr.meta {
            let args: syn::Result<
                syn::punctuated::Punctuated<syn::Meta, syn::Token![,]>
            > = meta_list.parse_args_with(syn::punctuated::Punctuated::parse_terminated);

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
                        } else if name_value.path.is_ident("id") {
                            if let syn::Expr::Lit(syn::ExprLit { lit: Lit::Str(s), .. }) =
                                &name_value.value
                            {
                                id_field_override = Some(s.value());
                            }
                        }
                    }
                }
            }
        }
    }

    let aggregate_name =
        aggregate_name.expect("domain_event attribute must specify aggregate = \"...\"");
    let aggregate_ident: syn::Ident = syn::parse_str(&aggregate_name).unwrap();

    let command_fn_ident: syn::Ident = if let Some(name) = command_fn_name {
        syn::parse_str(&name).unwrap()
    } else {
        let struct_name = name.to_string();
        let fn_name = derive_command_fn_name(&struct_name);
        syn::Ident::new(&fn_name, Span::call_site())
    };

    let apply_fn_name = format!("apply_{}", command_fn_ident);
    let apply_fn_ident = syn::Ident::new(&apply_fn_name, Span::call_site());

    let command_fn_impl_name = format!("{}_impl", command_fn_ident);
    let command_fn_impl_ident = syn::Ident::new(&command_fn_impl_name, Span::call_site());
    let trait_name = syn::Ident::new(&format!("{}Handler", name), Span::call_site());

    // --- Infer aggregate_id field ---
    let mut id_field_ident = None;

    if let Data::Struct(ds) = &input.data {
        if let Fields::Named(fields_named) = &ds.fields {
            if let Some(override_name) = id_field_override {
                id_field_ident = fields_named.named.iter()
                    .find(|f| f.ident.as_ref().unwrap().to_string() == override_name)
                    .map(|f| f.ident.clone())
                    .unwrap_or_else(|| panic!("No field named `{}` found in {}", override_name, name));
            } else {
                let conv_names = vec![
                    "aggregate_id".to_string(),
                    format!("{}_id", aggregate_name.to_lowercase()),
                    "id".to_string(),
                ];
                id_field_ident = fields_named.named.iter()
                    .find(|f| {
                        let fname = f.ident.as_ref().unwrap().to_string();
                        conv_names.iter().any(|c| &fname == c)
                    })
                    .map(|f| f.ident.clone()).expect("REASON");
            }
        }
    }

    let id_field_ident = id_field_ident
        .unwrap_or_else(|| panic!("Could not infer aggregate id field for `{}`. Use #[domain_event(id = \"...\")]", name));

    // --- Extract struct fields (excluding aggregate_id + #[event_id]) ---
    let mut command_params = Vec::new();
    let mut field_assignments = Vec::new();

    if let Data::Struct(ds) = &input.data {
        if let Fields::Named(fields_named) = &ds.fields {
            for field in &fields_named.named {
                if let Some(field_name) = &field.ident {
                    // Skip aggregate_id
                    if field_name == &id_field_ident {
                        continue;
                    }

                    // Skip #[event_id] fields
                    let skip_for_command = field.attrs.iter().any(|a| a.path().is_ident("event_id"));
                    if skip_for_command {
                        continue;
                    }

                    let field_type = &field.ty;
                    command_params.push(quote! { #field_name: #field_type });
                    field_assignments.push(quote! { #field_name });
                }
            }
        }
    }

    // --- Generate code ---
    let expanded = quote! {
        pub trait #trait_name {
            fn #apply_fn_ident(&mut self, event: &#name) -> Uuid;
            fn #command_fn_impl_ident(&self, #(#command_params),*) -> Result<#name, CommandError>;
        }

        impl DomainEvent<#aggregate_ident> for #name {
            fn aggregate_id(&self) -> <#aggregate_ident as Aggregate>::Id {
                self.#id_field_ident
            }

            fn apply(&self, state: &mut #aggregate_ident) -> Uuid {
                state.#apply_fn_ident(self)
            }
        }

        impl #aggregate_ident {
            pub fn #command_fn_ident(&self, #(#command_params),*) -> Result<#name, CommandError> {
                self.#command_fn_impl_ident(#(#field_assignments),*)
            }
        }
    };

    TokenStream::from(expanded)
}