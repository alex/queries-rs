use quote::ToTokens;

#[proc_macro_attribute]
pub fn queries(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let args = syn::parse_macro_input!(attr as syn::MetaNameValue);
    let input = syn::parse_macro_input!(item as syn::ItemTrait);

    expand(args, input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn expand(
    args: syn::MetaNameValue,
    input: syn::ItemTrait,
) -> syn::Result<proc_macro2::TokenStream> {
    if !args.path.is_ident("database") {
        return Err(syn::Error::new_spanned(
            args,
            "The only permitted argument is database.",
        ));
    }
    let database = args.value;

    if input.unsafety.is_some()
        || input.auto_token.is_some()
        || input.restriction.is_some()
        || !input.generics.params.is_empty()
        || input.generics.where_clause.is_some()
        || !input.supertraits.is_empty()
    {
        return Err(syn::Error::new_spanned(
            input,
            "Used an unsupported feature in trait definition",
        ));
    }

    let mut pool_method_impls = vec![];
    let mut tx_method_impls = vec![];

    for item in input.items {
        let syn::TraitItem::Fn(fn_def) = item else {
            return Err(syn::Error::new_spanned(
                item,
                "Only methods are allowed in the trait definition",
            ));
        };
        pool_method_impls.push(expand_pool_method_impl(&database, fn_def.clone())?);
        tx_method_impls.push(expand_tx_method_impl(&database, fn_def)?);
    }

    let name = input.ident;
    let vis = input.vis;

    let result = quote::quote! {
        #vis struct #name<E> {
            executor: E,
        }

        impl #name<sqlx::Pool<#database>> {
            pub fn from_pool(pool: sqlx::Pool<#database>) -> Self {
                Self { executor: pool }
            }

            pub async fn begin(&self) -> sqlx::Result<#name<sqlx::Transaction<'_, #database>>> {
                let tx = self.executor.begin().await?;
                Ok(#name::from_tx(tx))
            }

            #(#pool_method_impls)*
        }

        impl<'a> #name<sqlx::Transaction<'a, #database>> {
            pub fn from_tx(tx: sqlx::Transaction<'a, #database>) -> Self {
                Self { executor: tx }
            }

            pub async fn commit(self) -> sqlx::Result<()> {
                self.executor.commit().await
            }

            pub async fn rollback(self) -> sqlx::Result<()> {
                self.executor.rollback().await
            }

            #(#tx_method_impls)*
        }
    };
    Ok(result)
}

fn expand_method_impl_with_self(
    database: &syn::Expr,
    fn_def: syn::TraitItemFn,
    self_param: proc_macro2::TokenStream,
    executor_ref: proc_macro2::TokenStream,
) -> syn::Result<proc_macro2::TokenStream> {
    if fn_def.default.is_some() {
        return Err(syn::Error::new_spanned(
            fn_def,
            "Default implementations are not allowed",
        ));
    }

    if fn_def.sig.asyncness.is_none() {
        return Err(syn::Error::new_spanned(fn_def.sig, "Method must be async"));
    }

    for attr in &fn_def.attrs {
        if !attr.path().is_ident("query") {
            return Err(syn::Error::new_spanned(
                attr,
                "Only #[query] attributes are allowed",
            ));
        }
    }

    let query = &fn_def.attrs[0].meta.require_name_value()?.value;
    let name = &fn_def.sig.ident;
    let args = &fn_def.sig.inputs;
    let arg_names = args
        .iter()
        .map(|p| {
            let syn::FnArg::Typed(pat) = p else {
                return Err(syn::Error::new_spanned(p, "weird arg"));
            };
            let syn::Pat::Ident(i) = &*pat.pat else {
                return Err(syn::Error::new_spanned(pat, "weird arg"));
            };
            Ok(&i.ident)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let return_type = match &fn_def.sig.output {
        syn::ReturnType::Default => quote::quote! { () },
        syn::ReturnType::Type(_, ty) => ty.into_token_stream(),
    };

    let result = quote::quote! {
        async fn #name(#self_param, #args) -> Result<#return_type, sqlx::Error>
        {
            use queries::Probe;

            let q = sqlx::query(#query);
            #(let q = q.bind(#arg_names);)*
            <
                #return_type as queries::FromRows<
                    #database,
                    { queries::FromRowsCategory::<#return_type>::VALUE }
                >
            >::from_rows(q.fetch(#executor_ref)).await
        }
    };
    Ok(result)
}

fn expand_pool_method_impl(
    database: &syn::Expr,
    fn_def: syn::TraitItemFn,
) -> syn::Result<proc_macro2::TokenStream> {
    expand_method_impl_with_self(
        database,
        fn_def,
        quote::quote! { &self },
        quote::quote! { &self.executor },
    )
}

fn expand_tx_method_impl(
    database: &syn::Expr,
    fn_def: syn::TraitItemFn,
) -> syn::Result<proc_macro2::TokenStream> {
    expand_method_impl_with_self(
        database,
        fn_def,
        quote::quote! { &mut self },
        quote::quote! { &mut *self.executor },
    )
}
