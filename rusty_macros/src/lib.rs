extern crate proc_macro;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, ItemImpl, Path};
use convert_case::{Case, Casing};

/// Usage: #[find_by_uuid(post::Model)]
#[proc_macro_attribute]
pub fn find_by_uuid(attr: TokenStream, item: TokenStream) -> TokenStream {
    // With syn 2.0, we can parse the attribute token stream directly into the expected type.
    // For `#[find_by_id(post::Model)]`, the argument is a `Path`.
    let model_path = parse_macro_input!(attr as Path);

    // Get the model type and its prefix (e.g., post::Model -> post, Model)
    let segments = &model_path.segments;
    let (prefix, _model_ty_str) = if segments.len() >= 1 { // Allow single segment paths like `Model` or `module::Model`
        if segments.len() == 1 {
            (segments[0].ident.to_string(), segments[0].ident.to_string())
        } else {
            // Use the second to last segment as prefix if available, otherwise first. e.g. entity::post::Model -> post
            // Or post::Model -> post
            let prefix_segment = if segments.len() > 1 { &segments[segments.len()-2].ident } else { &segments[0].ident };
            (prefix_segment.to_string(), segments.last().unwrap().ident.to_string())
        }
    } else {
        panic!("Model path must have at least one segment, e.g., PostModel or post::Model");
    };

    // Function name: find_<prefix>_by_id
    let fn_name = format_ident!("find_{}_by_id", prefix.to_case(Case::Snake));
    let model_ty_ident = quote! { #model_path }; // Use the full path for the type
    // The actual struct/enum to call find_by_id on. Assumes it's the last segment of the path.
    let model_struct_name_ident = &segments.last().unwrap().ident;


    // Parse the impl block the attribute is attached to
    let mut item_impl = parse_macro_input!(item as ItemImpl);

    // Generate the function
    // Ensure DbConn, Uuid, DbErr are in scope where this macro is used.
    // Also, sea_orm::EntityTrait must be in scope for the ::Model associated type.
    let method = syn::parse_quote! {
        pub async fn #fn_name(db: &DbConn, id: Uuid) -> Result<Option<<#model_ty_ident as sea_orm::EntityTrait>::Model>, DbErr> {
            #model_struct_name_ident::find_by_id(id).one(db).await
        }
    };

    // Add the function to the impl block
    item_impl.items.push(syn::ImplItem::Fn(method));

    // Output the modified impl block
    TokenStream::from(quote! {
        #item_impl
    })
}

#[proc_macro_attribute]
pub fn find_by_id(attr: TokenStream, item: TokenStream) -> TokenStream {
    // With syn 2.0, we can parse the attribute token stream directly into the expected type.
    // For `#[find_by_id(post::Model)]`, the argument is a `Path`.
    let model_path = parse_macro_input!(attr as Path);

    // Get the model type and its prefix (e.g., post::Model -> post, Model)
    let segments = &model_path.segments;
    let (prefix, _model_ty_str) = if segments.len() >= 1 { // Allow single segment paths like `Model` or `module::Model`
        if segments.len() == 1 {
            (segments[0].ident.to_string(), segments[0].ident.to_string())
        } else {
            // Use the second to last segment as prefix if available, otherwise first. e.g. entity::post::Model -> post
            // Or post::Model -> post
            let prefix_segment = if segments.len() > 1 { &segments[segments.len()-2].ident } else { &segments[0].ident };
            (prefix_segment.to_string(), segments.last().unwrap().ident.to_string())
        }
    } else {
        panic!("Model path must have at least one segment, e.g., PostModel or post::Model");
    };

    // Function name: find_<prefix>_by_id
    let fn_name = format_ident!("find_{}_by_id", prefix.to_case(Case::Snake));
    let model_ty_ident = quote! { #model_path }; // Use the full path for the type
    // The actual struct/enum to call find_by_id on. Assumes it's the last segment of the path.
    let model_struct_name_ident = &segments.last().unwrap().ident;


    // Parse the impl block the attribute is attached to
    let mut item_impl = parse_macro_input!(item as ItemImpl);

    // Generate the function
    // Ensure DbConn, Uuid, DbErr are in scope where this macro is used.
    // Also, sea_orm::EntityTrait must be in scope for the ::Model associated type.
    let method = syn::parse_quote! {
        pub async fn #fn_name(db: &DbConn, id: i32) -> Result<Option<<#model_ty_ident as sea_orm::EntityTrait>::Model>, DbErr> {
            #model_struct_name_ident::find_by_id(id).one(db).await
        }
    };

    // Add the function to the impl block
    item_impl.items.push(syn::ImplItem::Fn(method));

    // Output the modified impl block
    TokenStream::from(quote! {
        #item_impl
    })
}


#[cfg(test)]
mod tests {
    // use super::*;

    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
