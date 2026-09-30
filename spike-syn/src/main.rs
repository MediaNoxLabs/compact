//! Feasibility spike for compact#95.
//!
//! The decisive question the prototype did not answer: the Rust AST crates
//! (`syn`, `quote`, `prettyplease`) are Rust libraries, and the Compact
//! emitter is Chez Scheme. So "emit through the Rust AST" is not a refactor
//! of the emitter — it needs either an interchange format between the two
//! languages, or the backend moving to Rust.
//!
//! This reads a small JSON description of a Rust item — the sort of thing a
//! Scheme pass could plausibly write — builds a real `syn` tree from it, and
//! prints it with `prettyplease`. If that works, option 2 in the issue (Scheme
//! emits an IR, a Rust tool renders it) is live. If it does not, the initiative
//! reduces to "our own typed IR and our own printer", which is a much smaller
//! claim and should be judged on its own terms.

use serde::Deserialize;

/// The interchange shape. Deliberately minimal: enough to express the type
/// positions the Scheme prototype already models, and nothing more. Widening
/// it to cover expressions is the expensive part, and the point of the spike
/// is to learn what widening would cost before paying for it.
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum TypeIr {
    Prim { name: String },
    Array { elem: Box<TypeIr>, len: usize },
    Tuple { elems: Vec<TypeIr> },
    Path { segments: Vec<String> },
    Generic { head: Box<TypeIr>, args: Vec<TypeIr> },
}

#[derive(Debug, Deserialize)]
struct FieldIr {
    name: String,
    ty: TypeIr,
}

#[derive(Debug, Deserialize)]
struct StructIr {
    name: String,
    fields: Vec<FieldIr>,
}

/// The whole question in one function: does a foreign IR turn into a real
/// `syn::Type`?
fn to_syn_type(t: &TypeIr) -> syn::Type {
    match t {
        TypeIr::Prim { name } => syn::parse_str::<syn::Type>(name)
            .unwrap_or_else(|e| panic!("primitive {name:?} is not a Rust type: {e}")),
        TypeIr::Array { elem, len } => {
            let inner = to_syn_type(elem);
            let n = proc_macro2::Literal::usize_unsuffixed(*len);
            syn::parse_quote!([#inner; #n])
        }
        TypeIr::Tuple { elems } => {
            let parts: Vec<syn::Type> = elems.iter().map(to_syn_type).collect();
            // FINDING (compact#95): the first version of this arm was
            //
            //     syn::parse_quote!((#(#parts),*))
            //
            // with a comment claiming `syn` settles the 1-tuple trailing
            // comma "which the text emitter has to remember by hand". That
            // comment was wrong and the spike printed `(Fr)` for a 1-tuple —
            // which Rust reads as `Fr` in redundant parentheses, not as a
            // one-element tuple. A silent type change, in generated code, of
            // exactly the kind this initiative is meant to prevent.
            //
            // The typed tree did not save me. `parse_quote!` takes tokens and
            // parses them, so a malformed token sequence yields a
            // well-formed-but-wrong `syn::Type` just as a malformed string
            // yields wrong text. The safety has to come from the *constructor*
            // being shaped so the mistake cannot be expressed — building
            // `syn::TypeTuple` directly, where the trailing comma is a field
            // of the data structure rather than a character you remember.
            let mut tuple = syn::TypeTuple {
                paren_token: syn::token::Paren::default(),
                elems: syn::punctuated::Punctuated::new(),
            };
            for p in &parts {
                tuple.elems.push_value(p.clone());
                tuple.elems.push_punct(syn::token::Comma::default());
            }
            // A 2+-tuple does not need the trailing punctuation; a 1-tuple
            // does, and `Punctuated` records that distinction as data.
            if parts.len() > 1 {
                tuple.elems.pop_punct();
            }
            syn::Type::Tuple(tuple)
        }
        TypeIr::Path { segments } => {
            let joined = segments.join("::");
            syn::parse_str::<syn::Type>(&joined)
                .unwrap_or_else(|e| panic!("path {joined:?} is not a Rust type: {e}"))
        }
        TypeIr::Generic { head, args } => {
            let h = to_syn_type(head);
            let a: Vec<syn::Type> = args.iter().map(to_syn_type).collect();
            syn::parse_quote!(#h<#(#a),*>)
        }
    }
}

fn to_syn_struct(s: &StructIr) -> syn::ItemStruct {
    let name = syn::Ident::new(&s.name, proc_macro2::Span::call_site());
    let fields: Vec<syn::Field> = s
        .fields
        .iter()
        .map(|f| {
            let fname = syn::Ident::new(&f.name, proc_macro2::Span::call_site());
            let fty = to_syn_type(&f.ty);
            syn::parse_quote!(pub #fname: #fty)
        })
        .collect();
    syn::parse_quote! {
        #[derive(Clone, Debug)]
        pub struct #name {
            #(#fields),*
        }
    }
}

fn main() {
    let input = std::io::read_to_string(std::io::stdin()).expect("read stdin");
    let s: StructIr = serde_json::from_str(&input).expect("parse IR");
    let item = to_syn_struct(&s);
    let file = syn::File {
        shebang: None,
        attrs: vec![],
        items: vec![syn::Item::Struct(item)],
    };
    print!("{}", prettyplease::unparse(&file));
}
