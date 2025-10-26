//! Derive macros for tryparse
//!
//! This crate provides derive macros for automatically generating
//! schema information and deserialization logic from Rust types.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields, GenericArgument, PathArguments, Type};

/// Derives the `SchemaInfo` trait for a struct or enum.
///
/// # Example
///
/// ```ignore
/// use tryparse::SchemaInfo;
///
/// #[derive(SchemaInfo)]
/// struct User {
///     name: String,
///     age: u32,
/// }
///
/// let schema = User::schema();
/// // Schema::Object { name: "User", fields: [...] }
/// ```
#[proc_macro_derive(SchemaInfo)]
pub fn derive_schema_info(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let schema_impl = match &input.data {
        Data::Struct(data_struct) => generate_struct_schema(name, data_struct),
        Data::Enum(data_enum) => generate_enum_schema(name, data_enum),
        Data::Union(_) => {
            return syn::Error::new_spanned(input, "SchemaInfo cannot be derived for unions")
                .to_compile_error()
                .into();
        }
    };

    let expanded = quote! {
        impl #impl_generics ::tryparse::schema::SchemaInfo for #name #ty_generics #where_clause {
            fn schema() -> ::tryparse::schema::Schema {
                #schema_impl
            }
        }
    };

    TokenStream::from(expanded)
}

