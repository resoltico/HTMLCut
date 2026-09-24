//! Detached subtree cloning without reparsing serialized HTML.

use ego_tree::NodeId;

use super::Html;

impl Html {
    /// Clones one existing subtree into a new fragment without reparsing serialized HTML.
    ///
    /// The returned tree has exactly one child below its fragment root. This is useful when a
    /// caller needs an isolated mutable projection of one selected element but must not duplicate
    /// or parse the complete source document.
    pub fn clone_subtree_as_fragment(&self, source_id: NodeId) -> Option<Self> {
        self.tree.get(source_id)?;

        let mut fragment = Self::new_fragment();
        fragment.quirks_mode = self.quirks_mode;
        #[cfg(feature = "errors")]
        {
            fragment.errors = self.errors.clone();
        }

        let fragment_root_id = fragment.tree.root().id();
        let mut pending = vec![(source_id, fragment_root_id)];

        while let Some((current_source_id, target_parent_id)) = pending.pop() {
            let source = self.tree.get(current_source_id)?;
            let children = source
                .children()
                .map(|child| child.id())
                .collect::<Vec<_>>();
            let target_id = fragment
                .tree
                .get_mut(target_parent_id)
                .expect("detached fragment parent must remain present")
                .append(source.value().clone())
                .id();

            for child_id in children.into_iter().rev() {
                pending.push((child_id, target_id));
            }
        }

        Some(fragment)
    }
}

#[cfg(test)]
mod tests {
    use super::Html;
    use crate::{ElementRef, Selector};

    #[test]
    fn clone_subtree_as_fragment_preserves_the_selected_tree_only() {
        let document = Html::parse_document(
            "<html><body><aside>outside</aside><article><h1>Title</h1></article></body></html>",
        );
        let selector = Selector::parse("article").expect("selector");
        let article = document.select(&selector).next().expect("article");

        let fragment = document
            .clone_subtree_as_fragment(article.id())
            .expect("selected node exists");
        let cloned = fragment
            .tree
            .root()
            .children()
            .find_map(ElementRef::wrap)
            .expect("fragment selected root");

        assert_eq!(cloned.html(), "<article><h1>Title</h1></article>");
        assert_eq!(document.select(&selector).count(), 1);
    }

    #[test]
    fn clone_subtree_as_fragment_refuses_an_out_of_tree_node_id() {
        let document = Html::parse_document("<main><article>One</article></main>");
        let selector = Selector::parse("article").expect("selector");
        let article_id = document.select(&selector).next().expect("article").id();
        let empty_document = Html::new_document();

        assert!(
            empty_document
                .clone_subtree_as_fragment(article_id)
                .is_none()
        );
    }
}
