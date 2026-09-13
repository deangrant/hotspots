//! Extract symbol ranges from Rust source via `syn`.

use std::collections::HashMap;

use hotspots::symbols::SymbolFact;
use syn::spanned::Spanned;
use syn::{ImplItem, Item, TraitItem};

/// Parses Rust source into symbol facts for `path`.
///
/// # Errors
///
/// Returns an error when the source is not valid Rust.
pub fn symbols_from_source(path: &str, source: &str) -> hotspots::Result<Vec<SymbolFact>> {
    let file = syn::parse_file(source)
        .map_err(|e| hotspots::Error::msg(format!("syn parse failed for `{path}`: {e}")))?;
    let mut facts = Vec::new();
    collect_items(path, "", &file.items, &mut facts);
    Ok(disambiguate_names(facts))
}

fn collect_items(path: &str, prefix: &str, items: &[Item], facts: &mut Vec<SymbolFact>) {
    for item in items {
        collect_item(path, prefix, item, facts);
    }
}

fn collect_item(path: &str, prefix: &str, item: &Item, facts: &mut Vec<SymbolFact>) {
    if let Item::Fn(func) = item {
        collect_fn(path, prefix, func, facts);
        return;
    }
    if let Item::Impl(imp) = item {
        collect_impl(path, prefix, imp, facts);
        return;
    }
    collect_container_item(path, prefix, item, facts);
}

fn collect_container_item(path: &str, prefix: &str, item: &Item, facts: &mut Vec<SymbolFact>) {
    match item {
        Item::Trait(tr) => collect_trait(path, prefix, tr, facts),
        Item::Mod(module) => collect_mod(path, prefix, module, facts),
        _ => {}
    }
}

fn collect_fn(path: &str, prefix: &str, func: &syn::ItemFn, facts: &mut Vec<SymbolFact>) {
    let name = qualify(prefix, &func.sig.ident.to_string());
    facts.push(fact(path, &name, func.span()));
}

fn collect_mod(path: &str, prefix: &str, module: &syn::ItemMod, facts: &mut Vec<SymbolFact>) {
    let Some((_, items)) = &module.content else {
        return;
    };
    let child = qualify(prefix, &module.ident.to_string());
    collect_items(path, &child, items, facts);
}

fn collect_impl(path: &str, prefix: &str, imp: &syn::ItemImpl, facts: &mut Vec<SymbolFact>) {
    for item in &imp.items {
        if let ImplItem::Fn(method) = item {
            let base = method_base_name(imp, &method.sig.ident.to_string());
            let name = qualify(prefix, &base);
            facts.push(fact(path, &name, method.span()));
        }
    }
}

fn collect_trait(path: &str, prefix: &str, tr: &syn::ItemTrait, facts: &mut Vec<SymbolFact>) {
    let trait_name = tr.ident.to_string();
    for item in &tr.items {
        if let TraitItem::Fn(method) = item {
            let base = format!("{trait_name}::{}", method.sig.ident);
            let name = qualify(prefix, &base);
            facts.push(fact(path, &name, method.span()));
        }
    }
}

fn method_base_name(imp: &syn::ItemImpl, method: &str) -> String {
    let Some(ty) = type_path_name(&imp.self_ty) else {
        return method.to_owned();
    };
    trait_path_name(imp).map_or_else(
        || format!("{ty}::{method}"),
        |tr| format!("{ty}::{tr}::{method}"),
    )
}

fn type_path_name(ty: &syn::Type) -> Option<String> {
    let syn::Type::Path(path) = ty else {
        return None;
    };
    path.path.segments.last().map(|seg| seg.ident.to_string())
}

fn trait_path_name(imp: &syn::ItemImpl) -> Option<String> {
    let (_, path, _) = imp.trait_.as_ref()?;
    path.segments.last().map(|seg| seg.ident.to_string())
}

fn qualify(prefix: &str, name: &str) -> String {
    if prefix.is_empty() {
        name.to_owned()
    } else {
        format!("{prefix}::{name}")
    }
}

