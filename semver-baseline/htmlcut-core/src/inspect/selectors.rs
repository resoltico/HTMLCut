use super::*;

pub(super) fn recommend_content_selector(
    document: &Html,
    element: &ElementRef<'_>,
    exact_path: &str,
) -> String {
    let target_node_id = element.id();
    let candidates = selector_candidates_for_element(element, exact_path);

    for candidate in candidates {
        if selector_uniquely_matches(document, &candidate, target_node_id) {
            return candidate;
        }
    }

    exact_path.to_owned()
}

pub(super) fn selector_candidates_for_element(
    element: &ElementRef<'_>,
    _exact_path: &str,
) -> Vec<String> {
    let tag_name = element.value().name();
    let mut candidates = Vec::new();

    if let Some(id) = element
        .value()
        .attr("id")
        .map(str::trim)
        .filter(|id| !id.is_empty())
    {
        candidates.push(id_selector(id));
    }

    if let Some(role) = element
        .value()
        .attr("role")
        .map(str::trim)
        .filter(|role| !role.is_empty())
    {
        let escaped = css_string_literal(role);
        candidates.push(format!("{tag_name}[role=\"{escaped}\"]"));
        candidates.push(format!("[role=\"{escaped}\"]"));
    }

    if let Some(itemprop) = element
        .value()
        .attr("itemprop")
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let escaped = css_string_literal(itemprop);
        candidates.push(format!("{tag_name}[itemprop=\"{escaped}\"]"));
    }

    let selector_classes = selector_classes(element);
    if !selector_classes.is_empty() {
        let class_suffix = selector_classes
            .iter()
            .map(|class_name| format!(".{class_name}"))
            .collect::<String>();
        candidates.push(format!("{tag_name}{class_suffix}"));
        candidates.push(class_suffix);
    }

    let mut seen = BTreeSet::new();
    candidates.push(tag_name.to_owned());
    candidates.retain(|candidate| seen.insert(candidate.clone()));
    candidates
}

pub(super) fn selector_classes(element: &ElementRef<'_>) -> Vec<String> {
    let valid_classes = element
        .value()
        .attr("class")
        .into_iter()
        .flat_map(|value| value.split_whitespace())
        .filter(|class_name| simple_css_identifier(class_name))
        .map(str::to_owned)
        .collect::<Vec<_>>();

    let mut specific_classes = valid_classes
        .iter()
        .filter(|class_name| !GENERIC_SELECTOR_CLASSES.contains(&class_name.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    if specific_classes.is_empty() {
        specific_classes = valid_classes;
    }
    specific_classes.truncate(2);
    specific_classes
}

pub(super) fn selector_uniquely_matches(
    document: &Html,
    selector: &str,
    target_node_id: ego_tree::NodeId,
) -> bool {
    let Ok(selector) = Selector::parse(selector) else {
        return false;
    };
    let mut matches = document.select(&selector);
    let Some(first_match) = matches.next() else {
        return false;
    };
    first_match.id() == target_node_id && matches.next().is_none()
}

pub(super) fn id_selector(id: &str) -> String {
    if simple_css_identifier(id) {
        format!("#{id}")
    } else {
        format!("[id=\"{}\"]", css_string_literal(id))
    }
}

pub(super) fn simple_css_identifier(value: &str) -> bool {
    let mut characters = value.chars();
    let Some(first) = characters.next() else {
        return false;
    };
    if !matches!(first, 'A'..='Z' | 'a'..='z' | '_' | '-') {
        return false;
    }

    characters.all(|character| matches!(character, 'A'..='Z' | 'a'..='z' | '0'..='9' | '_' | '-'))
}

pub(super) fn css_string_literal(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}
