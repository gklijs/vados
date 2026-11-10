//! Static Markdown alternates. Generated sections share the HTML renderer;
//! authored Markdown is inserted unchanged apart from URL resolution.
use pulldown_cmark::{Event, LinkType, Parser, Tag};
use scraper::{ElementRef, Html, Selector};
use url::Url;

pub(crate) fn href(path: &str) -> String {
    if path == "/" {
        "/index.md".into()
    } else {
        format!("{path}.md")
    }
}

fn destination(base: &Url, target: &str) -> String {
    // Fully qualified and protocol-relative URLs keep their spelling.
    if target.starts_with("//") || Url::parse(target).is_ok() {
        return target.into();
    }
    base.join(target)
        .map(|url| {
            if url.origin() == base.origin() {
                format!(
                    "{}{}{}",
                    url.path(),
                    url.query().map(|q| format!("?{q}")).unwrap_or_default(),
                    url.fragment().map(|f| format!("#{f}")).unwrap_or_default()
                )
            } else {
                url.to_string()
            }
        })
        .unwrap_or_else(|_| target.into())
}

fn escape(text: &str) -> String {
    text.chars()
        .flat_map(|c| {
            if "\\`*_[]<>#".contains(c) {
                vec!['\\', c]
            } else {
                vec![c]
            }
        })
        .collect()
}

fn children(element: ElementRef<'_>, base: &Url) -> String {
    element
        .children()
        .map(|node| {
            if let Some(text) = node.value().as_text() {
                // HTML collapses whitespace, but keep spaces separating inline nodes.
                let mut result = escape(&text.split_whitespace().collect::<Vec<_>>().join(" "));
                if text.starts_with(char::is_whitespace) {
                    result.insert(0, ' ');
                }
                if text.ends_with(char::is_whitespace) {
                    result.push(' ');
                }
                result
            } else if let Some(element) = ElementRef::wrap(node) {
                convert(element, base)
            } else {
                String::new()
            }
        })
        .collect()
}

