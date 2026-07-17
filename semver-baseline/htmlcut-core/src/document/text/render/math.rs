use ego_tree::NodeRef as DomNodeRef;
use scraper::{ElementRef, Node};

use super::format::collapse_inline_whitespace;
use super::tree::{direct_child_elements, push_inline_text};

pub(super) fn render_math_element(element: &ElementRef<'_>) -> Option<String> {
    let mut rendered = String::new();
    render_math_node(**element, &mut rendered);
    let normalized = collapse_inline_whitespace(rendered.trim());
    if !normalized.is_empty() {
        return Some(normalized);
    }

    element
        .value()
        .attr("alttext")
        .map(collapse_inline_whitespace)
        .filter(|alt| !alt.is_empty())
}

pub(super) fn render_math_node(node: DomNodeRef<'_, Node>, output: &mut String) {
    match node.value() {
        Node::Text(contents) => {
            let text = collapse_inline_whitespace(contents);
            if text.is_empty() {
                return;
            }
            push_inline_text(output, &text);
        }
        Node::Element(data) => {
            let tag_name = data.name();
            if matches!(tag_name, "annotation" | "annotation-xml") {
                return;
            }

            match tag_name {
                "mfrac" => {
                    if let Some(rendered) = render_math_fraction(node) {
                        push_inline_text(output, &rendered);
                        return;
                    }
                }
                "msub" => {
                    if let Some(rendered) = render_math_binary_operator(node, "_") {
                        push_inline_text(output, &rendered);
                        return;
                    }
                }
                "msup" => {
                    if let Some(rendered) = render_math_binary_operator(node, "^") {
                        push_inline_text(output, &rendered);
                        return;
                    }
                }
                "msubsup" => {
                    if let Some(rendered) = render_math_subsup(node) {
                        push_inline_text(output, &rendered);
                        return;
                    }
                }
                "msqrt" => {
                    if let Some(rendered) = render_math_wrapped(node, "sqrt(", ")") {
                        push_inline_text(output, &rendered);
                        return;
                    }
                }
                "mroot" => {
                    if let Some(rendered) = render_math_root(node) {
                        push_inline_text(output, &rendered);
                        return;
                    }
                }
                _ => {}
            }

            for child in node.children() {
                render_math_node(child, output);
            }
        }
        _ => {
            for child in node.children() {
                render_math_node(child, output);
            }
        }
    }
}

pub(super) fn render_math_fraction(node: DomNodeRef<'_, Node>) -> Option<String> {
    let children = direct_child_elements(node);
    if children.len() < 2 {
        return None;
    }

    let numerator = render_math_node_to_string(*children[0]);
    let denominator = render_math_node_to_string(*children[1]);
    if numerator.is_empty() || denominator.is_empty() {
        return None;
    }

    Some(format!(
        "{}/{}",
        wrap_math_operand(&numerator),
        wrap_math_operand(&denominator)
    ))
}

pub(super) fn render_math_binary_operator(
    node: DomNodeRef<'_, Node>,
    operator: &str,
) -> Option<String> {
    let children = direct_child_elements(node);
    if children.len() < 2 {
        return None;
    }

    let left = render_math_node_to_string(*children[0]);
    let right = render_math_node_to_string(*children[1]);
    if left.is_empty() || right.is_empty() {
        return None;
    }

    Some(format!("{left}{operator}{}", wrap_math_operand(&right)))
}

pub(super) fn render_math_subsup(node: DomNodeRef<'_, Node>) -> Option<String> {
    let children = direct_child_elements(node);
    if children.len() < 3 {
        return None;
    }

    let base = render_math_node_to_string(*children[0]);
    let sub = render_math_node_to_string(*children[1]);
    let sup = render_math_node_to_string(*children[2]);
    if base.is_empty() || sub.is_empty() || sup.is_empty() {
        return None;
    }

    Some(format!(
        "{base}_{}^{}",
        wrap_math_operand(&sub),
        wrap_math_operand(&sup)
    ))
}

pub(super) fn render_math_wrapped(
    node: DomNodeRef<'_, Node>,
    prefix: &str,
    suffix: &str,
) -> Option<String> {
    let rendered = render_math_children_to_string(node);
    if rendered.is_empty() {
        return None;
    }

    Some(format!("{prefix}{rendered}{suffix}"))
}

pub(super) fn render_math_root(node: DomNodeRef<'_, Node>) -> Option<String> {
    let children = direct_child_elements(node);
    if children.len() < 2 {
        return None;
    }

    let value = render_math_node_to_string(*children[0]);
    let degree = render_math_node_to_string(*children[1]);
    if value.is_empty() || degree.is_empty() {
        return None;
    }

    Some(format!("root({value}, {degree})"))
}

pub(super) fn render_math_children_to_string(node: DomNodeRef<'_, Node>) -> String {
    let mut rendered = String::new();
    for child in node.children() {
        render_math_node(child, &mut rendered);
    }
    collapse_inline_whitespace(rendered.trim())
}

fn render_math_node_to_string(node: DomNodeRef<'_, Node>) -> String {
    let mut rendered = String::new();
    render_math_node(node, &mut rendered);
    collapse_inline_whitespace(rendered.trim())
}

pub(super) fn wrap_math_operand(operand: &str) -> String {
    if operand.chars().any(|character| character.is_whitespace())
        || operand.contains('/')
        || operand.contains('^')
        || operand.contains('_')
    {
        format!("({operand})")
    } else {
        operand.to_owned()
    }
}
