use ego_tree::NodeRef as DomNodeRef;
use scraper::{ElementRef, Node};

use super::TextRenderIntent;
use super::format::{collapse_blank_lines, normalize_structured_line};
use super::tree::{direct_child_elements, render_children_to_string};

pub(super) fn render_table(
    node: DomNodeRef<'_, Node>,
    in_pre: bool,
    intent: TextRenderIntent,
) -> String {
    let caption = table_caption(node, in_pre, intent);
    let mut rows = Vec::<Vec<String>>::new();
    collect_table_rows_with_intent(node, in_pre, intent, &mut rows);
    rows.retain(|row| row.iter().any(|cell| !cell.is_empty()));
    if rows.is_empty() {
        return caption.unwrap_or_default();
    }

    let column_count = rows
        .iter()
        .map(Vec::len)
        .max()
        .expect("non-empty rendered tables must have at least one column");

    for row in &mut rows {
        row.resize(column_count, String::new());
    }

    let widths = (0..column_count)
        .map(|column_index| {
            rows.iter()
                .map(|row| row[column_index].chars().count())
                .max()
                .unwrap_or(0)
        })
        .collect::<Vec<_>>();

    let rendered_rows = rows
        .into_iter()
        .map(|row| format_table_row(&row, &widths))
        .collect::<Vec<_>>()
        .join("\n")
        .trim_matches('\n')
        .to_owned();

    match caption {
        Some(caption) => format!("{caption}\n{rendered_rows}"),
        None => rendered_rows,
    }
}

#[cfg(test)]
pub(super) fn collect_table_rows(
    node: DomNodeRef<'_, Node>,
    in_pre: bool,
    rows: &mut Vec<Vec<String>>,
) {
    collect_table_rows_with_intent(node, in_pre, TextRenderIntent::ReaderDocument, rows);
}

fn collect_table_rows_with_intent(
    node: DomNodeRef<'_, Node>,
    in_pre: bool,
    intent: TextRenderIntent,
    rows: &mut Vec<Vec<String>>,
) {
    let Some(element) = ElementRef::wrap(node) else {
        return;
    };

    match element.value().name() {
        "tr" => {
            let row = direct_child_elements(node)
                .into_iter()
                .filter(|cell| matches!(cell.value().name(), "td" | "th"))
                .map(|cell| render_table_cell(cell, in_pre, intent))
                .collect::<Vec<_>>();
            if !row.is_empty() {
                rows.push(row);
            }
        }
        "table" | "thead" | "tbody" | "tfoot" => {
            for child in node.children() {
                collect_table_rows_with_intent(child, in_pre, intent, rows);
            }
        }
        _ => {}
    }
}

fn render_table_cell(cell: ElementRef<'_>, in_pre: bool, intent: TextRenderIntent) -> String {
    let rendered = render_children_to_string(*cell, in_pre, false, intent);
    normalize_table_cell_text(&rendered)
}

fn table_caption(
    node: DomNodeRef<'_, Node>,
    in_pre: bool,
    intent: TextRenderIntent,
) -> Option<String> {
    direct_child_elements(node)
        .into_iter()
        .find(|child| child.value().name() == "caption")
        .and_then(|caption| {
            let rendered = render_children_to_string(*caption, in_pre, false, intent);
            let normalized = collapse_blank_lines(&rendered)
                .lines()
                .map(normalize_structured_line)
                .filter(|line| !line.is_empty())
                .collect::<Vec<_>>()
                .join(" ");
            (!normalized.is_empty()).then_some(normalized)
        })
}

fn normalize_table_cell_text(rendered: &str) -> String {
    collapse_blank_lines(rendered)
        .lines()
        .map(normalize_structured_line)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" / ")
}

fn format_table_row(row: &[String], widths: &[usize]) -> String {
    let mut line = String::new();

    for (index, cell) in row.iter().enumerate() {
        if index > 0 {
            line.push_str(" | ");
        }

        line.push_str(cell);
        if index + 1 != row.len() {
            line.push_str(&" ".repeat(widths[index].saturating_sub(cell.chars().count())));
        }
    }

    line.trim_end().to_owned()
}
