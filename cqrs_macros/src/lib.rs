use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Attribute};

#[proc_macro_derive(Event, attributes(event))]
pub fn derive_event(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    // try to find the #[event(id)] field
    let mut id_field = None;
    if let syn::Data::Struct(ref data) = input.data {
        for field in &data.fields {
            for attr in &field.attrs {
                if attr.path().is_ident("event") {
                    if let Ok(syn::Meta::List(meta_list)) = attr.parse_args() {
                        if meta_list.tokens.to_string().contains("id") {
                            id_field = field.ident.clone();
                        }
                    }
                }
            }
        }
    }

    // if missing -> emit compile_error!
    let id_field = match id_field {
        Some(f) => f,
        None => {
            return quote! {
                compile_error!("You must annotate one field with #[event(id)] to derive Event");
            }
                .into();
        }
    };

    let expanded = quote! {
        impl DomainEvent<Budget> for #name {
            fn aggregate_id(&self) -> <Budget as Aggregate>::Id {
                self.#id_field
            }

            fn apply(&self, state: &mut Budget) {
                todo!("implement apply for {}", stringify!(#name));
            }
        }
    };

    TokenStream::from(expanded)
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

#[proc_macro_derive(Command, attributes(command))]
pub fn derive_command(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    // parse aggregate + event types from #[command(aggregate = "X", event = "Y")]
    let mut agg_ty = None;
    let mut evt_ty = None;

    for attr in input.attrs {
        if attr.path().is_ident("command") {
            if let Ok(syn::Meta::List(meta_list)) = attr.parse_args() {
                for nested in meta_list.iter() {
                    if let syn::NestedMeta::Meta(syn::Meta::NameValue(nv)) = nested {
                        if nv.path.is_ident("aggregate") {
                            if let syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(s), .. }) = &nv.value {
                                agg_ty = Some(syn::Ident::new(&s.value(), s.span()));
                            }
                        }
                        if nv.path.is_ident("event") {
                            if let syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(s), .. }) = &nv.value {
                                evt_ty = Some(syn::Ident::new(&s.value(), s.span()));
                            }
                        }
                    }
                }
            }
        }
    }

    let agg_ty = agg_ty.expect("Command must specify aggregate type");
    let evt_ty = evt_ty.expect("Command must specify event type");

    let expanded = quote! {
        impl Decision<#agg_ty, #evt_ty> for #name {
            fn decide(self, state: Option<&#agg_ty>) -> Result<#evt_ty, CommandError> {
                let state = state.ok_or(CommandError::NotFound(
                    concat!(stringify!(#agg_ty), " not found")
                ))?;
                todo!("implement decide for {}", stringify!(#name));
            }
        }
    };

    TokenStream::from(expanded)
}

