use std::fs;
use std::path::Path;

use syn::{File, ImplItem, Item, ItemMacro, TraitItem};

use crate::model::{CoverageSourceKind, DynResult};

/// Classifies one tracked Rust source file by whether it contains executable semantics.
pub fn coverage_source_kind(path: &Path) -> DynResult<CoverageSourceKind> {
    let source = fs::read_to_string(path)?;
    let syntax = syn::parse_file(&source).map_err(|error| {
        format!(
            "invalid Rust source {} while classifying coverage policy: {error}",
            path.display()
        )
    })?;

    if file_is_declarative_only(&syntax) {
        Ok(CoverageSourceKind::DeclarativeOnly)
    } else {
        Ok(CoverageSourceKind::Executable)
    }
}

fn file_is_declarative_only(file: &File) -> bool {
    file.items.iter().all(item_is_declarative_only)
}

fn item_is_declarative_only(item: &Item) -> bool {
    match item {
        Item::Const(_)
        | Item::Enum(_)
        | Item::ExternCrate(_)
        | Item::ForeignMod(_)
        | Item::Static(_)
        | Item::Struct(_)
        | Item::TraitAlias(_)
        | Item::Type(_)
        | Item::Union(_)
        | Item::Use(_) => true,
        Item::Impl(item_impl) => item_impl.items.iter().all(impl_item_is_declarative_only),
        Item::Macro(item_macro) => macro_is_definition(item_macro),
        Item::Mod(item_mod) => item_mod
            .content
            .as_ref()
            .is_none_or(|(_, items)| items.iter().all(item_is_declarative_only)),
        Item::Trait(item_trait) => item_trait.items.iter().all(trait_item_is_declarative_only),
        Item::Fn(_) | Item::Verbatim(_) | _ => false,
    }
}

fn impl_item_is_declarative_only(item: &ImplItem) -> bool {
    match item {
        ImplItem::Const(_) | ImplItem::Type(_) => true,
        ImplItem::Fn(_) | ImplItem::Macro(_) | ImplItem::Verbatim(_) | _ => false,
    }
}

fn trait_item_is_declarative_only(item: &TraitItem) -> bool {
    match item {
        TraitItem::Fn(item_fn) => item_fn.default.is_none(),
        TraitItem::Const(_) | TraitItem::Type(_) | TraitItem::Macro(_) => true,
        TraitItem::Verbatim(_) | _ => false,
    }
}

fn macro_is_definition(item_macro: &ItemMacro) -> bool {
    item_macro.ident.is_some() || item_macro.mac.path.is_ident("macro_rules")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verbatim_trait_items_are_executable_policy_surfaces() {
        assert!(!trait_item_is_declarative_only(&TraitItem::Verbatim(
            Default::default()
        )));
    }
}
