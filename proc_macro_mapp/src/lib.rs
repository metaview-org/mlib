extern crate proc_macro;

use proc_macro2::{TokenStream, Span};
use quote::{quote, format_ident};
use syn::{parse_macro_input, Ident, DeriveInput, Type};

struct MappFunctionHeader {
    ident: Ident,
    arguments: Vec<(Ident, Type)>,
    return_type: Type,
}

impl MappFunctionHeader {
    fn to_client_header_tokens(&self) -> TokenStream {
        let ident = &self.ident;
        let args: Vec<_> = self.arguments[..].iter()
            .map(|(arg_ident, arg_ty)| quote! {
                #arg_ident: #arg_ty
            }).collect();
        let return_ty = &self.return_type;

        quote! {
            fn #ident(&mut self#(, #args)*) -> #return_ty;
        }
    }

    fn to_client_exported_fn_tokens(&self, implementor_type: &Ident) -> TokenStream {
        let ident = &self.ident;
        let arg_idents: Vec<_> = self.arguments[..].iter()
            .map(|(arg_ident, _)| arg_ident).collect();
        let arg_tys: Vec<_> = self.arguments[..].iter()
            .map(|(_, arg_ty)| arg_ty).collect();
        let return_ty = &self.return_type;

        quote! {
            #[wasm_bindgen]
            pub fn #ident(args: String) -> String {
                let (#(#arg_idents, )*) = ::json5::from_str::<(#(#arg_tys, )*)>(&args)
                    .unwrap_or_else(|e| panic!("Could not deserialize host-provided arguments of the method '{}': {:?}", stringify!(#ident), e));
                let mut ctx = __internal_mlib::MAPP_GLOBAL.write()
                    .unwrap_or_else(|e| panic!("Global state of the Mapp became poisoned: {}", e));
                let ctx = ctx.as_mut()
                    .unwrap_or_else(|| panic!("Method '{}' called without initialization of the Mapp.", stringify!(#ident)));
                let result: #return_ty = <#implementor_type as Mapp>::#ident(ctx, #(#arg_idents, )*);

                ::json5::to_string(&result)
                    .unwrap_or_else(|e| panic!("Could not serialize the result of calling the method '{}': {:?}", stringify!(#ident), e))
            }
        }
    }

    fn to_host_header_tokens(&self) -> TokenStream {
        let ident = &self.ident;

        quote! {
            fn #ident(&mut self, serialized_args: String) -> String;
        }
    }

    fn to_host_imported_fn_tokens(&self) -> TokenStream {
        let ident = &self.ident;
        let args: Vec<_> = self.arguments[..].iter()
            .map(|(arg_ident, arg_ty)| quote! {
                #arg_ident: #arg_ty
            }).collect();
        let arg_idents: Vec<_> = self.arguments[..].iter()
            .map(|(arg_ident, _)| arg_ident).collect();
        let return_ty = &self.return_type;

        quote! {
            fn #ident(&mut self, #(#args, )*) -> #return_ty {
                let serialized_args = ::json5::to_string(&(#(#arg_idents, )*))
                    .unwrap_or_else(|e| panic!("Could not serialize client-provided arguments of the method '{}': {:?}", stringify!(#ident), e));
                let serialized_result = self.exports.#ident(serialized_args);

                ::json5::from_str(&serialized_result)
                    .unwrap_or_else(|e| panic!("Could not deserialize the result of calling the method '{}': {:?}", stringify!(#ident), e))
            }
        }
    }

    fn to_fn_delegate_from_native_tokens(&self, implementor_type: &Ident) -> TokenStream {
        let ident = &self.ident;
        let args: Vec<_> = self.arguments[..].iter()
            .map(|(arg_ident, arg_ty)| quote! {
                #arg_ident: #arg_ty
            }).collect();
        let arg_idents: Vec<_> = self.arguments[..].iter()
            .map(|(arg_ident, _)| arg_ident).collect();
        let return_ty = &self.return_type;

        quote! {
            fn #ident(&mut self, #(#args, )*) -> #return_ty {
                <#implementor_type as Mapp>::#ident(self, #(#arg_idents,)*)
            }
        }
    }

    fn to_host_imported_fn_header_tokens(&self) -> TokenStream {
        let ident = &self.ident;
        let args: Vec<_> = self.arguments[..].iter()
            .map(|(arg_ident, arg_ty)| quote! {
                #arg_ident: #arg_ty
            }).collect();
        let return_ty = &self.return_type;

        quote! {
            fn #ident(&mut self, #(#args, )*) -> #return_ty;
        }
    }
}

macro_rules! mapp_function_header {
    {
        fn $ident:ident (&mut self$(, $arg_ident:ident: $arg_ty:ty)* $(,)?) -> $return_ty:ty;
    } => {{
        MappFunctionHeader {
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
                ),*
            ],
            return_type: {
                let ts = quote! { $return_ty }.into();
                parse_macro_input!(ts as Type)
            },
        }
    }};

    {
        fn $ident:ident (&mut self$(, $arg_ident:ident: $arg_ty:ty)* $(,)?);
    } => {{
        mapp_function_header! {
            fn $ident (&mut self$(, $arg_ident: $arg_ty)*) -> ();
        }
    }};
}

macro_rules! mapp_function_headers {
    {
        $(
            fn $ident:ident (&mut self$(, $arg_ident:ident: $arg_ty:ty)* $(,)?)$( -> $return_ty:ty)?
        );* $(;)?
    } => {{
        [
            $(
                mapp_function_header! {
                    fn $ident (&mut self$(, $arg_ident: $arg_ty)*)$( -> $return_ty)?;
                }
            ),*
        ]
    }};
}

/// Generates function exports for a WASM module
fn generate_client_interface(
    input: proc_macro::TokenStream,
    mapp_function_headers: &[MappFunctionHeader]
) -> proc_macro::TokenStream {
    let input_cloned = input.clone();
    let parsed_input = parse_macro_input!(input_cloned as DeriveInput);
    let implementor_type = parsed_input.ident;
    let mapp_function_headers_ts: Vec<_> = mapp_function_headers.iter()
        .map(|f| f.to_client_header_tokens())
        .collect();
    let mapp_exported_functions: Vec<_> = mapp_function_headers.iter()
        .map(|f| f.to_client_exported_fn_tokens(&implementor_type))
        .collect();
    let mapp_delegate_functions: Vec<_> = mapp_function_headers.iter()
        .map(|f| f.to_fn_delegate_from_native_tokens(&implementor_type))
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
                #(#mapp_function_headers_ts)*
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

            impl mlib::MappInterface for #implementor_type {
                #(#mapp_delegate_functions)*
            }
        }

        pub use __internal_mlib::Mapp;

        // EXPORTED FUNCTIONS
        #[wasm_bindgen]
        pub fn initialize() {
            *(__internal_mlib::MAPP_GLOBAL)
                .write()
                .unwrap_or_else(|e| panic!("Global state of the Mapp became poisoned: {}", e))
                = Some(<#implementor_type as Mapp>::new());
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

/// Generates function imports to communicate with a WASM Mapp module
fn generate_host_interface(
    input: proc_macro::TokenStream,
    mapp_function_headers: &[MappFunctionHeader]
) -> proc_macro::TokenStream {
    let input_cloned = input.clone();
    let parsed_input = parse_macro_input!(input_cloned as DeriveInput);
    let implementor_type = parsed_input.ident;
    let mapp_function_headers_ts: Vec<_> = mapp_function_headers.iter()
        .map(|f| f.to_host_header_tokens())
        .collect();
    let mapp_imported_function_headers: Vec<_> = mapp_function_headers.iter()
        .map(|f| f.to_host_imported_fn_header_tokens())
        .collect();
    let mapp_imported_functions: Vec<_> = mapp_function_headers.iter()
        .map(|f| f.to_host_imported_fn_tokens())
        .collect();
    let mapp_exports_ident = format_ident!("{}Exports", &implementor_type);
    let expanded = quote! {
        /// A wasmtime_rust-generated struct with bindings to a WASM container
        #[wasmtime_rust::wasmtime]
        pub trait #mapp_exports_ident {
            fn initialize(&mut self);
            fn api_version(&mut self) -> String;
            #(#mapp_function_headers_ts)*
        }

        pub struct #implementor_type {
            exports: #mapp_exports_ident,
        }

        impl #implementor_type {
            /// Loads and initializes the Mapp
            pub fn initialize(exports: #mapp_exports_ident) -> Self {
                let mut mapp = #implementor_type { exports };
                mapp.exports.initialize();
                mapp
            }
        }

        impl mlib::MappInterface for #implementor_type {
            #(#mapp_imported_functions)*
        }
    };

    // Hand the output tokens back to the compiler.
    let mut result = proc_macro::TokenStream::new();
    result.extend(proc_macro::TokenStream::from(expanded));
    result

}

/// Generates function imports to communicate with a WASM Mapp module
fn generate_typed_interface(
    input: proc_macro::TokenStream,
    mapp_function_headers: &[MappFunctionHeader]
) -> proc_macro::TokenStream {
    let input_cloned = input.clone();
    let parsed_input = parse_macro_input!(input_cloned as DeriveInput);
    let implementor_type = parsed_input.ident;
    let mapp_imported_function_headers: Vec<_> = mapp_function_headers.iter()
        .map(|f| f.to_host_imported_fn_header_tokens())
        .collect();
    let expanded = quote! {
        /// A trait implemented by Mapps
        pub trait #implementor_type {
            fn api_version(&mut self) -> String {
                env!("CARGO_PKG_VERSION").to_string()
            }

            #(#mapp_imported_function_headers)*
        }
    };

    // Hand the output tokens back to the compiler.
    let mut result = proc_macro::TokenStream::new();
    result.extend(proc_macro::TokenStream::from(expanded));
    result

}

#[proc_macro_attribute]
pub fn mapp(args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // Functions, which require serialization, implemented by the Metaview App.
    // Ensure absolute paths to types are used.
    let mapp_function_headers = mapp_function_headers! {
        // fn test(&mut self, arg: String) -> Vec<String>;
        fn update(&mut self, elapsed: std::time::Duration);
        fn send_command(&mut self) -> Option<mlib::Command>;
        fn receive_command_response(&mut self, response: mlib::CommandResponse);
        fn flush_io(&mut self) -> mlib::IO;
        fn receive_event(&mut self, event: mlib::Event);
    };

    match args.to_string().as_str() {
        "host" => generate_host_interface(input, &mapp_function_headers[..]),
        "interface" => generate_typed_interface(input, &mapp_function_headers[..]),
        _ => generate_client_interface(input, &mapp_function_headers[..]),
    }
}
