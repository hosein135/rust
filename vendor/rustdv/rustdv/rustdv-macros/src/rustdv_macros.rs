//! Proc macros (design-doc §6). Two macros only, per the governing
//! principle — a macro is justified only where Python used runtime
//! dynamism Rust lacks:
//!
//! - `#[rustdv::test]` (§6.1): registers an `async fn` or component test in
//!   the link-time test registry.
//! - `#[derive(Component)]` (§6.3): generates component-tree traversal,
//!   named-port access, and universal factory registration.
//!
//! `syn` parses Rust syntax, `quote` generates Rust tokens, and `linkme`
//! (re-exported by the facade crate) owns the cross-platform linker sections.

use proc_macro::TokenStream;
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{
    Attribute, Data, DeriveInput, Expr, ExprLit, Field, Fields, Item, Lit, LitStr, Result, Token,
    Type, parse_macro_input,
};

// ===========================================================================
// #[rustdv::test]
// ===========================================================================

#[derive(Default)]
struct TestOpts {
    name: Option<String>,
    timeout_time: Option<u64>,
    timeout_unit: Option<String>,
    skip: bool,
    expect_fail: bool,
    expect_error: Option<String>,
}

impl Parse for TestOpts {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut opts = TestOpts::default();
        while !input.is_empty() {
            let key: Ident = input.parse()?;
            let value = if input.peek(Token![=]) {
                input.parse::<Token![=]>()?;
                Some(input.parse::<Expr>()?)
            } else {
                None
            };

            match key.to_string().as_str() {
                "name" => opts.name = Some(string_value(&key, value)?),
                "timeout_time" => opts.timeout_time = Some(integer_value(&key, value)?),
                "timeout_unit" => opts.timeout_unit = Some(string_value(&key, value)?),
                "skip" => opts.skip = flag_value(&key, value)?,
                "expect_fail" => opts.expect_fail = flag_value(&key, value)?,
                "expect_error" => opts.expect_error = Some(string_value(&key, value)?),
                _ => {
                    return Err(syn::Error::new_spanned(
                        key,
                        "unknown #[rustdv::test] option",
                    ));
                }
            }

            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }
        Ok(opts)
    }
}

fn literal(key: &Ident, value: Option<Expr>) -> Result<Lit> {
    match value {
        Some(Expr::Lit(ExprLit { lit, .. })) => Ok(lit),
        Some(other) => Err(syn::Error::new_spanned(
            other,
            format!("{} requires a literal value", key),
        )),
        None => Err(syn::Error::new_spanned(
            key,
            format!("{} requires a value", key),
        )),
    }
}

fn string_value(key: &Ident, value: Option<Expr>) -> Result<String> {
    match literal(key, value)? {
        Lit::Str(value) => Ok(value.value()),
        other => Err(syn::Error::new_spanned(
            other,
            format!("{} requires a string literal", key),
        )),
    }
}

fn integer_value(key: &Ident, value: Option<Expr>) -> Result<u64> {
    match literal(key, value)? {
        Lit::Int(value) => value.base10_parse(),
        other => Err(syn::Error::new_spanned(
            other,
            format!("{} requires an integer literal", key),
        )),
    }
}

fn flag_value(key: &Ident, value: Option<Expr>) -> Result<bool> {
    match value {
        None => Ok(true),
        Some(value) => match literal(key, Some(value))? {
            Lit::Bool(value) => Ok(value.value),
            other => Err(syn::Error::new_spanned(
                other,
                format!("{} requires true or false", key),
            )),
        },
    }
}

enum TestForm {
    Function,
    Component,
}