fn fact(path: &str, name: &str, span: proc_macro2::Span) -> SymbolFact {
    let start = u32::try_from(span.start().line).unwrap_or(1);
    let end = u32::try_from(span.end().line).unwrap_or(start).max(start);
    SymbolFact {
        path: path.to_owned(),
        start_line: start,
        end_line: end,
        name: name.to_owned(),
    }
}

fn disambiguate_names(mut facts: Vec<SymbolFact>) -> Vec<SymbolFact> {
    let mut indices_by_name: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, fact) in facts.iter().enumerate() {
        indices_by_name.entry(fact.name.clone()).or_default().push(i);
    }
    for indices in indices_by_name.values() {
        if indices.len() <= 1 {
            continue;
        }
        let mut ordered = indices.clone();
        ordered.sort_by_key(|&i| (facts[i].start_line, facts[i].end_line));
        let base = facts[ordered[0]].name.clone();
        for (n, &i) in ordered.iter().enumerate() {
            facts[i].name = format!("{}#{}", base, n + 1);
        }
    }
    facts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_fn_impl_and_trait_methods() {
        let fn_impl =
            symbols_from_source("a.rs", "fn alpha() {}\nimpl Foo { fn beta(&self) {} }\n")
                .unwrap_or_default();
        assert!(fn_impl.iter().any(|f| f.name == "alpha"));
        assert!(fn_impl.iter().any(|f| f.name == "Foo::beta"));

        let traits = symbols_from_source(
            "a.rs",
            "trait Foo { fn bar(&self); }\nmod a { trait Baz { fn qux(&self); } }\n",
        )
        .unwrap_or_default();
        assert!(traits.iter().any(|f| f.name == "Foo::bar"));
        assert!(traits.iter().any(|f| f.name == "a::Baz::qux"));
    }

    #[test]
    fn suffixes_duplicate_names() {
        let src = "fn twin() {}\nfn twin() {}\n";
        let facts = symbols_from_source("a.rs", src).unwrap_or_default();
        let names: Vec<_> = facts.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"twin#1"));
        assert!(names.contains(&"twin#2"));
        assert!(!names.contains(&"twin"));
    }

    #[test]
    fn qualifies_nested_mod_duplicates() {
        let src = "mod a { fn twin() {} }\nmod b { fn twin() {} }\n";
        let facts = symbols_from_source("a.rs", src).unwrap_or_default();
        let names: Vec<_> = facts.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"a::twin"));
        assert!(names.contains(&"b::twin"));
        assert!(!names.iter().any(|n| n.contains('#')));
    }

    #[test]
    fn qualifies_trait_impl_methods() {
        let src = "\
impl TraitA for Foo { fn m(&self) {} }
impl TraitB for Foo { fn m(&self) {} }
";
        let facts = symbols_from_source("a.rs", src).unwrap_or_default();
        let names: Vec<_> = facts.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"Foo::TraitA::m"));
        assert!(names.contains(&"Foo::TraitB::m"));
    }

    #[test]
    fn duplicate_suffix_follows_start_line() {
        let src = "fn twin() {}\nfn twin() {}\n";
        let facts = symbols_from_source("a.rs", src).unwrap_or_default();
        let first = facts.iter().find(|f| f.name == "twin#1");
        let second = facts.iter().find(|f| f.name == "twin#2");
        assert!(first.is_some_and(|f| f.start_line < second.map_or(0, |s| s.start_line)));
    }

    #[test]
    fn rejects_invalid_rust() {
        assert!(symbols_from_source("a.rs", "fn (").is_err());
    }

    #[test]
    fn skips_empty_mod_and_non_path_impl_type() {
        let src = "\
mod external;
impl (u8,) { fn weird(self) {} }
struct S;
impl S { const N: u8 = 1; }
";
        let facts = symbols_from_source("a.rs", src).unwrap_or_default();
        // Non-path Self type falls back to bare method name; const items are ignored.
        assert!(facts.iter().any(|f| f.name == "weird"));
        assert!(!facts.iter().any(|f| f.name.contains("external")));
    }
}
