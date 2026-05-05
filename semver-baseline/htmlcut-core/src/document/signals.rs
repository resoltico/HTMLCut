use scraper::{ElementRef, Node};

const CONTENT_HINT_TOKENS: [&str; 8] = [
    "article", "body", "content", "entry", "guide", "help", "main", "text",
];
const UTILITY_CHROME_TOKENS: [&str; 63] = [
    "ad",
    "advert",
    "appearance",
    "author",
    "audio",
    "bio",
    "breadcrumb",
    "byline",
    "categories",
    "category",
    "caption",
    "catlinks",
    "comment",
    "comments",
    "copied",
    "copy",
    "control",
    "controls",
    "count",
    "dropdown",
    "eyebrow",
    "edit",
    "editsection",
    "featured",
    "export",
    "filter",
    "filters",
    "hidden",
    "indicator",
    "indicators",
    "interface",
    "kicker",
    "lang",
    "language",
    "meta",
    "metadata",
    "media",
    "menu",
    "nav",
    "newsletter",
    "noprint",
    "pending",
    "pagination",
    "player",
    "portlet",
    "print",
    "reaction",
    "reactions",
    "recommend",
    "recommended",
    "related",
    "revision",
    "share",
    "sidebar",
    "social",
    "status",
    "subtitle",
    "subscribe",
    "feedback",
    "topic",
    "topics",
    "video",
    "widget",
];
const UI_ROLES: [&str; 8] = [
    "complementary",
    "dialog",
    "menu",
    "menubar",
    "navigation",
    "search",
    "tablist",
    "toolbar",
];

pub(crate) fn structural_signal_tokens(element: &ElementRef<'_>) -> Vec<String> {
    let mut raw_values = vec![element.value().name().to_owned()];
    for (attribute_name, value) in element.value().attrs() {
        if matches!(
            attribute_name,
            "id" | "class" | "role" | "itemprop" | "title" | "name"
        ) || attribute_name.starts_with("aria-")
        {
            raw_values.push(attribute_name.to_owned());
            raw_values.push(value.to_owned());
            continue;
        }

        if attribute_name.starts_with("data-") {
            raw_values.push(value.to_owned());
        }
    }

    raw_values
        .into_iter()
        .flat_map(|value| tokenize_structural_signal(&value))
        .collect()
}

pub(crate) fn token_match_count(tokens: &[String], vocabulary: &[&str]) -> usize {
    tokens
        .iter()
        .filter(|token| {
            vocabulary.iter().any(|candidate| {
                token.as_str() == *candidate
                    || (candidate.len() >= 5
                        && token.len() > candidate.len() + 2
                        && token.contains(candidate))
            })
        })
        .count()
}

pub(crate) fn element_looks_like_utility_chrome(element: &ElementRef<'_>) -> bool {
    let tag_name = element.value().name();
    if matches!(
        tag_name,
        "button" | "footer" | "input" | "nav" | "option" | "select" | "textarea"
    ) {
        return true;
    }

    if element.value().attr("hidden").is_some()
        || element
            .value()
            .attr("aria-hidden")
            .is_some_and(|value| value.eq_ignore_ascii_case("true"))
    {
        return true;
    }

    if element.value().attr("role").is_some_and(|role| {
        UI_ROLES
            .iter()
            .any(|candidate| role.eq_ignore_ascii_case(candidate))
    }) {
        return true;
    }

    let tokens = structural_signal_tokens(element);
    let content_count = token_match_count(&tokens, &CONTENT_HINT_TOKENS);
    if element_looks_like_compact_utility_widget(element, content_count) {
        return true;
    }

    if !matches!(tag_name, "article" | "main") && token_match_count(&tokens, &["footer"]) > 0 {
        return true;
    }

    let utility_count = token_match_count(&tokens, &UTILITY_CHROME_TOKENS);
    if utility_count == 0 {
        return false;
    }

    utility_count > content_count && !matches!(tag_name, "article" | "main")
}

pub(crate) fn element_has_utility_chrome_ancestor(element: &ElementRef<'_>) -> bool {
    let mut parent = element.parent();
    while let Some(current) = parent {
        if let Some(parent_element) = ElementRef::wrap(current)
            && element_looks_like_utility_chrome(&parent_element)
        {
            return true;
        }
        parent = current.parent();
    }

    false
}

