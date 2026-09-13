//! Extract symbol ranges from Rust source via `syn`.

use std::collections::HashMap;

use hotspots::symbols::SymbolFact;
use syn::spanned::Spanned;
use syn::{ImplItem, Item};

/// Parses Rust source into symbol facts for `path`.
///
/// # Errors
///
/// Returns an error when the source is not valid Rust.
pub fn symbols_from_source(path: &str, source: &str) -> hotspots::Result<Vec<SymbolFact>> {
    let file = syn::parse_file(source)
        .map_err(|e| hotspots::Error::msg(format!("syn parse failed for `{path}`: {e}")))?;
    let mut facts = Vec::new();
    collect_items(path, &file.items, &mut facts);
    Ok(disambiguate_names(facts))
}

fn collect_items(path: &str, items: &[Item], facts: &mut Vec<SymbolFact>) {
    for item in items {
        match item {
            Item::Fn(func) => facts.push(fact(path, &func.sig.ident.to_string(), func.span())),
            Item::Impl(imp) => collect_impl(path, imp, facts),
            Item::Mod(module) => {
                if let Some((_, items)) = &module.content {
                    collect_items(path, items, facts);
                }
            }
            _ => {}
        }
    }
}

fn collect_impl(path: &str, imp: &syn::ItemImpl, facts: &mut Vec<SymbolFact>) {
    let type_name = type_path_name(&imp.self_ty);
    for item in &imp.items {
        if let ImplItem::Fn(method) = item {
            let name = type_name.as_ref().map_or_else(
                || method.sig.ident.to_string(),
                |ty| format!("{ty}::{}", method.sig.ident),
            );
            facts.push(fact(path, &name, method.span()));
        }
    }
}

fn type_path_name(ty: &syn::Type) -> Option<String> {
    let syn::Type::Path(path) = ty else {
        return None;
    };
    path.path.segments.last().map(|seg| seg.ident.to_string())
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
    let mut counts: HashMap<String, usize> = HashMap::new();
    for fact in &facts {
        *counts.entry(fact.name.clone()).or_insert(0) += 1;
    }
    let mut seen: HashMap<String, usize> = HashMap::new();
    for fact in &mut facts {
        let total = counts.get(&fact.name).copied().unwrap_or(1);
        if total <= 1 {
            continue;
        }
        let n = seen.entry(fact.name.clone()).or_insert(0);
        *n += 1;
        if *n > 1 {
            fact.name = format!("{}#{}", fact.name, *n);
        }
    }
    facts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_fn_and_impl_method() {
        let src = "fn alpha() {}\nimpl Foo { fn beta(&self) {} }\n";
        let facts = symbols_from_source("a.rs", src).unwrap_or_default();
        assert!(facts.iter().any(|f| f.name == "alpha"));
        assert!(facts.iter().any(|f| f.name == "Foo::beta"));
    }

    #[test]
    fn suffixes_duplicate_names() {
        let src = "fn twin() {}\nfn twin() {}\n";
        let facts = symbols_from_source("a.rs", src).unwrap_or_default();
        let names: Vec<_> = facts.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"twin"));
        assert!(names.contains(&"twin#2"));
    }

    #[test]
    fn rejects_invalid_rust() {
        assert!(symbols_from_source("a.rs", "fn (").is_err());
    }
}
