extern crate proc_macro;

use proc_macro2::{TokenStream, Span};
use quote::{quote, ToTokens};
use syn::{parse_macro_input, Ident, DeriveInput, Type};

struct MappFunctionHeader {
    ident: Ident,
    arguments: Vec<(Ident, Type)>,
    return_type: Type,
}

impl ToTokens for MappFunctionHeader {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let ident = &self.ident;
        let args: Vec<_> = self.arguments[..].iter()
            .map(|(arg_ident, arg_ty)| quote! {
                #arg_ident: #arg_ty
            }).collect();
        let return_ty = &self.return_type;

        tokens.extend(quote! {
            fn #ident(&mut self#(, #args)*) -> #return_ty;
        })
    }
}

impl MappFunctionHeader {
    fn to_exported_fn_tokens(&self) -> TokenStream {
        let ident = &self.ident;
        let arg_idents: Vec<_> = self.arguments[..].iter()
            .map(|(arg_ident, _)| arg_ident).collect();
        let arg_tys: Vec<_> = self.arguments[..].iter()
            .map(|(_, arg_ty)| arg_ty).collect();
        let return_ty = &self.return_type;

        quote! {
            #[wasm_bindgen]
            pub fn #ident(args: String) -> String {
                let (#(#arg_idents, )*) = ::json5::from_str::<(#(#arg_tys, )*)>(&args).unwrap();
                let mut ctx = __internal_mlib::MAPP_GLOBAL.write().unwrap();
                let ctx = ctx.as_mut().unwrap();
                let result: #return_ty = ctx.#ident(#(#arg_idents, )*);

                ::json5::to_string(&result).unwrap()
            }
        }
    }
}

macro_rules! mapp_function_headers {
    {
        $(
            fn $ident:ident (&mut self$(, $arg_ident:ident: $arg_ty:ty)* $(,)?) -> $return_ty:ty
        );* $(;)?
    } => {{
        [
            $(MappFunctionHeader {
                ident: Ident::new(stringify!($ident), Span::call_site()),
                arguments: vec![
                    $(
                        (
                            Ident::new(stringify!($arg_ident), Span::call_site()),
                            {
                                let ts = quote! { $arg_ty }.into();
                                parse_macro_input!(ts as Type)
                            },
                        )
                    ),+
                ],
                return_type: {
                    let ts = quote! { $return_ty }.into();
                    parse_macro_input!(ts as Type)
                },
            }),*
        ]
    }}
}

#[proc_macro_attribute]
pub fn mapp(_args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // Functions, which require serialization, implemented by the Metaview App.
    let mapp_function_headers = mapp_function_headers! {
        fn test(&mut self, arg: String) -> Vec<String>;
        fn get_model_matrices(&mut self, secs_elapsed: f32) -> Vec<::ammolite_math::Mat4>;
    };

    let input_cloned = input.clone();
    let parsed_input = parse_macro_input!(input_cloned as DeriveInput);
    let implementor_type = parsed_input.ident;
    let mapp_exported_functions: Vec<_> = mapp_function_headers.iter()
        .map(|f| f.to_exported_fn_tokens())
        .collect();
    let expanded = quote! {
        mod __internal_mlib {
            use ::std::marker::PhantomData;
            use ::std::sync::RwLock;
            use ::lazy_static::lazy_static;
            use super::#implementor_type;

            // TYPES
            pub trait Mapp {
                fn new() -> Self;
                #(#mapp_function_headers)*
            }

            // DISALLOW COMPILATION UNLESS TRAIT IMPLEMENTED
            #[allow(missing_copy_implementations)]
            #[allow(non_camel_case_types)]
            #[allow(dead_code)]
            struct TraitGuard<T: Mapp>(PhantomData<T>);
            const PLEASE_ENSURE_MAPP_IS_IMPLEMENTED: TraitGuard<#implementor_type> = TraitGuard(PhantomData);

            // GLOBAL STATE
            lazy_static! {
                pub static ref MAPP_GLOBAL: RwLock<Option<#implementor_type>> = RwLock::new(None);
            }
        }

        pub use __internal_mlib::Mapp;

        // EXPORTED FUNCTIONS
        #[wasm_bindgen]
        pub fn initialize() {
            *(__internal_mlib::MAPP_GLOBAL).write().unwrap() = Some(<#implementor_type as Mapp>::new());
        }

        #[wasm_bindgen]
        pub fn api_version() -> String {
            env!("CARGO_PKG_VERSION").to_string()
        }

        #(#mapp_exported_functions)*
    };

    // Hand the output tokens back to the compiler.
    let mut result = proc_macro::TokenStream::new();
    result.extend(proc_macro::TokenStream::from(expanded));
    result.extend(input);
    result
}