fn tokenize_structural_signal(value: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();

    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            current.push(character.to_ascii_lowercase());
        } else if !current.is_empty() {
            tokens.push(std::mem::take(&mut current));
        }
    }

    if current.is_empty() {
        return tokens;
    }

    tokens.push(current);

    tokens
}

fn element_looks_like_compact_utility_widget(
    element: &ElementRef<'_>,
    content_count: usize,
) -> bool {
    if content_count > 0
        || !matches!(element.value().name(), "aside" | "div" | "section")
        || has_heading_ancestor(element)
    {
        return false;
    }

    let mut child_element_count = 0usize;
    let mut link_count = 0usize;
    let mut image_count = 0usize;
    let mut text_chars = 0usize;

    for descendant in element.descendants() {
        match descendant.value() {
            Node::Element(data) => {
                if descendant.id() != element.id() {
                    child_element_count += 1;
                    if matches!(
                        data.name(),
                        "blockquote"
                            | "dd"
                            | "dl"
                            | "dt"
                            | "figcaption"
                            | "figure"
                            | "h1"
                            | "h2"
                            | "h3"
                            | "h4"
                            | "h5"
                            | "h6"
                            | "li"
                            | "ol"
                            | "p"
                            | "picture"
                            | "pre"
                            | "table"
                            | "ul"
                            | "video"
                    ) {
                        return false;
                    }

                    if data.name() == "img" {
                        image_count += 1;
                        if image_count > 1 {
                            return false;
                        }
                    }
                }

                if data.name() == "a" {
                    link_count += 1;
                    if link_count > if image_count > 0 { 2 } else { 1 } {
                        return false;
                    }
                }
            }
            Node::Text(contents) => {
                text_chars += contents
                    .chars()
                    .filter(|character| !character.is_whitespace())
                    .count();
                if text_chars > 64 {
                    return false;
                }
            }
            _ => {}
        }
    }

    child_element_count > 0 && text_chars > 0
}

