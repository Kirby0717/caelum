use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{ImplItem, ItemImpl, parse_macro_input};

#[proc_macro_attribute]
pub fn clm_handlers(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut impl_block = parse_macro_input!(item as ItemImpl);
    let type_name = &impl_block.self_ty;
    let mut handler_consts = Vec::new();

    for item in &impl_block.items {
        let ImplItem::Fn(method) = item
        else {
            continue;
        };
        let sig = &method.sig;
        // &mut selfの検査
        let Some(receiver) = sig.receiver()
        else {
            continue;
        };
        if !(receiver.reference.is_some() && receiver.mutability.is_some()) {
            continue;
        }

        let method_name = &sig.ident;
        let const_name =
            format_ident!("{}", method_name.to_string().to_uppercase());

        handler_consts.push(quote! {
            pub const #const_name: ::clm_plugin_api::core::RawHandler = {
                unsafe fn __raw_handler(
                    ptr: *mut (),
                    data: &::clm_plugin_api::core::EventData,
                    ctx: &mut dyn ::clm_plugin_api::core::PluginContext
                ) -> ::clm_plugin_api::core::EventResult {
                    (&mut *(ptr as *mut #type_name)).#method_name(data, ctx)
                }
                __raw_handler
            };
        });
    }

    for constant in handler_consts {
        impl_block.items.push(syn::parse2(constant).unwrap());
    }

    TokenStream::from(quote! { #impl_block })
}
