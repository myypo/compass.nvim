use std::fs;

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{quote, ToTokens};
use syn::{parse_macro_input, DeriveInput, Ident};

#[proc_macro_derive(FromLua)]
pub fn derive_deserializing_from_lua(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, .. } = parse_macro_input!(input as DeriveInput);

    quote! {
        use nvim_oxi::conversion::FromObject;

        impl nvim_oxi::conversion::FromObject for #ident {
            fn from_object(obj: nvim_oxi::Object) -> core::result::Result<Self, nvim_oxi::conversion::Error> {
                Self::deserialize(nvim_oxi::serde::Deserializer::new(obj)).map_err(Into::into)
            }
        }

        impl nvim_oxi::lua::Poppable for #ident {
            unsafe fn pop(lstate: *mut nvim_oxi::lua::ffi::lua_State) -> core::result::Result<Self, nvim_oxi::lua::Error> {
                let obj = nvim_oxi::Object::pop(lstate)?;
                Self::from_object(obj).map_err(|e| nvim_oxi::lua::Error::RuntimeError(e.to_string()))
            }
        }
    }
    .into()
}

#[proc_macro_derive(ToLua)]
pub fn derive_serializing_to_lua(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, .. } = parse_macro_input!(input as DeriveInput);

    quote! {
        use nvim_oxi::conversion::ToObject;

        impl nvim_oxi::conversion::ToObject for #ident {
            fn to_object(self) -> core::result::Result<nvim_oxi::Object, nvim_oxi::conversion::Error> {
                self.serialize(nvim_oxi::serde::Serializer::new()).map_err(Into::into)
            }
        }

        impl nvim_oxi::lua::Pushable for #ident {
            unsafe fn push(self, lstate: *mut nvim_oxi::lua::ffi::lua_State) -> core::result::Result<std::ffi::c_int, nvim_oxi::lua::Error> {
                self.to_object()
                    .map_err(nvim_oxi::lua::Error::push_error_from_err::<Self, _>)?
                    .push(lstate)
            }
        }
    }
    .into()
}

#[proc_macro]
pub fn functions_and_commands(input: TokenStream) -> TokenStream {
    let path = input.to_string();

    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("Failed to get Cargo manifest directory");
    let mut macro_call_path =
        fs::canonicalize(manifest_dir).expect("Failed to canonicalize Cargo manifest directory");
    macro_call_path.push(path.trim_matches('"'));

    let all_modules = fs::read_dir(macro_call_path)
        .expect("Failed to read directory")
        .filter_map(|entry| {
            if let Ok(entry) = entry {
                if let Some(entry_name) = entry.file_name().to_str() {
                    if entry_name != "mod.rs" && entry_name != "setup.rs" {
                        return Some(entry_name.to_owned());
                    }
                }
            }
            None
        });

    let all_modules: Vec<String> = all_modules
        .map(|mut m| {
            m.get_mut(0..1)
                .expect("Module must have a name")
                .make_ascii_uppercase();

            m
        })
        .collect();

    let external: Vec<Ident> = all_modules
        .clone()
        .into_iter()
        .map(|m| Ident::new(&m, Span::call_site()))
        .collect();

    quote! {
        #[derive(strum_macros::VariantNames, strum_macros::EnumString, strum_macros::Display, strum_macros::EnumIter)]
        #[strum(serialize_all = "lowercase")]
        pub enum CommandNames {
            #(#external),*
        }
    }
    .into()
}

#[proc_macro]
pub fn to_lowercase(input: TokenStream) -> TokenStream {
    Ident::new(&input.to_string().to_lowercase(), Span::call_site())
        .into_token_stream()
        .into()
}