fn convert(element: ElementRef<'_>, base: &Url) -> String {
    let tag = element.value().name();
    if element.value().attr("hidden").is_some()
        || matches!(
            tag,
            "script" | "style" | "svg" | "button" | "form" | "nav" | "noscript"
        )
    {
        return String::new();
    }
    if tag == "pre" || tag == "code" {
        let text = element.text().collect::<String>();
        let fence = "`".repeat(
            text.split(|c| c != '`')
                .map(str::len)
                .max()
                .unwrap_or(0)
                .max(if tag == "pre" { 2 } else { 0 })
                + 1,
        );
        return if tag == "pre" {
            let code = element.select(&Selector::parse("code").unwrap()).next();
            let language = code
                .and_then(|e| {
                    e.value()
                        .classes()
                        .find_map(|c| c.strip_prefix("language-"))
                })
                .unwrap_or("");
            format!(
                "\n\n{fence}{language}\n{}\n{fence}\n\n",
                text.trim_end_matches('\n')
            )
        } else {
            format!("{fence} {text} {fence}")
        };
    }
    let body = children(element, base);
    match tag {
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => format!(
            "\n\n{} {}\n\n",
            "#".repeat(tag[1..].parse().unwrap()),
            body.trim()
        ),
        "p" | "div" | "section" | "article" | "dl" => format!("\n\n{}\n\n", body.trim()),
        "br" => "  \n".into(),
        "hr" => "\n\n---\n\n".into(),
        "strong" | "b" => format!("**{}**", body.trim()),
        "em" | "i" => format!("*{}*", body.trim()),
        "a" => {
            if let Some(url) = element
                .value()
                .attr("href")
                .filter(|_| !body.trim().is_empty())
            {
                let url = destination(base, url);
                // Tiles wrap headings and paragraphs in one HTML anchor. Put
                // the link on the heading, keeping the remaining blocks outside.
                if body.contains('\n') {
                    let trimmed = body.trim();
                    let (first, rest) = trimmed.split_once('\n').unwrap_or((trimmed, ""));
                    let heading = first.len() - first.trim_start_matches('#').len();
                    let label = first[heading..].trim();
                    format!(
                        "\n\n{}[{}](<{}>)\n{}\n\n",
                        if heading > 0 {
                            format!("{} ", "#".repeat(heading))
                        } else {
                            String::new()
                        },
                        label,
                        url,
                        rest
                    )
                } else {
                    format!("[{}](<{}>)", body.trim(), url)
                }
            } else {
                body
            }
        }
        "time" => element
            .value()
            .attr("datetime")
            .map(|date| format!("{} ({})", body.trim(), escape(date)))
            .unwrap_or(body),
        "img" => element
            .value()
            .attr("src")
            .map(|url| {
                format!(
                    "![{}](<{}>)",
                    escape(element.value().attr("alt").unwrap_or("")),
                    destination(base, url)
                )
            })
            .unwrap_or_default(),
        "iframe" => element
            .value()
            .attr("src")
            .map(|url| {
                format!(
                    "\n\n[{}](<{}>)\n\n",
                    escape(element.value().attr("title").unwrap_or("Embedded content")),
                    destination(base, url)
                )
            })
            .unwrap_or_default(),
        "ul" | "ol" => {
            let start = element
                .value()
                .attr("start")
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(1);
            let entries: Vec<_> = element
                .child_elements()
                .filter(|e| e.value().name() == "li")
                .enumerate()
                .map(|(i, e)| {
                    let marker = if tag == "ol" {
                        format!("{}. ", start + i)
                    } else {
                        "- ".into()
                    };
                    let content = children(e, base);
                    format!(
                        "{}{}",
                        marker,
                        content
                            .trim()
                            .replace('\n', &format!("\n{}", " ".repeat(marker.len())))
                    )
                })
                .collect();
            format!("\n\n{}\n\n", entries.join("\n"))
        }
        "blockquote" => format!(
            "\n\n{}\n\n",
            body.trim()
                .lines()
                .map(|l| format!("> {l}"))
                .collect::<Vec<_>>()
                .join("\n")
        ),
        // Tables retain their cell boundaries without requiring a particular
        // Markdown table extension, including when cells contain paragraphs.
        "tr" => format!("\n\n{}\n\n", body.trim()),
        "td" | "th" => format!("{} | ", body.trim()),
        "dt" => format!("\n\n**{}**: ", body.trim()),
        "dd" => format!("{}\n\n", body.trim()),
        _ => body,
    }
}

/// Change only destination spans, leaving the author's formatting and code alone.
fn authored(text: &str, base: &Url) -> String {
    let parser = Parser::new(text);
    let mut changes = Vec::new();
    for (_, definition) in parser.reference_definitions().iter() {
        let span = definition.span.clone();
        if let Some(colon) = text[span.clone()].find("]:") {
            replace_destination(
                text,
                span.start + colon + 2..span.end,
                &definition.dest,
                base,
                &mut changes,
            );
        }
    }
    for (event, span) in parser.into_offset_iter() {
        match event {
            Event::Start(
                Tag::Link {
                    dest_url,
                    link_type: LinkType::Inline,
                    ..
                }
                | Tag::Image {
                    dest_url,
                    link_type: LinkType::Inline,
                    ..
                },
            ) => {
                if let Some(begin) = link_destination_start(&text[span.clone()]) {
                    replace_destination(
                        text,
                        span.start + begin + 2..span.end,
                        &dest_url,
                        base,
                        &mut changes,
                    );
                }
            }
            Event::Html(html) => {
                let fragment = Html::parse_fragment(&html);
                changes.push((span, children(fragment.root_element(), base)));
            }
            Event::InlineHtml(html) => {
                let normalized = html_attributes(&html, base);
                if normalized != html.as_ref() {
                    changes.push((span, normalized));
                }
            }
            _ => {}
        }
    }
    changes.sort_by_key(|(span, _)| span.start);
    let mut result = text.to_string();
    for (span, replacement) in changes.into_iter().rev() {
        result.replace_range(span, &replacement);
    }
    result
}

