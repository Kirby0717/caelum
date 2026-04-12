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
        // &selfの取得
        let Some(receiver) = sig.receiver()
        else {
            continue;
        };
        if receiver.reference.is_none() {
            continue;
        }

        let method_name = &sig.ident;
        let const_name =
            format_ident!("{}", method_name.to_string().to_uppercase());

        // EventHandler
        if receiver.mutability.is_some() && sig.inputs.len() == 3 {
            handler_consts.push(quote! {
                const #const_name: ::clm_plugin_api::core::RawEventHandler = {
                    unsafe fn __raw_event_handler(
                        ptr: *mut (),
                        data: &::clm_plugin_api::core::EventData,
                        ctx: &mut dyn ::clm_plugin_api::core::PluginContext
                    ) -> ::clm_plugin_api::core::EventResult {
                        (&mut *(ptr as *mut #type_name)).#method_name(data, ctx)
                    }
                    __raw_event_handler
                };
            });
        }
        // ServiceHandler
        else if receiver.mutability.is_none() && sig.inputs.len() == 2 {
            handler_consts.push(quote! {
                const #const_name: ::clm_plugin_api::core::RawServiceHandler = {
                    unsafe fn __raw_service_handler(
                        ptr: *const (),
                        args: &[::clm_plugin_api::core::Value],
                    ) -> ::clm_plugin_api::core::Value {
                        (&*(ptr as *const #type_name)).#method_name(args)
                    }
                    __raw_service_handler
                };
            });
        }
    }

    for constant in handler_consts {
        impl_block.items.push(syn::parse2(constant).unwrap());
    }

    TokenStream::from(quote! { #impl_block })
}
