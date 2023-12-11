use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(Event)]
pub fn event_macro_derive(input: TokenStream) -> TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    let ast = parse_macro_input!(input as DeriveInput);

    // Build the trait implementation
    impl_event_macro(&ast)
}

fn impl_event_macro(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let kind = name.to_string().to_case(Case::Snake);

    let impl_event = quote! {
        impl chang::events::Event for #name {
            fn kind() -> String {
                #kind.to_string()
            }

            fn from_event(value: &serde_json::Value) -> serde_json::Result<Self>
            where
                Self: Sized,
            {
                serde_json::from_value(value.clone())
            }
        }
    };

    TokenStream::from(impl_event)
}