/// Resolve URL attributes without balancing an inline opening/closing tag:
/// the rest of its contents belongs to subsequent Markdown parser events.
fn html_attributes(html: &str, base: &Url) -> String {
    let fragment = Html::parse_fragment(html);
    let mut result = html.to_string();
    let Some(element) = fragment.root_element().child_elements().next() else {
        return result;
    };
    let bytes = html.as_bytes();
    let mut i = html.find(char::is_whitespace).unwrap_or(html.len());
    let mut changes = Vec::new();
    while i < bytes.len() {
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        let name_start = i;
        while i < bytes.len() && !bytes[i].is_ascii_whitespace() && !b"=>/".contains(&bytes[i]) {
            i += 1;
        }
        if i == name_start {
            break;
        }
        let name = html[name_start..i].to_ascii_lowercase();
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if bytes.get(i) != Some(&b'=') {
            continue;
        }
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        let quote = bytes.get(i).copied().filter(|b| b"\"'".contains(b));
        if quote.is_some() {
            i += 1;
        }
        let start = i;
        while i < bytes.len()
            && match quote {
                Some(q) => bytes[i] != q,
                None => !bytes[i].is_ascii_whitespace() && bytes[i] != b'>',
            }
        {
            i += 1;
        }
        if matches!(name.as_str(), "href" | "src") {
            if let Some(value) = element.value().attr(name.as_str()) {
                let value = destination(base, value)
                    .replace('&', "&amp;")
                    .replace('\"', "&quot;")
                    .replace('\'', "&#39;");
                changes.push((start..i, value));
            }
        }
        if quote.is_some() {
            i += 1;
        }
    }
    for (range, value) in changes.into_iter().rev() {
        result.replace_range(range, &value);
    }
    result
}

