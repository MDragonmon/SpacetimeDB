use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::Parser;
use syn::{FnArg, ItemFn};

use crate::util::ident_to_litstr;

pub(crate) struct ViewArgs;

impl ViewArgs {
    /// Parse `#[view(public)]` where public is required
    pub(crate) fn parse(input: TokenStream) -> syn::Result<Self> {
        if input.is_empty() {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "views must be declared as #[view(public)]",
            ));
        }
        let mut public = false;
        syn::meta::parser(|meta| {
            if meta.path.is_ident("public") {
                if public {
                    return Err(syn::Error::new_spanned(meta.path, "duplicate `public`"));
                }
                public = true;
                Ok(())
            } else {
                Err(syn::Error::new_spanned(
                    meta.path,
                    "unknown #[view(...)] argument; expected `public`",
                ))
            }
        })
        .parse2(input)?;
        if !public {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "views must be declared as #[view(public)]",
            ));
        }
        Ok(Self)
    }
}

pub(crate) fn view_impl(_args: ViewArgs, original_function: &ItemFn) -> syn::Result<TokenStream> {
    let func_name = &original_function.sig.ident;
    let view_name = ident_to_litstr(func_name);
    let vis = &original_function.vis;

    for param in &original_function.sig.generics.params {
        let err = |msg| syn::Error::new_spanned(param, msg);
        match param {
            syn::GenericParam::Lifetime(_) => {}
            syn::GenericParam::Type(_) => return Err(err("type parameters are not allowed on views")),
            syn::GenericParam::Const(_) => return Err(err("const parameters are not allowed on views")),
        }
    }

    // Extract all function parameters, except for `self` ones that aren't allowed.
    let typed_args = original_function
        .sig
        .inputs
        .iter()
        .map(|arg| match arg {
            FnArg::Typed(arg) => Ok(arg),
            FnArg::Receiver(_) => Err(syn::Error::new_spanned(arg, "`self` arguments not allowed in views")),
        })
        .collect::<syn::Result<Vec<_>>>()?;

    // Extract all function parameter names.
    let opt_arg_names = typed_args.iter().map(|arg| {
        if let syn::Pat::Ident(i) = &*arg.pat {
            let name = i.ident.to_string();
            quote!(Some(#name))
        } else {
            quote!(None)
        }
    });

    let arg_tys = typed_args.iter().map(|arg| arg.ty.as_ref()).collect::<Vec<_>>();

    // Extract the context type
    let ctx_ty = arg_tys.first().ok_or_else(|| {
        syn::Error::new_spanned(
            original_function.sig.fn_token,
            "`&ViewContext` or `&AnonymousViewContext` must always be the first parameter of a view",
        )
    })?;

    // Extract the return type
    let ret_ty = match &original_function.sig.output {
        syn::ReturnType::Default => None,
        syn::ReturnType::Type(_, t) => Some(&**t),
    }
    .ok_or_else(|| {
        syn::Error::new_spanned(
            original_function.sig.fn_token,
            "views must return one of `T`, `Option<T>`, or `Vec<T>` where `T` is a `SpacetimeType`",
        )
    })?;

    // Extract the non-context parameters
    let arg_tys = arg_tys.iter().skip(1);

    let register_describer_symbol = format!("__preinit__20_register_describer_{}", view_name.value());

    let lt_params = &original_function.sig.generics;
    let lt_where_clause = &lt_params.where_clause;

    let generated_describe_function = quote! {
        #[export_name = #register_describer_symbol]
        pub extern "C" fn __register_describer() {
            spacetimedb::rt::register_view::(#func_name)
        }
    };

    Ok(quote! {
        const _: () = { #generated_describe_function };

        #[allow(non_camel_case_types)]
        #vis struct #func_name { _never: ::core::convert::Infallible }

        const _: () = {
            fn _assert_args #lt_params () #lt_where_clause {
                  let _ = <#ctx_ty  as spacetimedb::rt::ViewContextArg>::_ITEM;
                #(let _ = <#arg_tys as spacetimedb::rt::ViewArg>::_ITEM;)*
            }
        };

        impl #func_name {
            fn invoke(__ctx: #ctx_ty, __args: &[u8]) -> Vec<u8> {
                spacetimedb::rt::invoke_view(#func_name, __ctx, __args)
            }
        }

        #[automatically_derived]
        impl spacetimedb::rt::FnInfo for #func_name {
            type Ctx = #ctx_ty;
            type Invoke = #ctx_ty::Invoke;
            const NAME: &'static str = #view_name;
            const ARG_NAMES: &'static [Option<&'static str>] = &[#(#opt_arg_names),*];
            const INVOKE: Self::Invoke = #func_name::invoke;

            fn push(module: &mut spacetimedb::rt::ModuleBuilder, f: Self::Invoke) {
                <#ctx_ty as spacetimedb::rt::ViewContextArg>::push(module, f);
            }

            fn return_type(ts: &mut impl spacetimedb::sats::typespace::TypespaceBuilder) -> Option<spacetimedb::sats::AlgebraicTypeRef> {
                Some(<#ret_ty as spacetimedb::SpacetimeType>::make_type(ts))
            }
        }
    })
}
