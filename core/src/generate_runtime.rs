use crate::{
    TokenStream2,
    TypeGenerator,
};
use frame_metadata::{
    RuntimeMetadata,
    v13::{
        RuntimeMetadataV13,
        RuntimeMetadataPrefixed,
    },
};
use quote::{
    format_ident,
    quote,
    IdentFragment,
};
use scale_info::{
    form::FormString,
    prelude::string::ToString,
};

pub struct RuntimeGenerator<S: FormString> {
    metadata: RuntimeMetadataV13<S>,
}

impl<S> RuntimeGenerator<S>
where
    S: FormString + From<&'static str> + ToString + IdentFragment,
{
    pub fn new(metadata: RuntimeMetadataPrefixed<S>) -> Self {
        match metadata.1 {
            RuntimeMetadata::V13(v13) => {
                Self {
                    metadata: v13,
                }
            },
            _ => panic!("Unsupported metadata version {:?}", metadata.1)
        }
    }

    pub fn generate_runtime(&self, mod_name: &str) -> TokenStream2 {
        let types_mod = "types";
        let type_generator = TypeGenerator::new(&self.metadata.types);
        let types = type_generator.generate(types_mod);
        let modules = self.metadata.modules.iter().map(|module| {
            use heck::SnakeCase as _;
            let mod_name = format_ident!("{}", module.name.to_string().to_snake_case());
            let calls = module
                .calls
                .as_ref()
                .unwrap_or(&Vec::new())
                .iter()
                .map(|call| {
                    use heck::CamelCase as _;
                    // todo: add free functions to Call mod and doc strings
                    let name = format_ident!("{}", call.name.to_string().to_camel_case());
                    let args = call.arguments.iter().map(|arg| {
                        let name = format_ident!("{}", arg.name);
                        let ty = type_generator.resolve_type(arg.ty.id(), &[]);
                        // todo: add docs and #[compact] attr
                        quote! { #name: #ty }
                    });
                    quote! {
                        pub struct #name {
                            #( #args ),*
                        }
                    }
                })
                .collect::<Vec<_>>();
            let events = module
                .event
                .as_ref()
                .unwrap_or(&Vec::new())
                .iter()
                .map(|event| {
                    let name = format_ident!("{}", event.name);
                    let args = event.arguments.iter().map(|arg| {
                        type_generator.resolve_type(arg.ty.id(), &[])
                        // todo: add docs and #[compact] attr
                    });
                    quote! {
                        pub struct #name (
                            #( #args ),*
                        );
                    }
                })
                .collect::<Vec<_>>();
            let calls = if !calls.is_empty() {
                quote! {
                    mod calls {
                        // todo: use types mod name defined earlier
                        use super::*;
                        #( #calls )*
                    }
                }
            } else {
                quote! {}
            };
            let events = if !events.is_empty() {
                quote! {
                    pub mod events {
                        use super::*;
                        #( #events )*
                    }
                }
            } else {
                quote! {}
            };

            quote! {
                pub mod #mod_name {
                    use super::types::*;
                    #calls
                    #events
                }
            }
        });

        let mod_name = format_ident!("{}", mod_name);
        quote! {
            #[allow(dead_code, unused_imports)]
            pub mod #mod_name {
                #types

                #( #modules )*
            }
        }
        // todo: generate outer event? needs custom decode for potential changing indices
    }
}
