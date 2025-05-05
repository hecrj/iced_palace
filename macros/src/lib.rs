use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemFn, LitStr, parse_macro_input};

#[proc_macro_attribute]
pub fn time(attr: TokenStream, item: TokenStream) -> TokenStream {
    let ItemFn {
        attrs,
        vis,
        sig,
        block,
    } = parse_macro_input!(item as ItemFn);

    let name: Option<LitStr> = parse_macro_input!(attr as Option<LitStr>);

    let name = name
        .as_ref()
        .map(LitStr::value)
        .unwrap_or_else(|| sig.ident.to_string());

    let expanded = quote! {
        #(#attrs)*
        #vis #sig {
            ::iced::debug::time_with(#name, || #block)
        }
    };

    TokenStream::from(expanded)
}
