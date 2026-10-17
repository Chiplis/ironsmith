//! `#[derive(TagKeyWalk)]`: visit and rewrite every `TagKey` a value carries.
//!
//! The derive walks every field of a struct or of each enum variant and
//! delegates to the field's own `TagKeyWalk` implementation. A field marked
//! `#[tag_walk(skip)]` is a leaf: it carries no reference keys, or its keys
//! are not part of the value's semantic identity.

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Fields, GenericParam, Index, parse_macro_input, parse_quote};

#[proc_macro_derive(TagKeyWalk, attributes(tag_walk))]
pub fn derive_tag_key_walk(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let mut generics = input.generics.clone();
    for param in generics.params.iter_mut() {
        if let GenericParam::Type(type_param) = param {
            type_param
                .bounds
                .push(parse_quote!(::ironsmith_core::tag::TagKeyWalk));
        }
    }
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let player_ref = has_type_flag(&input.attrs, "player_ref");
    let (visit_body, map_body, object_body, player_body) = match &input.data {
        Data::Struct(data) => {
            let (visit, map, object) = fields_walk(&data.fields, quote!(self), true);
            let player = fields_player_walk(&data.fields);
            (visit, map, object, player)
        }
        Data::Enum(data) => {
            let mut visit_arms = Vec::new();
            let mut map_arms = Vec::new();
            let mut object_arms = Vec::new();
            let mut player_arms = Vec::new();
            for variant in &data.variants {
                let variant_name = &variant.ident;
                let (pattern, visit, map, object) = variant_walk(&variant.fields);
                let player = variant_player_walk(&variant.fields);
                visit_arms.push(quote!(Self::#variant_name #pattern => { #visit }));
                map_arms.push(quote!(Self::#variant_name #pattern => { #map }));
                object_arms.push(quote!(Self::#variant_name #pattern => { #object }));
                player_arms.push(quote!(Self::#variant_name #pattern => { #player }));
            }
            let visit = if visit_arms.is_empty() {
                quote!(match *self {})
            } else {
                quote!(match self { #(#visit_arms)* })
            };
            let map = if map_arms.is_empty() {
                quote!(match *self {})
            } else {
                quote!(match self { #(#map_arms)* })
            };
            let object = if object_arms.is_empty() {
                quote!(match *self {})
            } else {
                quote!(match self { #(#object_arms)* })
            };
            let player = if player_arms.is_empty() {
                quote!(match *self {})
            } else {
                quote!(match self { #(#player_arms)* })
            };
            (visit, map, object, player)
        }
        Data::Union(_) => {
            return syn::Error::new_spanned(name, "TagKeyWalk cannot be derived for unions")
                .to_compile_error()
                .into();
        }
    };
    // A `#[tag_walk(player_ref)]` type is itself a player reference: offer
    // it to the visitor before descending into its fields.
    let player_hook = if player_ref {
        quote!(if f(&mut *self as &mut dyn ::std::any::Any) {
            return;
        })
    } else {
        quote!()
    };
    quote! {
        impl #impl_generics ::ironsmith_core::tag::TagKeyWalk for #name #type_generics #where_clause {
            fn for_each_tag_key(&self, f: &mut dyn FnMut(&::ironsmith_core::tag::TagKey)) {
                let _ = &f;
                #visit_body
            }
            fn for_each_object_ref(&self, f: &mut dyn FnMut(&::ironsmith_core::filter_model::ObjectRef)) {
                let _ = &f;
                #object_body
            }
            fn map_tag_keys(&mut self, f: &mut dyn FnMut(&mut ::ironsmith_core::tag::TagKey)) {
                let _ = &f;
                #map_body
            }
            fn map_player_refs(&mut self, f: &mut dyn FnMut(&mut dyn ::std::any::Any) -> bool) {
                let _ = &f;
                #player_hook
                #player_body
            }
        }
    }
    .into()
}

fn is_skipped(field: &syn::Field) -> bool {
    field.attrs.iter().any(|attr| {
        if !attr.path().is_ident("tag_walk") {
            return false;
        }
        let mut skip = false;
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("skip") {
                skip = true;
            }
            Ok(())
        });
        skip
    })
}

/// Walk the fields of a struct through `receiver` (`self`).
fn fields_walk(
    fields: &Fields,
    receiver: proc_macro2::TokenStream,
    _is_struct: bool,
) -> (proc_macro2::TokenStream, proc_macro2::TokenStream, proc_macro2::TokenStream) {
    let mut visit = Vec::new();
    let mut map = Vec::new();
    let mut object = Vec::new();
    match fields {
        Fields::Named(named) => {
            for field in &named.named {
                if is_skipped(field) {
                    continue;
                }
                let ident = field.ident.as_ref().unwrap();
                visit.push(quote!(::ironsmith_core::tag::TagKeyWalk::for_each_tag_key(&#receiver.#ident, f);));
                object.push(quote!(::ironsmith_core::tag::TagKeyWalk::for_each_object_ref(&#receiver.#ident, f);));
                map.push(quote!(::ironsmith_core::tag::TagKeyWalk::map_tag_keys(&mut #receiver.#ident, f);));
            }
        }
        Fields::Unnamed(unnamed) => {
            for (position, field) in unnamed.unnamed.iter().enumerate() {
                if is_skipped(field) {
                    continue;
                }
                let index = Index::from(position);
                visit.push(quote!(::ironsmith_core::tag::TagKeyWalk::for_each_tag_key(&#receiver.#index, f);));
                object.push(quote!(::ironsmith_core::tag::TagKeyWalk::for_each_object_ref(&#receiver.#index, f);));
                map.push(quote!(::ironsmith_core::tag::TagKeyWalk::map_tag_keys(&mut #receiver.#index, f);));
            }
        }
        Fields::Unit => {}
    }
    (quote!(#(#visit)*), quote!(#(#map)*), quote!(#(#object)*))
}

/// Pattern and walks for one enum variant; bound names are `f0`, `f1`, … or
/// the field names.
fn variant_walk(
    fields: &Fields,
) -> (
    proc_macro2::TokenStream,
    proc_macro2::TokenStream,
    proc_macro2::TokenStream,
    proc_macro2::TokenStream,
) {
    let mut bindings = Vec::new();
    let mut visit = Vec::new();
    let mut map = Vec::new();
    let mut object = Vec::new();
    match fields {
        Fields::Named(named) => {
            for field in &named.named {
                let ident = field.ident.as_ref().unwrap();
                if is_skipped(field) {
                    continue;
                }
                bindings.push(quote!(#ident));
                visit.push(quote!(::ironsmith_core::tag::TagKeyWalk::for_each_tag_key(#ident, f);));
                object.push(quote!(::ironsmith_core::tag::TagKeyWalk::for_each_object_ref(#ident, f);));
                map.push(quote!(::ironsmith_core::tag::TagKeyWalk::map_tag_keys(#ident, f);));
            }
            let pattern = quote!({ #(#bindings,)* .. });
            (pattern, quote!(#(#visit)*), quote!(#(#map)*), quote!(#(#object)*))
        }
        Fields::Unnamed(unnamed) => {
            let mut pattern_parts = Vec::new();
            for (position, field) in unnamed.unnamed.iter().enumerate() {
                if is_skipped(field) {
                    pattern_parts.push(quote!(_));
                    continue;
                }
                let ident = format_ident!("f{position}");
                pattern_parts.push(quote!(#ident));
                visit.push(quote!(::ironsmith_core::tag::TagKeyWalk::for_each_tag_key(#ident, f);));
                object.push(quote!(::ironsmith_core::tag::TagKeyWalk::for_each_object_ref(#ident, f);));
                map.push(quote!(::ironsmith_core::tag::TagKeyWalk::map_tag_keys(#ident, f);));
            }
            let pattern = quote!(( #(#pattern_parts),* ));
            (pattern, quote!(#(#visit)*), quote!(#(#map)*), quote!(#(#object)*))
        }
        Fields::Unit => (quote!(), quote!(), quote!(), quote!()),
    }
}

/// Whether the item carries `#[tag_walk(<flag>)]`.
fn has_type_flag(attrs: &[syn::Attribute], flag: &str) -> bool {
    attrs.iter().any(|attr| {
        if !attr.path().is_ident("tag_walk") {
            return false;
        }
        let mut found = false;
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident(flag) {
                found = true;
            }
            Ok(())
        });
        found
    })
}

/// The player-reference walk over a struct's fields (`self.<field>`).
fn fields_player_walk(fields: &Fields) -> proc_macro2::TokenStream {
    let mut player = Vec::new();
    match fields {
        Fields::Named(named) => {
            for field in &named.named {
                if is_skipped(field) {
                    continue;
                }
                let ident = field.ident.as_ref().unwrap();
                player.push(quote!(::ironsmith_core::tag::TagKeyWalk::map_player_refs(&mut self.#ident, f);));
            }
        }
        Fields::Unnamed(unnamed) => {
            for (position, field) in unnamed.unnamed.iter().enumerate() {
                if is_skipped(field) {
                    continue;
                }
                let index = Index::from(position);
                player.push(quote!(::ironsmith_core::tag::TagKeyWalk::map_player_refs(&mut self.#index, f);));
            }
        }
        Fields::Unit => {}
    }
    quote!(#(#player)*)
}

/// The player-reference walk over one enum variant's bindings, matching the
/// names `variant_walk` binds.
fn variant_player_walk(fields: &Fields) -> proc_macro2::TokenStream {
    let mut player = Vec::new();
    match fields {
        Fields::Named(named) => {
            for field in &named.named {
                if is_skipped(field) {
                    continue;
                }
                let ident = field.ident.as_ref().unwrap();
                player.push(quote!(::ironsmith_core::tag::TagKeyWalk::map_player_refs(#ident, f);));
            }
        }
        Fields::Unnamed(unnamed) => {
            for (position, field) in unnamed.unnamed.iter().enumerate() {
                if is_skipped(field) {
                    continue;
                }
                let ident = format_ident!("f{position}");
                player.push(quote!(::ironsmith_core::tag::TagKeyWalk::map_player_refs(#ident, f);));
            }
        }
        Fields::Unit => {}
    }
    quote!(#(#player)*)
}
