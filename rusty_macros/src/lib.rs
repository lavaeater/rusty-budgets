extern crate proc_macro;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, ItemImpl, Path};
use convert_case::{Case, Casing};

/// Usage: #[find_by_uuid(post::Model)]
/// Generates a paginated list function for a model.
/// Usage: #[list_at_page(post::Model)]
/// Generates: pub async fn list_posts_at_page(db: &DbConn, page: u64, per_page: u64) -> Result<(Vec<post::Model>, u64), DbErr>
#[proc_macro_attribute]
pub fn list_at_page(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse the model path (e.g., post::Model)
    let model_path = parse_macro_input!(attr as Path);

    // Get the model type and its prefix (e.g., post::Model -> post, Model)
    let segments = &model_path.segments;
    let (prefix, model_ty) = if segments.len() >= 1 {
        if segments.len() == 1 {
            (quote! {}, segments[0].ident.clone())
        } else {
            // Create a clearer range expression by first calculating the last index
            let last_index = segments.len() - 1;
            let prefix = segments.iter().take(last_index).map(|s| s.ident.clone()).collect::<Vec<_>>();
            let model_ty = segments[last_index].ident.clone();
            (quote! { #(#prefix)::* }, model_ty)
        }
    } else {
        // This should be caught by the parser, but handle it just in case
        return quote! {
            compile_error!("Expected a path with at least one segment");
        }
        .into();
    };
    let model_ty_str = model_ty.to_string();

    // Parse the input as an impl block
    let mut item_impl = parse_macro_input!(item as ItemImpl);

    // Generate the function name based on the model name (e.g., post -> list_posts_at_page)
    let model_name = model_ty_str.trim_end_matches("Model").to_lowercase();
    let fn_name = format_ident!("list_{}_at_page", model_name);
    
    // Get the model type identifier and struct name
    let model_ty_ident = format_ident!("{}", model_ty_str);
    let model_struct_name_ident = format_ident!("{}", model_ty_str.trim_end_matches("Model"));

    // Generate the list_at_page function
    let method = syn::parse_quote! {
        pub async fn #fn_name(
            db: &DbConn,
            page: u64,
            per_page: u64,
        ) -> Result<(Vec<#prefix #model_ty_ident>, u64), DbErr> {
            // Setup paginator
            let paginator = #model_struct_name_ident::find()
                .order_by_asc(sea_orm::ColumnTrait::default())
                .paginate(db, per_page);
                
            let num_pages = paginator.num_pages().await?;

            // Fetch paginated items
            paginator
                .fetch_page(if page == 0 { 0 } else { page - 1 })
                .await
                .map(|items| (items, num_pages))
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
