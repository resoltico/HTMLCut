use super::*;
use crate::{Html, Selector};

fn render(element: ElementRef<'_>, include_root: bool, excluded: &HashSet<NodeId>) -> String {
    let mut bytes = Vec::new();
    element
        .write_filtered_html(
            &mut bytes,
            include_root,
            excluded,
            &SelectorWorkBudget::new(1000),
        )
        .unwrap();
    String::from_utf8(bytes).unwrap()
}

#[test]
fn exclusions_are_immutable_and_serialization_preserves_markup_and_escaping() {
    let html = Html::parse_fragment(
        "<div title='a&amp;b'><!--note--><p>A &lt; B</p><aside><b>secret</b></aside><br></div>",
    );
    let root = html
        .select(&Selector::parse("div").unwrap())
        .next()
        .unwrap();
    let aside = html
        .select(&Selector::parse("aside").unwrap())
        .next()
        .unwrap();
    let original = root.html();
    let excluded = HashSet::from([aside.id()]);
    assert_eq!(
        render(root, true, &excluded),
        "<div title=\"a&amp;b\"><!--note--><p>A &lt; B</p><br></div>"
    );
    assert_eq!(
        render(root, false, &excluded),
        "<!--note--><p>A &lt; B</p><br>"
    );
    assert_eq!(root.html(), original);
    assert_eq!(render(root, true, &HashSet::from([root.id()])), "");
    assert_eq!(render(root, false, &HashSet::from([root.id()])), "");
}

#[test]
fn shared_work_exact_boundary_and_writer_failures_are_propagated() {
    let html = Html::parse_fragment("<p>text</p>");
    let root = html.select(&Selector::parse("p").unwrap()).next().unwrap();
    // The root and text each have an open and close traversal edge.
    for (units, success) in [(3, false), (4, true), (5, true)] {
        let mut bytes = Vec::new();
        let budget = SelectorWorkBudget::new(units);
        let result = root.write_filtered_html(&mut bytes, true, &HashSet::new(), &budget);
        assert_eq!(result.is_ok(), success);
        assert_eq!(budget.exhausted(), !success);
        if success {
            assert_eq!(bytes, b"<p>text</p>");
        }
    }
    struct FailsAfter(usize);
    impl Write for FailsAfter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if bytes.len() > self.0 {
                return Err(io::Error::other("bounded writer"));
            }
            self.0 -= bytes.len();
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    // Fail while starting the element, writing text, and ending the element.
    for capacity in [0, 3, 7, 10] {
        let error = root
            .write_filtered_html(
                FailsAfter(capacity),
                true,
                &HashSet::new(),
                &SelectorWorkBudget::new(100),
            )
            .unwrap_err();
        assert_eq!(error.to_string(), "bounded writer");
    }
}

#[test]
fn caller_constructed_special_nodes_follow_the_html_serializer_contract() {
    use crate::node::{Doctype, ProcessingInstruction};
    let mut html = Html::parse_fragment("<div></div>");
    let root_id = html
        .select(&Selector::parse("div").unwrap())
        .next()
        .unwrap()
        .id();
    let mut root = html.tree.get_mut(root_id).unwrap();
    root.append(Node::Doctype(Doctype {
        name: "html".into(),
        public_id: "".into(),
        system_id: "".into(),
    }));
    root.append(Node::Fragment);
    root.append(Node::ProcessingInstruction(ProcessingInstruction {
        target: "test".into(),
        data: "ignored".into(),
    }));
    let root = ElementRef::wrap(html.tree.get(root_id).unwrap()).unwrap();
    assert_eq!(
        render(root, true, &HashSet::new()),
        "<div><!DOCTYPE html></div>"
    );
}
