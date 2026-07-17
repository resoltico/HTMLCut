use super::*;

pub(super) fn normalized_body_text_char_count(document: &Html) -> usize {
    let mut rendered = String::new();
    if let Some(body) = first_body(document) {
        collect_visible_text_for_count(body.children(), &mut rendered);
    } else if select_first(document, "html").is_some() {
        collect_visible_text_for_count(document.root_element().children(), &mut rendered);
    } else {
        collect_visible_text_for_count(document.tree.root().children(), &mut rendered);
    }

    rendered.chars().count()
}

pub(super) fn collect_visible_text_for_count<'a>(
    nodes: impl Iterator<Item = ego_tree::NodeRef<'a, Node>>,
    output: &mut String,
) {
    for node in nodes {
        match node.value() {
            Node::Text(contents) => {
                let normalized = collapse_inline_whitespace_for_count(contents);
                if normalized.is_empty() {
                    continue;
                }
                push_count_text(output, &normalized);
            }
            Node::Element(data) => {
                if matches!(
                    data.name(),
                    "head" | "noscript" | "script" | "style" | "template"
                ) {
                    continue;
                }

                if data.name() == "img" {
                    let alt_text = data
                        .attr("alt")
                        .map(collapse_inline_whitespace_for_count)
                        .filter(|alt| !alt.is_empty());
                    if let Some(alt_text) = alt_text {
                        push_count_text(output, &alt_text);
                    }
                    continue;
                }

                collect_visible_text_for_count(node.children(), output);
            }
            _ => collect_visible_text_for_count(node.children(), output),
        }
    }
}

pub(super) fn push_count_text(output: &mut String, text: &str) {
    if output
        .chars()
        .last()
        .is_some_and(|character| !character.is_whitespace())
    {
        output.push(' ');
    }
    output.push_str(text);
}

pub(super) fn collapse_inline_whitespace_for_count(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}
