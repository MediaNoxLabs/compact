// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
use quote::{ToTokens, quote};
use syn::parse_quote;

fn tokens(items: &[Item]) -> String {
    quote!(#(#items)*).to_string()
}
fn names(items: &[Item]) -> Vec<String> {
    items
        .iter()
        .map(|item| match item {
            Item::Fn(f) => f.sig.ident.to_string(),
            _ => "other".into(),
        })
        .collect()
}
fn retain(helpers: Vec<Item>, roots: &mut [Item]) -> Vec<Item> {
    retain_reachable_helpers(helpers, roots, &mut [], &mut [])
}
#[test]
fn nested_transitive_graph_keeps_stable_order_and_terminates_cycles() {
    // A graph cycle is syntax-only here, not admission of recursive Compact.
    let helpers: Vec<Item> = vec![
        parse_quote!(
            fn b() {
                a();
            }
        ),
        parse_quote!(
            fn unused() {
                other();
            }
        ),
        parse_quote!(
            fn a() {
                if flag {
                    b();
                } else {
                    let f = b;
                    f();
                }
            }
        ),
        parse_quote!(
            fn other() {}
        ),
    ];
    let before = helpers.clone();
    let mut roots = vec![parse_quote!(
        pub fn root() {
            if flag {
                a();
                a();
            }
        }
    )];
    let root_before = tokens(&roots);
    let kept = retain(helpers, &mut roots);
    assert_eq!(names(&kept), ["b", "a"]);
    assert_eq!(
        tokens(&kept),
        tokens(&[before[0].clone(), before[2].clone()])
    );
    assert_eq!(
        tokens(&roots),
        root_before,
        "visitor does not mutate public syntax"
    );
}
#[test]
fn registry_uses_allocated_raw_names_qualified_paths_and_function_values() {
    let helpers = vec![
        parse_quote!(
            fn r#type() {}
        ),
        parse_quote!(
            fn allocated_fallback_2() {}
        ),
        parse_quote!(
            fn dead() {}
        ),
    ];
    let mut roots = vec![parse_quote!(
        pub fn __compact_recorded_body_unrelated() {
            let callback = self::r#type;
            callback();
            crate::recorded::allocated_fallback_2();
        }
    )];
    let before = tokens(&roots);
    let kept = retain(helpers, &mut roots);
    assert_eq!(names(&kept), ["r#type", "allocated_fallback_2"]);
    assert_eq!(
        tokens(&roots),
        before,
        "reserved-looking public functions remain untouched"
    );
    // A shadowed path may over-retain; conservative over-retention is safe.
    let mut roots = vec![parse_quote!(
        pub fn root() {
            let dead = local_callback;
            dead();
        }
    )];
    assert_eq!(
        names(&retain(
            vec![parse_quote!(
                fn dead() {}
            )],
            &mut roots
        )),
        ["dead"]
    );
}
#[test]
fn facade_and_borrowed_methods_are_reference_roots() {
    let helpers = vec![
        parse_quote!(
            fn plain() {}
        ),
        parse_quote!(
            fn borrowed() {}
        ),
        parse_quote!(
            fn dead() {}
        ),
    ];
    let mut plain: Vec<syn::ImplItemFn> = vec![parse_quote!(
        pub fn run(&self) {
            plain();
        }
    )];
    let mut borrowed: Vec<syn::ImplItemFn> = vec![parse_quote!(
        pub fn run(&self) {
            self::borrowed();
        }
    )];
    let before = (
        plain[0].to_token_stream().to_string(),
        borrowed[0].to_token_stream().to_string(),
    );
    let kept = retain_reachable_helpers(helpers, &mut [], &mut plain, &mut borrowed);
    assert_eq!(names(&kept), ["plain", "borrowed"]);
    assert_eq!(
        (
            plain[0].to_token_stream().to_string(),
            borrowed[0].to_token_stream().to_string()
        ),
        before
    );
}
#[test]
fn opaque_macro_attribute_alias_and_verbatim_carriers_retain_candidates() {
    let helpers: Vec<Item> = vec![
        parse_quote!(
            fn helper() {}
        ),
        parse_quote!(
            fn other() {}
        ),
    ];
    for root in [
        parse_quote!(
            pub fn root() {
                unknown!();
            }
        ),
        parse_quote!(
            #[custom_attribute]
            pub fn root() {}
        ),
        parse_quote!(
            pub fn root() {
                use self::helper as alias;
                alias();
            }
        ),
        parse_quote!(
            pub fn root() {
                use self::*;
            }
        ),
        Item::Verbatim(quote!(opaque_source)),
    ] {
        assert_eq!(
            tokens(&retain(helpers.clone(), &mut [root])),
            tokens(&helpers)
        );
    }
    // Even an unreachable candidate's expansion can generate module-level
    // references. Opaque syntax anywhere in candidates preserves the registry.
    for helper in [
        parse_quote!(
            #[custom_attribute]
            fn hidden() {}
        ),
        parse_quote!(
            fn hidden() {
                unknown!();
            }
        ),
    ] {
        let candidates = vec![
            helper,
            parse_quote!(
                fn otherwise_unused() {}
            ),
        ];
        assert_eq!(
            tokens(&retain(candidates.clone(), &mut [])),
            tokens(&candidates)
        );
    }
    let mut root: syn::ItemFn = parse_quote!(
        pub fn root() {}
    );
    root.block.stmts.push(syn::Stmt::Expr(
        syn::Expr::Verbatim(quote!(opaque_expression)),
        None,
    ));
    assert_eq!(
        tokens(&retain(helpers.clone(), &mut [Item::Fn(root)])),
        tokens(&helpers)
    );
    let mut root: Item = parse_quote!(
        pub fn root() {
            let value: u8 = 0;
        }
    );
    let Item::Fn(f) = &mut root else {
        unreachable!()
    };
    let syn::Stmt::Local(local) = &mut f.block.stmts[0] else {
        unreachable!()
    };
    let syn::Pat::Type(pat) = &mut local.pat else {
        unreachable!()
    };
    *pat.ty = syn::Type::Verbatim(quote!(opaque_type));
    assert_eq!(
        tokens(&retain(helpers.clone(), &mut [root])),
        tokens(&helpers)
    );
    let mut root: syn::ItemFn = parse_quote!(
        pub fn root() {
            let value = 0;
        }
    );
    let syn::Stmt::Local(local) = &mut root.block.stmts[0] else {
        unreachable!()
    };
    local.pat = syn::Pat::Verbatim(quote!(opaque_pattern));
    assert_eq!(
        tokens(&retain(helpers.clone(), &mut [Item::Fn(root)])),
        tokens(&helpers)
    );
}
#[test]
fn ambiguous_nonprivate_or_unknown_registry_items_are_preserved() {
    for candidates in [
        vec![
            parse_quote!(
                fn same() {}
            ),
            parse_quote!(
                fn r#same() {}
            ),
        ],
        vec![parse_quote!(
            pub fn public_helper() {}
        )],
        vec![parse_quote!(
            pub(crate) fn crate_helper() {}
        )],
        vec![parse_quote!(
            const VALUE: u8 = 0;
        )],
    ] {
        assert_eq!(
            tokens(&retain(candidates.clone(), &mut [])),
            tokens(&candidates)
        );
    }
}
#[test]
fn no_references_prunes_only_helpers_and_inert_attributes_do_not_block() {
    let mut roots = vec![parse_quote!(
        #[doc = "helper is not a syntactic reference"]
        #[allow(unused_variables)]
        pub fn root() {
            let value = 0;
        }
    )];
    let before = tokens(&roots);
    assert!(
        retain(
            vec![parse_quote!(
                fn helper() {}
            )],
            &mut roots
        )
        .is_empty()
    );
    assert_eq!(tokens(&roots), before);
    assert!(retain(Vec::new(), &mut roots).is_empty());
    assert_eq!(tokens(&roots), before);
}
