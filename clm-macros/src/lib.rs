use quote::{format_ident, quote};
use syn::{Attribute, DeriveInput, Expr, Ident, ImplItem, ItemImpl, LitStr, parse_macro_input};

#[proc_macro_derive(ConvertValue)]
pub fn derive_convert_value(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let derive = parse_macro_input!(item as DeriveInput);
    let struct_name = derive.ident;
    quote! {
        impl ::std::convert::From<#struct_name> for ::clm_plugin_api::core::Value {
            fn from(value: #struct_name) -> Self {
                ::clm_plugin_api::core::to_value(&value).unwrap()
            }
        }
        impl ::std::convert::TryFrom<::clm_plugin_api::core::Value> for #struct_name {
            type Error = ::clm_plugin_api::core::ValueConvertError;
            fn try_from(value: ::clm_plugin_api::core::Value) -> Result<Self, Self::Error> {
                ::clm_plugin_api::core::from_value(value)
            }
        }
    }
    .into()
}
#[proc_macro_derive(ConvertValueInApi)]
pub fn derive_convert_value_in_api(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let derive = parse_macro_input!(item as DeriveInput);
    let struct_name = derive.ident;
    quote! {
        impl ::std::convert::From<&#struct_name> for ::clm_core::value::Value {
            fn from(value: &#struct_name) -> Self {
                ::clm_core::value::to_value(value).unwrap()
            }
        }
        impl ::std::convert::From<#struct_name> for ::clm_core::value::Value {
            fn from(value: #struct_name) -> Self {
                ::clm_core::value::to_value(&value).unwrap()
            }
        }
        impl ::std::convert::TryFrom<::clm_core::value::Value> for #struct_name {
            type Error = ::clm_core::value::ValueConvertError;
            fn try_from(value: ::clm_core::value::Value) -> Result<Self, Self::Error> {
                ::clm_core::value::from_value(value)
            }
        }
    }
    .into()
}

struct SubscribeInfo {
    kind: String,
    const_name: Ident,
    properties: Vec<(String, Expr)>,
}
struct SubscribeAttrData {
    kind: Option<String>,
    properties: Vec<(String, Expr)>,
}
#[derive(Debug)]
struct ServiceInfo {
    name: String,
    const_name: Ident,
    mutability: bool,
}
#[derive(Debug)]
struct ServiceAttrData {
    name: Option<String>,
}

