use super::*;

#[test]
fn rendered_text_filters_document_chrome_and_preserves_content() {
    assert_eq!(
        render_html_as_text(
            "<article><p>Ratio <span style=\"display:none\"><math><mfrac><mn>3</mn><mn>2</mn></mfrac></math></span><img aria-hidden=\"true\" alt=\"{\\\\textstyle {\\\\frac {3}{2}}}\" src=\"math.svg\"></p></article>",
            WhitespaceMode::Rendered,
        ),
        "Ratio 3/2"
    );
    assert_eq!(
        render_html_as_text(
            "<main>\
                <nav class=\"page-tools\"><a href=\"/edit\">Edit</a></nav>\
                <div class=\"live-story-filter-tags\"><button>All</button><button>catch up</button></div>\
                <div class=\"live-story__post-count\"><span>7 Posts</span></div>\
                <div class=\"social-share_compact\">\
                    <a class=\"social-share_compact__share\" href=\"mailto:?subject=Hello&amp;body=World\"><svg></svg></a>\
                    <div class=\"social-share_compact__copied\">Link Copied!</div>\
                </div>\
                <div class=\"featured-video\" data-video-id=\"123\"><div class=\"video-player\"><a href=\"https://example.com/video\">Watch</a></div><div class=\"caption\"><h4>Video title</h4><p>Video caption.</p></div></div>\
                <div class=\"mw-editsection-bracket\">[</div><div class=\"mw-editsection-bracket\">]</div>\
                <article><h2>Story</h2><p>Body <a href=\"https://example.com/guide\">Guide</a></p></article>\
                <section class=\"related-topics\"><h3>Related Topics</h3><a href=\"/other\">Other</a></section>\
                <footer><h3>More from here</h3><a href=\"/other\">Other</a></footer>\
             </main>",
            WhitespaceMode::Rendered,
        ),
        "## Story\n\nBody Guide [https://example.com/guide]"
    );
    assert_eq!(
        render_html_as_text(
            "<article>\
                <header class=\"article-header\">\
                    <div class=\"eyebrow\"><a href=\"/category\">Updates</a></div>\
                    <h1>Primary Title</h1>\
                    <div class=\"author-byline\">By Reporter</div>\
                </header>\
                <div class=\"notice\"><span class=\"flag\">NEW</span> playback available</div>\
                <p>Body paragraph.</p>\
                <p><a href=\"/background\"><strong>BACKGROUND READING FOR THIS TOPIC</strong></a></p>\
                <div class=\"author-bio\"><p>Reporter bio.</p></div>\
                <div class=\"catlinks\"><a href=\"/category\">Category</a></div>\
            </article>",
            WhitespaceMode::Rendered,
        ),
        "# Primary Title\n\nBody paragraph."
    );
    assert_eq!(
        render_html_as_text(
            "<article><span>When you purchase through links on our site, we may earn an affiliate commission. <a href=\"/terms\">Here’s how it works</a>.</span><h1>Story</h1><p>Body paragraph.</p></article>",
            WhitespaceMode::Rendered,
        ),
        "# Story\n\nBody paragraph."
    );
    assert_eq!(
        render_html_as_text(
            "<article><p><a href=\"/promo\"><strong>READ THE FULL TRANSCRIPT HERE</strong></a></p><p>Body paragraph.</p></article>",
            WhitespaceMode::Rendered,
        ),
        "Body paragraph."
    );
    assert_eq!(
        render_html_as_text(
            "<article><div class=\"hatnote\">For other uses, see <a href=\"/wiki/Math_(disambiguation)\">Math (disambiguation)</a>.</div><p>Body paragraph.</p></article>",
            WhitespaceMode::Rendered,
        ),
        "Body paragraph."
    );
    assert_eq!(
        render_html_as_text(
            "<article><pre><a href=\"https://example.com/guide\">\nhttps://example.com/guide\n</a></pre></article>",
            WhitespaceMode::Rendered,
        ),
        "https://example.com/guide"
    );
    assert!(
        render_html_as_text("<article><h2>   </h2></article>", WhitespaceMode::Rendered).is_empty()
    );
    assert_eq!(
        render_html_as_text(
            "<article><h2>Heading <span class=\"mw-editsection-bracket\">[</span><span class=\"mw-editsection-bracket\">]</span></h2></article>",
            WhitespaceMode::Rendered,
        ),
        "## Heading"
    );
    assert_eq!(
        render_html_as_text(
            "<article><h2 class=\"editable-heading\">Heading</h2><p>Body.</p></article>",
            WhitespaceMode::Rendered,
        ),
        "## Heading\n\nBody."
    );
    assert_eq!(
        render_html_as_text(
            "<article><h2 data-editable=\"headline\">Heading</h2><p>Body.</p></article>",
            WhitespaceMode::Rendered,
        ),
        "## Heading\n\nBody."
    );
    assert_eq!(
        render_html_as_text(
            "<article><h1><div><div>Host Liability Insurance Program Summary</div></div></h1><div class=\"notice\"><span class=\"flag\">NEW</span> playback available</div><p>Body paragraph.</p></article>",
            WhitespaceMode::Rendered,
        ),
        "# Host Liability Insurance Program Summary\n\nBody paragraph."
    );
    assert_eq!(
        render_html_as_text(
            "<article><div><div><p>Release Date:</p></div><div><p><b>4/14/2026</b></p></div></div><div><div><p>Version:</p></div><div><p><b>OS Build 20348.5020</b></p></div></div></article>",
            WhitespaceMode::Rendered,
        ),
        "Release Date: 4/14/2026\n\nVersion: OS Build 20348.5020"
    );
    assert_eq!(
        render_html_as_text(
            "<article><table><tr><td><p><b>Change date</b></p></td><td><p><b>Change description</b></p></td></tr><tr><td><p>May 1, 2026</p></td><td><ul><li><p>Improvement added: <b>[Vulnerable driver blocklist]</b></p></li></ul></td></tr><tr><td><p>April 27, 2026</p></td><td><p>Corrected the known issue.</p></td></tr></table></article>",
            WhitespaceMode::Rendered,
        ),
        "Change date    | Change description\nMay 1, 2026    | - Improvement added: [Vulnerable driver blocklist]\nApril 27, 2026 | Corrected the known issue."
    );
    assert_eq!(
        render_html_as_text(
            "<article><table><caption>Windows builds</caption><tr><th>Date</th><th>Build</th></tr><tr><td>April 14, 2026</td><td>20348.5020</td></tr></table></article>",
            WhitespaceMode::Rendered,
        ),
        "Windows builds\nDate           | Build\nApril 14, 2026 | 20348.5020"
    );
    assert_eq!(
        render_html_as_text(
            "<article><h3><button type=\"button\"><div aria-hidden=\"true\">Chevron</div><div>Windows Secure Boot certificate expiration</div></button></h3><p>Body.</p></article>",
            WhitespaceMode::Rendered,
        ),
        "### Windows Secure Boot certificate expiration\n\nBody."
    );
    assert_eq!(
        render_html_as_text(
            "<article><h3><button type=\"button\"><div>Windows Secure Boot certificate expiration</div></button></h3><p><b>Windows Secure Boot certificate expiration</b></p><p>Body.</p></article>",
            WhitespaceMode::Rendered,
        ),
        "### Windows Secure Boot certificate expiration\n\nBody."
    );
    assert_eq!(
        render_html_as_text(
            "<article><div class=\"image-ct inline\"><div class=\"m\"><img alt=\"Rudy Giuliani attending ceremony\" src=\"hero.jpg\"></div><div class=\"info\"><div class=\"caption\"><p>Photo caption.</p></div></div></div><p>Body.</p></article>",
            WhitespaceMode::Rendered,
        ),
        "Body."
    );
    assert_eq!(
        render_html_as_text(
            "<article><div class=\"side-box plainlinks\"><div class=\"side-box-image\"><a href=\"https://example.com/file\"><img alt=\"Wiktionary logo\" src=\"logo.png\"></a></div><div class=\"side-box-text\">Look up <a href=\"https://example.com/help\">help</a> in the dictionary.</div></div><p>Body.</p></article>",
            WhitespaceMode::Rendered,
        ),
        "Body."
    );
    assert_eq!(
        render_html_as_text(
            "<main><h1>Help</h1><p>Body.</p><div class=\"printfooter\" data-nosnippet=\"\">Retrieved from <a href=\"https://example.com/oldid\">old revision</a></div></main>",
            WhitespaceMode::Rendered,
        ),
        "# Help\n\nBody."
    );
    assert_eq!(
        render_html_as_text(
            "<main id=\"main\"><section class=\"section section-design section-product-story\"><h2>Design</h2><p>All-screen front.</p></section><section id=\"accessories\" class=\"section section-accessories section-product-story\"><h2>Accessories</h2><p><a href=\"/shop\">Shop all iPhone accessories</a></p></section><section class=\"section section-faq\"><h2>Questions? Answers.</h2><p>FAQ body.</p></section><section class=\"section section-upgrade\"><h2>Upgrade</h2><p><a href=\"/trade\">Find your trade-in value</a></p></section></main>",
            WhitespaceMode::Rendered,
        ),
        "## Design\n\nAll-screen front."
    );
    assert_eq!(
        render_html_as_text(
            "<main><h1>April 14, 2026—KB5082142</h1><p>Body.</p><div class=\"ocArticleFooterSection articleFooterBridge\"><h3>Need more help?</h3><a href=\"https://example.com/support\">Contact support</a></div></main>",
            WhitespaceMode::Rendered,
        ),
        "# April 14, 2026—KB5082142\n\nBody."
    );
    assert_eq!(
        render_html_as_text(
            "<article><ol start=\"5\"><li>Five</li><li>Six</li></ol></article>",
            WhitespaceMode::Rendered,
        ),
        "5. Five\n6. Six"
    );
    assert_eq!(
        render_html_as_text(
            "<article><ol reversed><li>Two</li><li>One</li></ol></article>",
            WhitespaceMode::Rendered,
        ),
        "2. Two\n1. One"
    );
    assert_eq!(
        render_html_as_text(
            "<article><ol><li value=\"7\">Seven</li><li>Eight</li></ol></article>",
            WhitespaceMode::Rendered,
        ),
        "7. Seven\n8. Eight"
    );
    let pre_image_rendered = render_html_as_text(
        "<pre><img src=\"hero.png\" alt=\"  Hero  \"></pre>",
        WhitespaceMode::Rendered,
    );
    assert_eq!(pre_image_rendered, "Hero");
    let empty_alt_rendered = render_html_as_text(
        "<p><img src=\"hero.png\" alt=\"   \"></p>",
        WhitespaceMode::Rendered,
    );
    assert!(empty_alt_rendered.is_empty());
}
