use proc_macro::TokenStream;
use quote::quote;
use syn;

#[proc_macro_derive(Event)]
pub fn event_derive(input: TokenStream) -> TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    let ast = syn::parse(input).unwrap();

    // Build the trait implementation
    impl_event_macro(&ast)
}

fn impl_event_macro(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let gen = quote! {
        impl crate::events::types::Event for #name {
            fn kind() -> String {
                String::from(#name)
            }

            fn from_event(value: &serde_json::Value) -> serde_json::Result<Self>
            where
                Self: Sized,
            {
                serde_json::from_value(value.clone())
            }
        }
    };
    gen.into()
}
