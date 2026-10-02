//! Bounded immutable element facts for generic matcher contract tests.
use crate::attr::{AttrSelectorOperation, CaseSensitivity, NamespaceConstraint};
use crate::bloom::BloomFilter;
use crate::matching::{ElementSelectorFlags, MatchingContext};
use crate::parser::tests::{
    DummyAtom, DummyAttrValue, DummySelectorImpl, PseudoClass, PseudoElement,
};
use crate::tree::{Element, OpaqueElement};
use cssparser::ToCss;
use precomputed_hash::PrecomputedHash;
use std::cell::{Cell, RefCell};

#[derive(Debug, Default)]
pub struct Record {
    pub name: &'static str,
    pub id: &'static str,
    pub parent: Option<usize>,
    pub namespace: &'static str,
    pub classes: &'static [&'static str],
    pub attrs: &'static [(&'static str, &'static str, &'static str)],
    pub link: bool,
    pub hover: bool,
    pub active: bool,
    pub text: bool,
    pub pseudo: Option<PseudoElement>,
    pub slot: Option<usize>,
    pub parts: &'static [&'static str],
    pub exports: &'static [(&'static str, &'static str)],
    pub ignore_nth: bool,
}

pub struct Tree {
    pub records: Vec<Record>,
    pub hash_visits: Cell<usize>,
    pub flags: RefCell<Vec<usize>>,
}
impl Tree {
    pub fn new(records: Vec<Record>) -> Self {
        let flags = RefCell::new(vec![0; records.len()]);
        Self {
            records,
            flags,
            hash_visits: Cell::new(0),
        }
    }
    pub fn element(&self, index: usize) -> Node<'_> {
        Node { tree: self, index }
    }
}
#[derive(Clone, Copy)]
pub struct Node<'a> {
    pub tree: &'a Tree,
    pub index: usize,
}
impl std::fmt::Debug for Node<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ElementFact").field(&self.index).finish()
    }
}
impl Node<'_> {
    fn record(&self) -> &Record {
        &self.tree.records[self.index]
    }
    fn sibling(&self, forward: bool) -> Option<Self> {
        let parent = self.record().parent?;
        let mut siblings = self
            .tree
            .records
            .iter()
            .enumerate()
            .filter(|(_, record)| record.parent == Some(parent) && record.name != "#shadow");
        let index = if forward {
            siblings
                .find(|(index, _)| *index > self.index)
                .map(|(index, _)| index)
        } else {
            siblings
                .filter(|(index, _)| *index < self.index)
                .last()
                .map(|(index, _)| index)
        };
        index.map(|index| self.tree.element(index))
    }
}
impl Element for Node<'_> {
    type Impl = DummySelectorImpl;
    fn opaque(&self) -> OpaqueElement {
        OpaqueElement::new(self.record())
    }
    fn parent_element(&self) -> Option<Self> {
        self.record()
            .parent
            .filter(|index| self.tree.records[*index].name != "#shadow")
            .map(|index| self.tree.element(index))
    }
    fn parent_node_is_shadow_root(&self) -> bool {
        self.record()
            .parent
            .is_some_and(|index| self.tree.records[index].name == "#shadow")
    }
    fn containing_shadow_host(&self) -> Option<Self> {
        let mut parent = self.record().parent;
        while let Some(index) = parent {
            let record = &self.tree.records[index];
            if record.name == "#shadow" {
                return record.parent.map(|index| self.tree.element(index));
            }
            parent = record.parent;
        }
        None
    }
    fn is_pseudo_element(&self) -> bool {
        self.record().pseudo.is_some()
    }
    fn prev_sibling_element(&self) -> Option<Self> {
        self.sibling(false)
    }
    fn next_sibling_element(&self) -> Option<Self> {
        self.sibling(true)
    }
    fn first_element_child(&self) -> Option<Self> {
        self.tree
            .records
            .iter()
            .position(|record| record.parent == Some(self.index) && record.name != "#shadow")
            .map(|index| self.tree.element(index))
    }
    fn is_html_element_in_html_document(&self) -> bool {
        self.record().namespace.is_empty()
    }
    fn has_local_name(&self, name: &DummyAtom) -> bool {
        *name == DummyAtom::from(self.record().name)
    }
    fn has_namespace(&self, name: &DummyAtom) -> bool {
        *name == DummyAtom::from(self.record().namespace)
    }
    fn is_same_type(&self, other: &Self) -> bool {
        self.record().name == other.record().name
            && self.record().namespace == other.record().namespace
    }
    fn attr_matches(
        &self,
        namespace: &NamespaceConstraint<&DummyAtom>,
        name: &DummyAtom,
        operation: &AttrSelectorOperation<&DummyAttrValue>,
    ) -> bool {
        self.record().attrs.iter().any(|(ns, local, value)| {
            let namespace_matches = match namespace {
                NamespaceConstraint::Any => true,
                NamespaceConstraint::Specific(required) => **required == DummyAtom::from(*ns),
            };
            namespace_matches && *name == DummyAtom::from(*local) && operation.eval_str(value)
        })
    }
    fn match_non_ts_pseudo_class(
        &self,
        class: &PseudoClass,
        _: &mut MatchingContext<Self::Impl>,
    ) -> bool {
        match class {
            PseudoClass::Hover => self.record().hover,
            PseudoClass::Active => self.record().active,
            PseudoClass::Lang(language) => self.record().attrs.iter().any(|(_, name, value)| {
                *name == "lang"
                    && (value.eq_ignore_ascii_case(language)
                        || value
                            .to_ascii_lowercase()
                            .starts_with(&format!("{}-", language.to_ascii_lowercase())))
            }),
        }
    }
    fn match_pseudo_element(
        &self,
        pseudo: &PseudoElement,
        _: &mut MatchingContext<Self::Impl>,
    ) -> bool {
        self.record().pseudo.as_ref() == Some(pseudo)
    }
    fn apply_selector_flags(&self, flags: ElementSelectorFlags) {
        let mut recorded = self.tree.flags.borrow_mut();
        recorded[self.index] |= flags.for_self().bits();
        if let Some(parent) = self.parent_element() {
            recorded[parent.index] |= flags.for_parent().bits();
        }
    }
    fn is_link(&self) -> bool {
        self.record().link
    }
    fn is_html_slot_element(&self) -> bool {
        self.record().name == "slot"
    }
    fn assigned_slot(&self) -> Option<Self> {
        self.record().slot.map(|index| self.tree.element(index))
    }
    fn has_id(&self, id: &DummyAtom, sensitivity: CaseSensitivity) -> bool {
        sensitivity.eq(self.record().id.as_bytes(), id.to_css_string().as_bytes())
    }
    fn has_class(&self, class: &DummyAtom, sensitivity: CaseSensitivity) -> bool {
        self.record()
            .classes
            .iter()
            .any(|name| sensitivity.eq(name.as_bytes(), class.to_css_string().as_bytes()))
    }
    fn has_custom_state(&self, name: &DummyAtom) -> bool {
        *name == DummyAtom::from("ready")
    }
    fn imported_part(&self, name: &DummyAtom) -> Option<DummyAtom> {
        self.record()
            .exports
            .iter()
            .find(|(outer, _)| *name == DummyAtom::from(*outer))
            .map(|(_, inner)| DummyAtom::from(*inner))
    }
    fn is_part(&self, name: &DummyAtom) -> bool {
        self.record()
            .parts
            .iter()
            .any(|part| *name == DummyAtom::from(*part))
    }
    fn is_empty(&self) -> bool {
        !self.record().text && self.first_element_child().is_none()
    }
    fn is_root(&self) -> bool {
        self.record().parent.is_none()
    }
    fn ignores_nth_child_selectors(&self) -> bool {
        self.record().ignore_nth
    }
    fn add_element_unique_hashes(&self, filter: &mut BloomFilter) -> bool {
        self.tree.hash_visits.set(self.tree.hash_visits.get() + 1);
        filter.insert_hash(DummyAtom::from(self.record().namespace).precomputed_hash());
        for name in [self.record().name, self.record().id] {
            if !name.is_empty() {
                filter.insert_hash(DummyAtom::from(name).precomputed_hash());
            }
        }
        for class in self.record().classes {
            filter.insert_hash(DummyAtom::from(*class).precomputed_hash());
        }
        true
    }
}