fn has_heading_ancestor(element: &ElementRef<'_>) -> bool {
    let mut parent = element.parent();
    while let Some(current) = parent {
        if let Some(parent_element) = ElementRef::wrap(current)
            && matches!(
                parent_element.value().name(),
                "h1" | "h2" | "h3" | "h4" | "h5" | "h6"
            )
        {
            return true;
        }
        parent = current.parent();
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{parse_document_node, select_first};

    #[test]
    fn utility_signal_helpers_cover_roles_tail_tokens_and_compact_widget_limits() {
        let toolbar = parse_document_node("<div role=\"toolbar\">Controls</div>");
        let toolbar_element = select_first(&toolbar, "div").expect("toolbar");
        assert!(element_looks_like_utility_chrome(&toolbar_element));

        let hidden = parse_document_node("<section hidden>Hidden</section>");
        let hidden_element = select_first(&hidden, "section").expect("hidden");
        assert!(element_looks_like_utility_chrome(&hidden_element));
        let aria_hidden = parse_document_node("<section aria-hidden=\"true\">Hidden</section>");
        let aria_hidden_element = select_first(&aria_hidden, "section").expect("aria hidden");
        assert!(element_looks_like_utility_chrome(&aria_hidden_element));

        let tokenized = parse_document_node(
            "<div id=\"supportCenter42\" class=\"utility-pane\" data-track=\"LiveFeed\"></div>",
        );
        let tokenized_element = select_first(&tokenized, "div").expect("tokenized");
        assert!(structural_signal_tokens(&tokenized_element).contains(&"livefeed".to_owned()));
        assert_eq!(
            token_match_count(&structural_signal_tokens(&tokenized_element), &["support"]),
            1
        );
        assert_eq!(
            tokenize_structural_signal("LiveFeed42"),
            vec!["livefeed42".to_owned()]
        );
        assert!(tokenize_structural_signal("!!!").is_empty());

        let footer_wrapper =
            parse_document_node("<div class=\"printfooter articleFooterSection\">Help links</div>");
        let footer_wrapper_element = select_first(&footer_wrapper, "div").expect("footer");
        assert!(element_looks_like_utility_chrome(&footer_wrapper_element));
        let article_footer = parse_document_node(
            "<article class=\"footer\">Meaningful article footer text</article>",
        );
        let article_footer_element = select_first(&article_footer, "article").expect("article");
        assert!(!element_looks_like_utility_chrome(&article_footer_element));
        let main_footer =
            parse_document_node("<main class=\"footer\">Meaningful main footer text</main>");
        let main_footer_element = select_first(&main_footer, "main").expect("main");
        assert!(!element_looks_like_utility_chrome(&main_footer_element));

        let balanced_content =
            parse_document_node("<div class=\"content caption\">Meaningful copy</div>");
        let balanced_content_element = select_first(&balanced_content, "div").expect("balanced");
        assert!(!element_looks_like_utility_chrome(
            &balanced_content_element
        ));
        let article_caption =
            parse_document_node("<article class=\"caption\">Meaningful copy</article>");
        let article_caption_element = select_first(&article_caption, "article").expect("article");
        assert!(!element_looks_like_utility_chrome(&article_caption_element));
        let main_caption = parse_document_node("<main class=\"caption\">Meaningful copy</main>");
        let main_caption_element = select_first(&main_caption, "main").expect("main");
        assert!(!element_looks_like_utility_chrome(&main_caption_element));

        let compact_with_comment = parse_document_node(
            "<div class=\"widget-box\"><a href=\"/help\">Help</a><!-- note -->Text</div>",
        );
        let compact = select_first(&compact_with_comment, "div").expect("compact widget");
        assert!(element_looks_like_utility_chrome(&compact));
        assert!(!element_looks_like_compact_utility_widget(&compact, 1));
        assert!(!element_looks_like_compact_utility_widget(&compact, 2));

        let two_images = parse_document_node(
            "<div><img alt=\"One\" src=\"one.png\"><img alt=\"Two\" src=\"two.png\"><a href=\"/more\">More</a>Text</div>",
        );
        let two_images_element = select_first(&two_images, "div").expect("two images");
        assert!(!element_looks_like_compact_utility_widget(
            &two_images_element,
            0
        ));

        let too_many_links = parse_document_node(
            "<div><img alt=\"One\" src=\"one.png\"><a href=\"/one\">One</a><a href=\"/two\">Two</a><a href=\"/three\">Three</a>Text</div>",
        );
        let too_many_links_element = select_first(&too_many_links, "div").expect("links");
        assert!(!element_looks_like_compact_utility_widget(
            &too_many_links_element,
            0
        ));

        let two_links_no_image =
            parse_document_node("<div><a href=\"/one\">One</a><a href=\"/two\">Two</a>Text</div>");
        let two_links_no_image_element = select_first(&two_links_no_image, "div").expect("links");
        assert!(!element_looks_like_compact_utility_widget(
            &two_links_no_image_element,
            0
        ));

        let empty_widget = parse_document_node("<div>   </div>");
        let empty_widget_element = select_first(&empty_widget, "div").expect("empty");
        assert!(!element_looks_like_compact_utility_widget(
            &empty_widget_element,
            0
        ));

        let long_text = parse_document_node(
            "<div><a href=\"/one\">One</a>This text is intentionally long enough to exceed the compact utility widget threshold and force the detector to reject it.</div>",
        );
        let long_text_element = select_first(&long_text, "div").expect("long text");
        assert!(!element_looks_like_compact_utility_widget(
            &long_text_element,
            0
        ));

        let inside_heading = parse_document_node(
            "<h2><div class=\"widget-box\"><a href=\"/one\">One</a>Text</div></h2>",
        );
        let inside_heading_element = select_first(&inside_heading, "div").expect("inside heading");
        assert!(!element_looks_like_compact_utility_widget(
            &inside_heading_element,
            0
        ));
        assert!(has_heading_ancestor(&inside_heading_element));

        let ancestor_document =
            parse_document_node("<nav class=\"tools\"><div><span>Alpha</span></div></nav>");
        let nested_div = select_first(&ancestor_document, "nav.tools div").expect("nested div");
        assert!(element_has_utility_chrome_ancestor(&nested_div));
        let standalone_document = parse_document_node("<main><div><span>Alpha</span></div></main>");
        let standalone_div = select_first(&standalone_document, "main div").expect("standalone");
        assert!(!element_has_utility_chrome_ancestor(&standalone_div));
    }
}