fn expand_test(attr: TokenStream2, item: TokenStream2) -> Result<TokenStream2> {
    let opts = syn::parse2::<TestOpts>(attr)?;
    let item = syn::parse2::<Item>(item)?;
    let (form, item_name) = match &item {
        Item::Fn(function) => (TestForm::Function, function.sig.ident.clone()),
        Item::Struct(structure) => (TestForm::Component, structure.ident.clone()),
        Item::Type(alias) => (TestForm::Component, alias.ident.clone()),
        other => {
            return Err(syn::Error::new_spanned(
                other,
                "#[rustdv::test] must be applied to an async fn, a struct, or a type alias",
            ));
        }
    };

    let test_name = LitStr::new(
        opts.name.as_deref().unwrap_or(&item_name.to_string()),
        item_name.span(),
    );
    let timeout = match opts.timeout_time {
        Some(time) => {
            let unit = LitStr::new(
                opts.timeout_unit.as_deref().unwrap_or("ns"),
                Span::call_site(),
            );
            quote!(::core::option::Option::Some((#time, #unit)))
        }
        None => quote!(::core::option::Option::None),
    };
    let skip = opts.skip;
    let expect_fail = opts.expect_fail;
    let expect_error = match opts.expect_error {
        Some(value) => {
            let value = LitStr::new(&value, Span::call_site());
            quote!(::core::option::Option::Some(#value))
        }
        None => quote!(::core::option::Option::None),
    };
    let body = match form {
        TestForm::Function => quote!(::std::boxed::Box::pin(super::#item_name(ctx))),
        TestForm::Component => quote! {
            ::std::boxed::Box::pin(async move {
                let mut __ctx = ctx;
                let mut __test = <super::#item_name as ::core::default::Default>::default();
                ::rustdv::run_component_test(&mut __test, &mut __ctx).await
            })
        },
    };
    let registration_module = format_ident!("__rustdv_test_registration_{}", item_name);
    let module_path_constant = format_ident!("__RUSTDV_TEST_MODULE_{}", item_name);

    Ok(quote! {
        #item

        #[doc(hidden)]
        #[allow(non_upper_case_globals)]
        const #module_path_constant: &str = ::core::module_path!();

        #[doc(hidden)]
        #[allow(non_snake_case)]
        mod #registration_module {
            fn shim(
                ctx: ::rustdv::RustdvCtx,
            ) -> ::std::pin::Pin<::std::boxed::Box<
                dyn ::std::future::Future<
                    Output = ::core::result::Result<(), ::rustdv::TestError>
                >,
            >> {
                #body
            }

            #[::rustdv::__private::linkme::distributed_slice(::rustdv::TEST_REGISTRATIONS)]
            #[linkme(crate = ::rustdv::__private::linkme)]
            static REGISTRATION: ::rustdv::TestRegistration = ::rustdv::TestRegistration {
                name: #test_name,
                module: super::#module_path_constant,
                file: ::core::file!(),
                line: ::core::line!(),
                run: shim,
                timeout: #timeout,
                skip: #skip,
                expect_fail: #expect_fail,
                expect_error: #expect_error,
            };
        }
    })
}

/// Both test front doors: an async function or a component/type alias.
#[proc_macro_attribute]
pub fn test(attr: TokenStream, item: TokenStream) -> TokenStream {
    match expand_test(attr.into(), item.into()) {
        Ok(output) => output.into(),
        Err(error) => error.into_compile_error().into(),
    }
}

// ===========================================================================
// #[derive(Component)]
// ===========================================================================

#[derive(Clone, Copy, PartialEq, Eq)]
enum ChildKind {
    RustdvComp,
    Option,
    Vec,
    Plain,
}

struct FieldInfo {
    name: Ident,
    ty: Type,
    child: Option<ChildKind>,
    port: Option<Ident>,
}

fn type_name(ty: &Type) -> Option<&Ident> {
    let Type::Path(path) = ty else { return None };
    if path.qself.is_some() {
        return None;
    }
    path.path.segments.last().map(|segment| &segment.ident)
}

fn child_kind(ty: &Type) -> ChildKind {
    match type_name(ty).map(ToString::to_string).as_deref() {
        Some("RustdvComp") => ChildKind::RustdvComp,
        Some("Option") => ChildKind::Option,
        Some("Vec") => ChildKind::Vec,
        _ => ChildKind::Plain,
    }
}

fn has_attr(attrs: &[Attribute], name: &str) -> bool {
    attrs.iter().any(|attr| attr.path().is_ident(name))
}

fn port_attr(field: &Field) -> Result<Option<Ident>> {
    let mut ports = field
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("port"));
    let Some(attr) = ports.next() else {
        return Ok(None);
    };
    if let Some(duplicate) = ports.next() {
        return Err(syn::Error::new_spanned(
            duplicate,
            "a field may have only one #[port(...)] attribute",
        ));
    }
    attr.parse_args::<Ident>().map(Some)
}

fn fields(input: &DeriveInput) -> Result<Vec<FieldInfo>> {
    let fields = match &input.data {
        Data::Struct(structure) => match &structure.fields {
            Fields::Named(fields) => &fields.named,
            Fields::Unit => return Ok(Vec::new()),
            Fields::Unnamed(fields) => {
                return Err(syn::Error::new_spanned(
                    fields,
                    "#[derive(Component)] requires named fields, or a unit struct",
                ));
            }
        },
        _ => {
            return Err(syn::Error::new(
                input.ident.span(),
                "#[derive(Component)] supports only structs",
            ));
        }
    };

    fields
        .iter()
        .map(|field| {
            let name = field.ident.clone().expect("named fields have identifiers");
            let port = port_attr(field)?;
            let is_child = has_attr(&field.attrs, "component");
            if is_child && port.is_some() {
                return Err(syn::Error::new_spanned(
                    field,
                    "a field cannot be both #[component] and #[port(...)]",
                ));
            }
            Ok(FieldInfo {
                name,
                ty: field.ty.clone(),
                child: is_child.then(|| child_kind(&field.ty)),
                port,
            })
        })
        .collect()
}

fn expand_component(input: DeriveInput) -> Result<TokenStream2> {
    let fields = fields(&input)?;
    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

    let mut visits = Vec::new();
    let mut resolves = Vec::new();
    let mut takes = Vec::new();
    let mut restores = Vec::new();
    for field in fields.iter().filter(|field| field.child.is_some()) {
        let field_name = &field.name;
        let field_label = LitStr::new(&field_name.to_string(), field_name.span());
        match field.child.expect("filtered above") {
            ChildKind::RustdvComp => {
                visits.push(quote! {
                    if let ::core::option::Option::Some(__c) = self.#field_name.as_node_mut() {
                        __out.push((::std::string::String::from(#field_label), __c));
                    }
                });
                resolves.push(quote!(self.#field_name.resolve(__ctx, #field_label);));
                takes.push(quote! {
                    if let ::core::option::Option::Some(__c) = self.#field_name.take_node() {
                        __out.push((::std::string::String::from(#field_label), __c));
                    }
                });
                restores.push(quote! {
                    if __name == #field_label {
                        self.#field_name.put_node(__node);
                        continue;
                    }
                });
            }
            ChildKind::Option => visits.push(quote! {
                if let ::core::option::Option::Some(__c) = &mut self.#field_name {
                    __out.push((
                        ::std::string::String::from(#field_label),
                        __c as &mut (dyn ::rustdv::ComponentNode + 'static),
                    ));
                }
            }),
            ChildKind::Vec => visits.push(quote! {
                for (__i, __c) in self.#field_name.iter_mut().enumerate() {
                    __out.push((
                        ::std::format!("{}[{}]", #field_label, __i),
                        __c as &mut (dyn ::rustdv::ComponentNode + 'static),
                    ));
                }
            }),
            ChildKind::Plain => visits.push(quote! {
                __out.push((
                    ::std::string::String::from(#field_label),
                    &mut self.#field_name as &mut (dyn ::rustdv::ComponentNode + 'static),
                ));
            }),
        }
    }

    let mut port_arms = Vec::new();
    let mut port_items = Vec::new();
    let mut port_consts = Vec::new();
    for field in fields.iter().filter(|field| field.port.is_some()) {
        let field_name = &field.name;
        let field_label = LitStr::new(&field_name.to_string(), field_name.span());
        let kind = field.port.as_ref().expect("filtered above");
        let kind_name = kind.to_string();
        if !matches!(
            kind_name.as_str(),
            "put" | "get" | "peek" | "publish" | "subscribe" | "seq_item"
        ) {
            return Err(syn::Error::new_spanned(
                kind,
                "expected put, get, peek, publish, subscribe, or seq_item",
            ));
        }
        let kind_label = LitStr::new(&kind_name, kind.span());
        let required = !matches!(kind_name.as_str(), "publish" | "subscribe");
        let ty = &field.ty;
        let constant = format_ident!("{}", field_name.to_string().to_uppercase());
        let doc = LitStr::new(
            &format!("The `{}` port, for `connect`.", field_name),
            field_name.span(),
        );

        port_arms.push(quote! {
            #field_label => ::core::option::Option::Some(
                ::rustdv::PortField::slot_any(&self.#field_name)
            ),
        });
        port_items.push(quote! {
            ::rustdv::PortInfo {
                name: #field_label,
                kind: #kind_label,
                required: #required,
                connected: ::rustdv::PortField::bound(&self.#field_name),
            },
        });
        port_consts.push(quote! {
            #[doc = #doc]
            pub const #constant: ::rustdv::PortName<<#ty as ::rustdv::PortField>::Iface> =
                ::rustdv::PortName::new(#field_label);
        });
    }

    let port_impl = (!port_arms.is_empty()).then(|| {
        quote! {
            fn port_slot(
                &self,
                __name: &str,
            ) -> ::core::option::Option<::std::rc::Rc<dyn ::std::any::Any>> {
                match __name {
                    #(#port_arms)*
                    _ => ::core::option::Option::None,
                }
            }

            fn port_infos(&self) -> ::std::vec::Vec<::rustdv::PortInfo> {
                ::std::vec![#(#port_items)*]
            }
        }
    });
    let resolve_impl = (!resolves.is_empty()).then(|| {
        quote! {
            fn resolve_children(&mut self, __ctx: &::rustdv::RustdvCtx) {
                #(#resolves)*
            }
        }
    });
    let take_impl = (!takes.is_empty()).then(|| {
        quote! {
            fn take_children(
                &mut self,
            ) -> ::std::vec::Vec<(
                ::std::string::String,
                ::std::boxed::Box<dyn ::rustdv::ComponentNode>,
            )> {
                let mut __out = ::std::vec::Vec::new();
                #(#takes)*
                __out
            }

            fn restore_children(
                &mut self,
                __taken: ::std::vec::Vec<(
                    ::std::string::String,
                    ::std::boxed::Box<dyn ::rustdv::ComponentNode>,
                )>,
            ) {
                for (__name, __node) in __taken {
                    let __name: &str = &__name;
                    #(#restores)*
                }
            }
        }
    });
    let constants = (!port_consts.is_empty()).then(|| {
        quote! {
            impl #impl_generics #name #type_generics #where_clause {
                #(#port_consts)*
            }
        }
    });

    let registration = generics.params.is_empty().then(|| {
        let module = format_ident!("__rustdv_component_registration_{}", name);
        quote! {
            #[doc(hidden)]
            #[allow(non_snake_case)]
            mod #module {
                fn component_name() -> &'static str {
                    ::core::stringify!(#name)
                }

                fn make() -> ::std::boxed::Box<dyn ::rustdv::ComponentNode> {
                    ::std::boxed::Box::new(
                        <super::#name as ::core::default::Default>::default()
                    )
                }

                #[::rustdv::__private::linkme::distributed_slice(
                    ::rustdv::COMPONENT_REGISTRATIONS
                )]
                #[linkme(crate = ::rustdv::__private::linkme)]
                static REGISTRATION: ::rustdv::ComponentReg = ::rustdv::ComponentReg {
                    name: component_name,
                    make,
                };
            }
        }
    });

    Ok(quote! {
        impl #impl_generics ::rustdv::ComponentNode for #name #type_generics #where_clause {
            fn node_name(&self) -> &'static str {
                ::core::stringify!(#name)
            }

            fn children_mut(
                &mut self,
            ) -> ::std::vec::Vec<(
                ::std::string::String,
                &mut (dyn ::rustdv::ComponentNode + 'static),
            )> {
                let mut __out = ::std::vec::Vec::new();
                #(#visits)*
                __out
            }

            #port_impl
            #resolve_impl
            #take_impl
        }

        impl #impl_generics ::rustdv::PortOwner for #name #type_generics #where_clause {
            fn owner_port_slot(
                &self,
                __name: &str,
            ) -> ::core::option::Option<::std::rc::Rc<dyn ::std::any::Any>> {
                ::rustdv::ComponentNode::port_slot(self, __name)
            }

            fn owner_label(&self) -> &'static str {
                ::core::stringify!(#name)
            }
        }

        #constants
        #registration
    })
}

/// Generates `ComponentNode`, `PortOwner`, port constants, and registration.
#[proc_macro_derive(Component, attributes(component, port))]
pub fn derive_component(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand_component(input) {
        Ok(output) => output.into(),
        Err(error) => error.into_compile_error().into(),
    }
}