#[proc_macro_attribute]
pub fn clm_handlers(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let mut plugin_name: Option<String> = None;
    let attr_parser = syn::meta::parser(|meta| {
        if meta.path.is_ident("name") {
            let value: LitStr = meta.value()?.parse()?;
            plugin_name = Some(value.value());
            Ok(())
        } else {
            Err(meta.error("unsupported property"))
        }
    });
    parse_macro_input!(attr with attr_parser);
    let plugin_name = plugin_name.unwrap();

    let mut impl_block = parse_macro_input!(item as ItemImpl);
    let type_name = &impl_block.self_ty;

    let mut handler_consts = vec![];
    let mut subscribes = vec![];
    let mut services = vec![];

    for item in &mut impl_block.items {
        let ImplItem::Fn(method) = item else {
            continue;
        };

        let mut subscribe_attr = None;
        let mut service_attr = None;
        let mut manually_register_attr = false;
        method.attrs.retain(|attr| {
            if attr.path().is_ident("subscribe") {
                subscribe_attr = Some(parse_subscribe_attr(attr));
                false
            } else if attr.path().is_ident("service") {
                service_attr = Some(parse_service_attr(attr));
                false
            } else if attr.path().is_ident("manually_register") {
                manually_register_attr = true;
                false
            } else {
                true
            }
        });
        if subscribe_attr.is_none() && service_attr.is_none() {
            continue;
        }
        if subscribe_attr.is_some() && service_attr.is_some() {
            panic!("conflict attributes");
        }

        let sig = &method.sig;
        let Some(receiver) = sig.receiver() else {
            panic!("invalid receiver");
        };

        let method_name = &sig.ident;
        let const_name = format_ident!("{}", method_name.to_string().to_uppercase());

        // EventHandler
        if let Some(subscribe_attr) = subscribe_attr {
            let arg_types: Vec<_> = sig
                .inputs
                .iter()
                .filter_map(|arg| {
                    let syn::FnArg::Typed(pat_type) = arg else {
                        return None;
                    };
                    Some(pat_type.ty.clone())
                })
                .collect();
            if 1 < arg_types.len() {
                panic!("subscribe args len should 0 or 1");
            }
            if let Some(arg_type) = arg_types.first() {
                handler_consts.push(quote! {
                    const #const_name: ::clm_plugin_api::core::RawEventHandler = {
                        unsafe fn __raw_event_handler(
                            ptr: *mut (),
                            data: &::clm_plugin_api::core::Value,
                        ) -> ::clm_plugin_api::core::EventResult {
                            let Ok(arg) = #arg_type::try_from(data.clone()) else {
                                return EventResult::Propagate;
                            };
                            (&mut *(ptr as *mut #type_name)).#method_name(arg)
                        }
                        __raw_event_handler
                    };
                });
            } else {
                handler_consts.push(quote! {
                    const #const_name: ::clm_plugin_api::core::RawEventHandler = {
                        unsafe fn __raw_event_handler(
                            ptr: *mut (),
                            _data: &::clm_plugin_api::core::Value,
                        ) -> ::clm_plugin_api::core::EventResult {
                            (&mut *(ptr as *mut #type_name)).#method_name()
                        }
                        __raw_event_handler
                    };
                });
            }
            if !manually_register_attr {
                subscribes.push(SubscribeInfo {
                    kind: subscribe_attr.kind.unwrap_or({
                        let method_name = method_name.to_string();
                        if let Some(kind) = method_name.strip_prefix("on_") {
                            kind.to_string()
                        } else {
                            method_name
                        }
                    }),
                    const_name: const_name.clone(),
                    properties: subscribe_attr.properties,
                });
            }
        }
        if let Some(service_attr) = service_attr {
            let (arg_idents, (arg_types, arg_indices)): (Vec<_>, (Vec<_>, Vec<_>)) = sig
                .inputs
                .iter()
                .filter_map(|arg| {
                    let syn::FnArg::Typed(pat_type) = arg else {
                        return None;
                    };
                    Some(pat_type.ty.clone())
                })
                .enumerate()
                .map(|(i, ty)| (format_ident!("arg{i}"), (ty, i)))
                .unzip();
            // MutServiceHandler
            if receiver.mutability.is_some() {
                handler_consts.push(quote! {
                    const #const_name: ::clm_plugin_api::core::RawMutServiceHandler = {
                        unsafe fn __raw_mut_service_handler(
                            ptr: *mut (),
                            args: &[::clm_plugin_api::core::Value],
                        ) -> ::std::result::Result<::clm_plugin_api::core::Value, ::std::string::String> {
                            #(let #arg_idents: #arg_types = ::clm_plugin_api::core::get_arg(args, #arg_indices)?;)*
                            Ok((&mut *(ptr as *mut #type_name)).#method_name(#(#arg_idents),*)?.into())
                        }
                        __raw_mut_service_handler
                    };
                });
            }
            // ServiceHandler
            else {
                handler_consts.push(quote! {
                    const #const_name: ::clm_plugin_api::core::RawServiceHandler = {
                        unsafe fn __raw_service_handler(
                            ptr: *const (),
                            args: &[::clm_plugin_api::core::Value],
                        ) -> ::std::result::Result<::clm_plugin_api::core::Value, ::std::string::String> {
                            #(let #arg_idents: #arg_types = ::clm_plugin_api::core::get_arg(args, #arg_indices)?;)*
                            Ok((&*(ptr as *const #type_name)).#method_name(#(#arg_idents),*)?.into())
                        }
                        __raw_service_handler
                    };
                });
            }
            if !manually_register_attr {
                services.push(ServiceInfo {
                    name: service_attr
                        .name
                        .unwrap_or(plugin_name.clone() + "." + &method_name.to_string()),
                    const_name,
                    mutability: receiver.mutability.is_some(),
                });
            }
        }
    }

    for constant in handler_consts {
        impl_block.items.push(syn::parse2(constant).unwrap());
    }

    let subscribe_stmts: Vec<_> = subscribes
        .iter()
        .map(|s| {
            let kind = &s.kind;
            let const_name = &s.const_name;
            let properties: Vec<_> = s
                .properties
                .iter()
                .map(|(key, expr)| {
                    quote! {
                        (
                            ::clm_plugin_api::core::PropertyKey(#key.to_string()),
                            (#expr).into(),
                        )
                    }
                })
                .collect();
            quote! {
                reg.subscribe(
                    #kind,
                    ::std::collections::HashMap::from([
                        #(#properties),*
                    ]),
                    Self::#const_name,
                );
            }
        })
        .collect();
    let service_stmts: Vec<_> = services
        .iter()
        .map(|s| {
            let name = &s.name;
            let const_name = &s.const_name;
            if s.mutability {
                quote! {
                    reg.register_mut_service(#name, Self::#const_name);
                }
            } else {
                quote! {
                    reg.register_service(#name, Self::#const_name);
                }
            }
        })
        .collect();
    impl_block.items.push(
        syn::parse2(quote! {
            fn register_service_and_subscribe(reg: ::clm_plugin_api::core::PluginRegistrar) {
                #(#subscribe_stmts)*
                #(#service_stmts)*
            }
        })
        .unwrap(),
    );

    quote! { #impl_block }.into()
}

fn parse_subscribe_attr(attr: &Attribute) -> SubscribeAttrData {
    let mut kind = None;
    let mut properties = vec![];

    let _ = attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("kind") {
            let value: LitStr = meta.value()?.parse()?;
            kind = Some(value.value());
        } else if let Some(ident) = meta.path.get_ident() {
            let value: Expr = meta.value()?.parse()?;
            properties.push((ident.to_string(), value));
        }
        Ok(())
    });

    SubscribeAttrData { kind, properties }
}
fn parse_service_attr(attr: &Attribute) -> ServiceAttrData {
    let mut name = None;
    let _ = attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("name") {
            let value: LitStr = meta.value()?.parse()?;
            name = Some(value.value());
        }
        Ok(())
    });

    ServiceAttrData { name }
}