fn generate_struct_schema(name: &syn::Ident, data: &syn::DataStruct) -> proc_macro2::TokenStream {
    let name_str = name.to_string();

    match &data.fields {
        Fields::Named(fields) => {
            let field_defs = fields.named.iter().map(|f| {
                let field_name = f.ident.as_ref().unwrap().to_string();
                let field_type = &f.ty;

                quote! {
                    ::tryparse::schema::Field::new(
                        #field_name,
                        <#field_type as ::tryparse::schema::SchemaInfo>::schema()
                    )
                }
            });

            quote! {
                ::tryparse::schema::Schema::Object {
                    name: #name_str.to_string(),
                    fields: vec![#(#field_defs),*],
                }
            }
        }
        Fields::Unnamed(fields) => {
            // Treat tuple structs as tuples
            let field_types = fields.unnamed.iter().map(|f| {
                let ty = &f.ty;
                quote! {
                    <#ty as ::tryparse::schema::SchemaInfo>::schema()
                }
            });

            quote! {
                ::tryparse::schema::Schema::Tuple(vec![#(#field_types),*])
            }
        }
        Fields::Unit => {
            // Unit struct is like null
            quote! {
                ::tryparse::schema::Schema::Null
            }
        }
    }
}

fn generate_enum_schema(name: &syn::Ident, data: &syn::DataEnum) -> proc_macro2::TokenStream {
    let name_str = name.to_string();

    let variant_defs = data.variants.iter().map(|v| {
        let variant_name = v.ident.to_string();

        let variant_schema = match &v.fields {
            Fields::Named(fields) => {
                // Variant with named fields - treat as object
                let field_defs = fields.named.iter().map(|f| {
                    let field_name = f.ident.as_ref().unwrap().to_string();
                    let field_type = &f.ty;

                    quote! {
                        ::tryparse::schema::Field::new(
                            #field_name,
                            <#field_type as ::tryparse::schema::SchemaInfo>::schema()
                        )
                    }
                });

                quote! {
                    ::tryparse::schema::Schema::Object {
                        name: #variant_name.to_string(),
                        fields: vec![#(#field_defs),*],
                    }
                }
            }
            Fields::Unnamed(fields) => {
                // Variant with unnamed fields - treat as tuple
                let field_types = fields.unnamed.iter().map(|f| {
                    let ty = &f.ty;
                    quote! {
                        <#ty as ::tryparse::schema::SchemaInfo>::schema()
                    }
                });

                quote! {
                    ::tryparse::schema::Schema::Tuple(vec![#(#field_types),*])
                }
            }
            Fields::Unit => {
                // Unit variant - like null
                quote! {
                    ::tryparse::schema::Schema::Null
                }
            }
        };

        quote! {
            ::tryparse::schema::Variant::new(#variant_name, #variant_schema)
        }
    });

    quote! {
        ::tryparse::schema::Schema::Union {
            name: #name_str.to_string(),
            variants: vec![#(#variant_defs),*],
        }
    }
}

/// Derives the `LlmDeserialize` trait for a struct.
///
/// This macro generates a custom deserialization implementation using BAML's
/// algorithms for fuzzy field matching and type coercion.
///
/// # Example
///
/// ```ignore
/// use tryparse::deserializer::LlmDeserialize;
///
/// #[derive(LlmDeserialize)]
/// struct User {
///     name: String,
///     age: u32,
///     email: Option<String>, // Optional field
/// }
///
/// // The macro generates an implementation that:
/// // - Handles fuzzy field matching (userName → user_name)
/// // - Supports optional fields with defaults
/// // - Detects circular references
/// // - Tracks transformations
/// ```
#[proc_macro_derive(LlmDeserialize, attributes(llm))]
pub fn derive_llm_deserialize(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    match &input.data {
        Data::Struct(data_struct) => {
            let deserialize_impl = generate_struct_deserialize(name, data_struct);

            let expanded = quote! {
                impl #impl_generics ::tryparse::deserializer::LlmDeserialize for #name #ty_generics #where_clause {
                    #deserialize_impl
                }
            };

            TokenStream::from(expanded)
        }
        Data::Enum(data_enum) => {
            // Check if this is a union enum (has #[llm(union)] attribute)
            let is_union = has_union_attribute(&input.attrs);

            let deserialize_impl = if is_union {
                generate_union_deserialize(name, data_enum, &input.attrs)
            } else {
                generate_enum_deserialize(name, data_enum, &input.attrs)
            };

            let expanded = quote! {
                impl #impl_generics ::tryparse::deserializer::LlmDeserialize for #name #ty_generics #where_clause {
                    #deserialize_impl
                }
            };

            TokenStream::from(expanded)
        }
        Data::Union(_) => {
            syn::Error::new_spanned(input, "LlmDeserialize cannot be derived for unions")
                .to_compile_error()
                .into()
        }
    }
}

fn generate_struct_deserialize(
    name: &syn::Ident,
    data: &syn::DataStruct,
) -> proc_macro2::TokenStream {
    match &data.fields {
        Fields::Named(fields) => {
            let field_names: Vec<_> = fields.named.iter().map(|f| &f.ident).collect();
            let field_types: Vec<_> = fields.named.iter().map(|f| &f.ty).collect();
            let field_name_strs: Vec<_> = fields
                .named
                .iter()
                .map(|f| f.ident.as_ref().unwrap().to_string())
                .collect();

            // Check if each field is Option<T>
            let is_optional: Vec<_> = field_types.iter().map(|ty| is_option_type(ty)).collect();

            // Extract inner type for Option<T> fields
            let inner_types: Vec<_> = field_types
                .iter()
                .zip(&is_optional)
                .map(|(ty, opt)| {
                    if *opt {
                        extract_option_inner(ty)
                    } else {
                        (*ty).clone()
                    }
                })
                .collect();

            let name_str = name.to_string();

            // Generate field descriptor setup (collect to Vec for reuse)
            let field_descriptors: Vec<_> = field_name_strs
                .iter()
                .zip(&field_types)
                .zip(&is_optional)
                .map(|((name, ty), opt)| {
                    let type_name = quote!(stringify!(#ty)).to_string();
                    quote! {
                        .field(::tryparse::deserializer::FieldDescriptor::new(
                            #name,
                            #type_name,
                            #opt
                        ))
                    }
                })
                .collect();

            // Generate field extraction for try_deserialize (returns Option)
            let field_extractions_strict: Vec<_> = field_names
                .iter()
                .zip(&inner_types)
                .zip(&is_optional)
                .map(|((field_name, inner_ty), opt)| {
                    let field_name_str = field_name.as_ref().unwrap().to_string();
                    if *opt {
                        // Optional field
                        quote! {
                            let #field_name = fields.get(#field_name_str)
                                .and_then(|v| v.downcast_ref::<#inner_ty>())
                                .cloned();
                        }
                    } else {
                        // Required field - return None if missing
                        quote! {
                            let #field_name = fields.get(#field_name_str)
                                .and_then(|v| v.downcast_ref::<#inner_ty>())
                                .cloned()?;
                        }
                    }
                })
                .collect();

            // Generate field extraction for deserialize (returns Result)
            let field_extractions_lenient: Vec<_> = field_names.iter().zip(&inner_types).zip(&is_optional).map(|((field_name, inner_ty), opt)| {
                let field_name_str = field_name.as_ref().unwrap().to_string();
                if *opt {
                    // Optional field
                    quote! {
                        let #field_name = fields.get(#field_name_str)
                            .and_then(|v| v.downcast_ref::<#inner_ty>())
                            .cloned();
                    }
                } else {
                    // Required field
                    quote! {
                        let #field_name = fields.get(#field_name_str)
                            .and_then(|v| v.downcast_ref::<#inner_ty>())
                            .cloned()
                            .ok_or_else(|| ::tryparse::error::ParseError::DeserializeFailed(
                                ::tryparse::error::DeserializeError::missing_field(#field_name_str)
                            ))?;
                    }
                }
            }).collect();

            quote! {
                fn try_deserialize(
                    value: &::tryparse::value::FlexValue,
                    ctx: &mut ::tryparse::deserializer::CoercionContext,
                ) -> Option<Self> {
                    use std::any::Any;

                    let mut deserializer = ::tryparse::deserializer::StructDeserializer::new()
                        #(#field_descriptors)*;

                    let fields = deserializer.try_deserialize(
                        value,
                        ctx,
                        #name_str,
                        |field_name, field_value, field_ctx| {
                            // Dispatch to the appropriate field type's LlmDeserialize impl (strict mode only)
                            match field_name {
                                #(
                                    #field_name_strs => {
                                        // Try strict deserialization
                                        <#inner_types as ::tryparse::deserializer::LlmDeserialize>::try_deserialize(field_value, field_ctx)
                                            .map(|v| Box::new(v) as Box<dyn Any>)
                                    }
                                )*
                                _ => None
                            }
                        }
                    ).ok()?;

                    // Extract fields from Box<dyn Any> (strict mode - return None on failure)
                    #(#field_extractions_strict)*

                    Some(Self {
                        #(#field_names),*
                    })
                }

                fn deserialize(
                    value: &::tryparse::value::FlexValue,
                    ctx: &mut ::tryparse::deserializer::CoercionContext,
                ) -> ::tryparse::error::Result<Self> {
                    use std::any::Any;

                    let mut deserializer = ::tryparse::deserializer::StructDeserializer::new()
                        #(#field_descriptors)*;

                    let fields = deserializer.deserialize(
                        value,
                        ctx,
                        #name_str,
                        |field_name, field_value, field_ctx, strict| {
                            // Dispatch to the appropriate field type's LlmDeserialize impl
                            match field_name {
                                #(
                                    #field_name_strs => {
                                        if strict {
                                            // Try strict deserialization
                                            if let Some(v) = <#inner_types as ::tryparse::deserializer::LlmDeserialize>::try_deserialize(field_value, field_ctx) {
                                                Ok(Box::new(v) as Box<dyn Any>)
                                            } else {
                                                Err(::tryparse::error::ParseError::DeserializeFailed(
                                                    ::tryparse::error::DeserializeError::type_mismatch(
                                                        stringify!(#inner_types),
                                                        "value"
                                                    )
                                                ))
                                            }
                                        } else {
                                            // Lenient deserialization
                                            let v = <#inner_types as ::tryparse::deserializer::LlmDeserialize>::deserialize(field_value, field_ctx)?;
                                            Ok(Box::new(v) as Box<dyn Any>)
                                        }
                                    }
                                )*
                                _ => Err(::tryparse::error::ParseError::DeserializeFailed(
                                    ::tryparse::error::DeserializeError::Custom(
                                        format!("Unknown field: {}", field_name)
                                    )
                                ))
                            }
                        }
                    )?;

                    // Extract fields from Box<dyn Any> (lenient mode - return error on failure)
                    #(#field_extractions_lenient)*

                    Ok(Self {
                        #(#field_names),*
                    })
                }
            }
        }
        Fields::Unnamed(_) => syn::Error::new_spanned(
            data.fields.clone(),
            "LlmDeserialize does not support tuple structs yet",
        )
        .to_compile_error(),
        Fields::Unit => syn::Error::new_spanned(
            data.fields.clone(),
            "LlmDeserialize does not support unit structs",
        )
        .to_compile_error(),
    }
}

/// Check if a type is Option<T>
fn is_option_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "Option";
        }
    }
    false
}

/// Extract the inner type T from Option<T>
fn extract_option_inner(ty: &Type) -> Type {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "Option" {
                if let PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(GenericArgument::Type(inner)) = args.args.first() {
                        return inner.clone();
                    }
                }
            }
        }
    }
    // Fallback: return the original type
    ty.clone()
}

fn generate_enum_deserialize(
    name: &syn::Ident,
    data: &syn::DataEnum,
    _attrs: &[syn::Attribute],
) -> proc_macro2::TokenStream {
    let name_str = name.to_string();

    // Build EnumMatcher setup with all variants
    let matcher_setup = data.variants.iter().map(|v| {
        let variant_name = v.ident.to_string();
        quote! {
            .variant(::tryparse::deserializer::enum_coercer::EnumVariant::new(#variant_name))
        }
    });

    // Build match arms for each variant
    let match_arms = data.variants.iter().map(|v| {
        let variant_ident = &v.ident;
        let variant_name = v.ident.to_string();

        match &v.fields {
            Fields::Unit => {
                // Simple unit variant (e.g., Status::Active)
                quote! {
                    #variant_name => Ok(Self::#variant_ident),
                }
            }
            Fields::Named(_) | Fields::Unnamed(_) => {
                // Complex variants with fields - not yet supported in derive macro
                // Users can implement LlmDeserialize manually for these cases
                quote! {
                    #variant_name => Err(::tryparse::error::ParseError::DeserializeFailed(
                        ::tryparse::error::DeserializeError::Custom(
                            format!("Enum variant '{}' has fields - derive macro only supports unit variants", #variant_name)
                        )
                    )),
                }
            }
        }
    });

    quote! {
        fn deserialize(
            value: &::tryparse::value::FlexValue,
            _ctx: &mut ::tryparse::deserializer::CoercionContext,
        ) -> ::tryparse::error::Result<Self> {
            // Build matcher with all enum variants
            let matcher = ::tryparse::deserializer::enum_coercer::EnumMatcher::new()
                #(#matcher_setup)*;

            // Use BAML's fuzzy matching to find the best variant
            let matched_variant = ::tryparse::deserializer::enum_coercer::match_enum_variant(
                value,
                &matcher
            )?;

            // Construct the matched variant
            match matched_variant.as_str() {
                #(#match_arms)*
                _ => Err(::tryparse::error::ParseError::DeserializeFailed(
                    ::tryparse::error::DeserializeError::UnknownVariant {
                        enum_name: #name_str.to_string(),
                        variant: matched_variant,
                    }
                )),
            }
        }
    }
}

/// Check if enum has #[llm(union)] attribute.
fn has_union_attribute(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        if attr.path().is_ident("llm") {
            // Parse as #[llm(union)]
            if let Ok(meta_list) = attr.meta.require_list() {
                // Check if any nested item is "union"
                return meta_list.tokens.to_string().trim() == "union";
            }
        }
        false
    })
}

/// Generate union deserialization code for enums with #[llm(union)].
fn generate_union_deserialize(
    name: &syn::Ident,
    data: &syn::DataEnum,
    _attrs: &[syn::Attribute],
) -> proc_macro2::TokenStream {
    if data.variants.len() != 2 {
        return syn::Error::new_spanned(name, "Union enums must have exactly 2 variants")
            .to_compile_error();
    }

    let variants: Vec<_> = data.variants.iter().collect();
    let variant1 = &variants[0];
    let variant2 = &variants[1];

    // Extract variant types
    let (variant1_ident, variant1_type) = match &variant1.fields {
        Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
            (&variant1.ident, &fields.unnamed[0].ty)
        }
        _ => {
            return syn::Error::new_spanned(
                variant1,
                "Union variants must have exactly one unnamed field",
            )
            .to_compile_error();
        }
    };

    let (variant2_ident, variant2_type) = match &variant2.fields {
        Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
            (&variant2.ident, &fields.unnamed[0].ty)
        }
        _ => {
            return syn::Error::new_spanned(
                variant2,
                "Union variants must have exactly one unnamed field",
            )
            .to_compile_error();
        }
    };

    quote! {
        fn deserialize(
            value: &::tryparse::value::FlexValue,
            ctx: &mut ::tryparse::deserializer::CoercionContext,
        ) -> ::tryparse::error::Result<Self> {
            use ::tryparse::deserializer::LlmDeserialize;

            // BAML ALGORITHM: Try strict matching first (try_cast)
            if let Some(v1) = <#variant1_type as LlmDeserialize>::try_deserialize(value, ctx) {
                // Add UnionMatch transformation for strict match
                ctx.add_transformation(::tryparse::value::Transformation::UnionMatch {
                    index: 0,
                    candidates: vec![
                        stringify!(#variant1_type).to_string(),
                        stringify!(#variant2_type).to_string(),
                    ],
                });
                return Ok(Self::#variant1_ident(v1));
            }

            if let Some(v2) = <#variant2_type as LlmDeserialize>::try_deserialize(value, ctx) {
                // Add UnionMatch transformation for strict match
                ctx.add_transformation(::tryparse::value::Transformation::UnionMatch {
                    index: 1,
                    candidates: vec![
                        stringify!(#variant1_type).to_string(),
                        stringify!(#variant2_type).to_string(),
                    ],
                });
                return Ok(Self::#variant2_ident(v2));
            }

            // BAML ALGORITHM: Try lenient matching with scoring (coerce)
            struct MatchResult {
                variant: u8,  // 1 or 2
                score: u32,
            }

            let mut matches = Vec::new();

            // Try variant 1 with separate FlexValue to track transformations
            let value1 = value.clone();
            if let Ok(_) = <#variant1_type as LlmDeserialize>::deserialize(&value1, ctx) {
                let score: u32 = value1.transformations().iter().map(|t| t.penalty()).sum();
                matches.push(MatchResult { variant: 1, score });
            }

            // Try variant 2 with separate FlexValue to track transformations
            let value2 = value.clone();
            if let Ok(_) = <#variant2_type as LlmDeserialize>::deserialize(&value2, ctx) {
                let score: u32 = value2.transformations().iter().map(|t| t.penalty()).sum();
                matches.push(MatchResult { variant: 2, score });
            }

            if matches.is_empty() {
                return Err(::tryparse::error::ParseError::DeserializeFailed(
                    ::tryparse::error::DeserializeError::Custom(
                        "No union variant matched".to_string()
                    )
                ));
            }

            // Sort by score (lower is better)
            matches.sort_by_key(|m| m.score);

            // Add UnionMatch transformation to track which variant was selected
            let variant_index = (matches[0].variant - 1) as usize;
            ctx.add_transformation(::tryparse::value::Transformation::UnionMatch {
                index: variant_index,
                candidates: vec![
                    stringify!(#variant1_type).to_string(),
                    stringify!(#variant2_type).to_string(),
                ],
            });

            // Deserialize the best match
            match matches[0].variant {
                1 => {
                    let v1 = <#variant1_type as LlmDeserialize>::deserialize(value, ctx)?;
                    Ok(Self::#variant1_ident(v1))
                }
                2 => {
                    let v2 = <#variant2_type as LlmDeserialize>::deserialize(value, ctx)?;
                    Ok(Self::#variant2_ident(v2))
                }
                _ => unreachable!(),
            }
        }
    }
}