// Find the closing bracket of the outer label, skipping escaped characters
// and code spans, rather than mistaking brackets in a label/title for it.
fn link_destination_start(source: &str) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut depth = 0usize;
    let mut ticks = 0usize;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' if ticks == 0 => {
                i += 2;
                continue;
            }
            b'`' => {
                let count = bytes[i..].iter().take_while(|&&b| b == b'`').count();
                if ticks == 0 {
                    ticks = count;
                } else if ticks == count {
                    ticks = 0;
                }
                i += count;
                continue;
            }
            b'[' if ticks == 0 => depth += 1,
            b']' if ticks == 0 => {
                depth = depth.saturating_sub(1);
                if depth == 0 && bytes.get(i + 1) == Some(&b'(') {
                    return Some(i);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

fn replace_destination(
    text: &str,
    span: std::ops::Range<usize>,
    target: &str,
    base: &Url,
    changes: &mut Vec<(std::ops::Range<usize>, String)>,
) {
    let raw = &text[span.clone()];
    let start = raw.len() - raw.trim_start().len();
    let start = start + usize::from(raw[start..].starts_with('<'));
    // Use the source destination, including escapes, for the replacement range.
    let end = if raw[..start].ends_with('<') {
        raw[start..].find('>').map(|n| start + n)
    } else {
        let mut depth = 0usize;
        let mut escaped = false;
        let mut end = raw.len();
        for (i, c) in raw[start..].char_indices() {
            if escaped {
                escaped = false;
                continue;
            }
            if c == '\\' {
                escaped = true;
                continue;
            }
            if (c == ')' && depth == 0) || c.is_whitespace() {
                end = start + i;
                break;
            }
            if c == '(' {
                depth += 1;
            }
            if c == ')' {
                depth = depth.saturating_sub(1);
            }
        }
        Some(end)
    };
    if let Some(end) = end {
        changes.push((
            span.start + start..span.start + end,
            destination(base, target)
                .replace('(', "%28")
                .replace(')', "%29"),
        ));
    }
}

pub(crate) fn render(
    page: &str,
    path: &str,
    site_url: Option<&str>,
    original: Option<(&str, &str)>,
    updated: Option<&str>,
) -> String {
    let canonical = crate::feed::page_url(site_url.unwrap_or(""), path);
    let base = Url::parse(&canonical)
        .or_else(|_| Url::parse(&crate::feed::page_url("https://vados.invalid", path)))
        .unwrap();
    let document = Html::parse_document(page);
    let main = document
        .select(&Selector::parse("#main-content").unwrap())
        .next()
        .unwrap();
    let mut markdown = children(main, &base);
    // Child/sibling cards are content even when the layout puts them beside
    // the article. The side menu and breadcrumbs are navigation and omitted.
    for card in document.select(&Selector::parse("#side-column > .notification").unwrap()) {
        markdown.push_str(&convert(card, &base));
    }
    // Normalize layout whitespace without changing code, hard line breaks,
    // or the authored Markdown inserted afterwards.
    markdown = normalize_generated(&markdown);
    if let Some((marker, text)) = original {
        markdown = markdown.replacen(marker, &authored(text, &base), 1);
    }
    let updated = updated
        .map(|date| format!("Updated: {date}\n\n"))
        .unwrap_or_default();
    format!("Canonical: {canonical}\n\n{updated}{}\n", markdown.trim())
}

fn normalize_generated(text: &str) -> String {
    fn prose(text: &str) -> String {
        let mut result = text
            .lines()
            .map(|line| if line.trim().is_empty() { "" } else { line })
            .collect::<Vec<_>>()
            .join("\n");
        if text.ends_with('\n') {
            result.push('\n');
        }
        while result.contains("\n\n\n") {
            result = result.replace("\n\n\n", "\n\n");
        }
        result
    }
    let mut result = String::new();
    let mut end = 0;
    for (event, span) in Parser::new(text).into_offset_iter() {
        if matches!(event, Event::Start(Tag::CodeBlock(_))) {
            result.push_str(&prose(&text[end..span.start]));
            result.push_str(&text[span.clone()]);
            end = span.end;
        }
    }
    result.push_str(&prose(&text[end..]));
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_cleanup_preserves_code_whitespace_and_hard_breaks() {
        let code = "```rust\nlet x = 1;  \n\n\nlet y = 2;\n```\n";
        let text = format!("\n\n\n# Heading\n\n\n{code}\n\nFirst  \nSecond\n");
        let normalized = normalize_generated(&text);
        assert!(normalized.contains(code), "{normalized:?}");
        assert!(normalized.contains("First  \nSecond"), "{normalized:?}");
        assert!(!normalized.starts_with("\n\n\n"));
    }

    #[test]
    fn links_keep_their_targets_through_nested_labels_titles_and_reference_definitions() {
        let base = Url::parse("https://vados.invalid/blog/post").unwrap();
        let source = concat!(
            "[a](../target \"title ](other\")\n\n",
            "[![image](img.png)](target)\n\n",
            "[`](`](#part)\n\n",
            "[escaped](foo\\)bar)\n\n",
            "[ref]\n\n[ref]:\n  <../reference> 'title'\n\n",
            "[external](https://example.com/a)\n\n",
            "    [code](unchanged)\n",
        );
        let result = authored(source, &base);
        for expected in [
            "[a](/target \"title ](other\")",
            "[![image](/blog/img.png)](/blog/target)",
            "[`](`](/blog/post#part)",
            "[escaped](/blog/foo%29bar)",
            "[ref]:\n  </reference> 'title'",
            "[external](https://example.com/a)",
            "    [code](unchanged)",
        ] {
            assert!(result.contains(expected), "missing {expected:?}: {result}");
        }
        // Both the image destination and the outer anchor remain parseable.
        let targets: Vec<_> = Parser::new(&result)
            .filter_map(|event| match event {
                Event::Start(Tag::Link { dest_url, .. } | Tag::Image { dest_url, .. }) => {
                    Some(dest_url.to_string())
                }
                _ => None,
            })
            .collect();
        assert!(targets.contains(&"/blog/img.png".to_string()));
        assert!(targets.contains(&"/blog/target".to_string()));
    }
}
