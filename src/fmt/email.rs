use std::str::FromStr;

use pulldown_cmark::{
    CodeBlockKind,
    CowStr,
    Event,
    Tag,
    TagEnd,
    html,
};

use super::{
    build_github_url,
    html_escape,
    resolve_repo,
};
use crate::parse::{
    CodeBlockInfo,
    MarkdownParser,
};

pub fn to_plain_text(markdown_content: &str) -> String {
    let parser = MarkdownParser::new(markdown_content).unwrap();
    let front_matter = parser.front_matter();

    let mut output = String::new();
    let mut in_code_block = false;
    let mut in_heading = false;
    let mut heading_text = String::new();
    let mut list_depth: usize = 0;

    // Add front matter at the top if present
    if let Some(fm) = front_matter {
        if let Some(title) = &fm.title {
            output.push_str(title);
            output.push('\n');
            output.push_str(&"=".repeat(title.len()));
            output.push_str("\n\n");
        }
        if let Some(author) = &fm.author {
            output.push_str("By ");
            output.push_str(author);
            output.push('\n');
        }
        if let Some(date) = &fm.date {
            output.push_str("Date: ");
            output.push_str(date);
            output.push('\n');
        }
        if fm.author.is_some() || fm.date.is_some() {
            output.push('\n');
        }
    }

    for event in parser.into_inner() {
        match event {
            Event::Start(Tag::Heading { level: _, .. }) => {
                in_heading = true;
                heading_text.clear();
            }
            Event::End(TagEnd::Heading(level)) => {
                in_heading = false;
                output.push_str(&heading_text);
                output.push('\n');

                // Add underline for level 1 and 2 headings
                let underline_char = match level {
                    pulldown_cmark::HeadingLevel::H1 => '=',
                    pulldown_cmark::HeadingLevel::H2 => '-',
                    _ => {
                        output.push('\n');
                        continue;
                    }
                };
                output.push_str(&underline_char.to_string().repeat(heading_text.len()));
                output.push_str("\n\n");
            }
            Event::Start(Tag::Paragraph) => {}
            Event::End(TagEnd::Paragraph) => {
                output.push_str("\n\n");
            }
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info))) => {
                in_code_block = true;
                let code_info = CodeBlockInfo::from_str(&info).unwrap();
                if let Some(filename) = code_info.filename {
                    output.push_str(&filename);
                    output.push_str(":\n");
                }
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code_block = false;
                output.push_str("\n\n");
            }
            Event::Start(Tag::List(_)) => {
                list_depth += 1;
            }
            Event::End(TagEnd::List(_)) => {
                list_depth -= 1;
                if list_depth == 0 {
                    output.push('\n');
                }
            }
            Event::Start(Tag::Item) => {
                output.push_str(&"  ".repeat(list_depth.saturating_sub(1)));
                output.push_str("* ");
            }
            Event::End(TagEnd::Item) => {
                output.push('\n');
            }
            Event::Start(Tag::BlockQuote(_)) => {
                output.push_str("> ");
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                output.push_str("\n\n");
            }
            Event::Code(code) => {
                if in_heading {
                    heading_text.push_str(&code);
                } else {
                    output.push('`');
                    output.push_str(&code);
                    output.push('`');
                }
            }
            Event::Text(text) => {
                if in_heading {
                    heading_text.push_str(&text);
                } else {
                    output.push_str(&text);
                }
            }
            Event::SoftBreak => {
                if in_code_block {
                    output.push('\n');
                } else {
                    output.push(' ');
                }
            }
            Event::HardBreak => {
                output.push('\n');
            }
            Event::Rule => {
                output.push_str(&"-".repeat(70));
                output.push_str("\n\n");
            }
            _ => {}
        }
    }

    output
}

pub fn to_html(markdown_content: &str) -> String {
    let parser = MarkdownParser::new(markdown_content).unwrap();
    let front_matter = parser.front_matter().cloned();

    // Process events to handle special code blocks
    let mut events = Vec::new();
    let mut in_code_block = false;
    let mut code_block_info = None;
    let mut code_content = String::new();

    for event in parser.into_inner() {
        match event {
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(ref info))) => {
                in_code_block = true;
                code_block_info = CodeBlockInfo::from_str(info).ok();
                code_content.clear();
            }
            Event::End(TagEnd::CodeBlock) if in_code_block => {
                in_code_block = false;

                if let Some(ref info) = code_block_info {
                    let mut custom_html = String::new();

                    custom_html.push_str("<div style=\"margin: 16px 0;\">");

                    if let Some(filename) = info.filename.as_ref() {
                        let repo_to_use = resolve_repo(info.repo.as_ref(), front_matter.as_ref());

                        if let Some(repo) = repo_to_use {
                            let github_url = build_github_url(
                                filename,
                                info.start_line,
                                repo,
                                info.refspec.as_deref(),
                            );
                            custom_html.push_str(&format!(
                                "<div style=\"background-color: #e1e4e8; color: #24292e; padding: 8px 16px; font-family: 'Courier New', Courier, monospace; font-size: 14px; font-weight: 600; border-bottom: 1px solid #d0d7de;\"><a href=\"{}\" style=\"color: #24292e; text-decoration: none;\">{}</a></div>",
                                html_escape(&github_url),
                                html_escape(filename)
                            ));
                        } else {
                            custom_html.push_str(&format!(
                                "<div style=\"background-color: #e1e4e8; color: #24292e; padding: 8px 16px; font-family: 'Courier New', Courier, monospace; font-size: 14px; font-weight: 600; border-bottom: 1px solid #d0d7de;\">{}</div>",
                                html_escape(filename)
                            ));
                        }
                    }

                    custom_html.push_str("<pre style=\"background-color: #f6f8fa; padding: 16px; margin: 0; overflow-x: auto;\"><code");
                    if !info.language.is_empty() {
                        custom_html.push_str(" style=\"font-family: 'Courier New', Courier, monospace; font-size: 14px;\"");
                    }
                    custom_html.push('>');

                    if let Some(start_line) = info.start_line {
                        let lines: Vec<&str> = code_content.lines().collect();
                        for (idx, line) in lines.iter().enumerate() {
                            let line_num = start_line + idx;
                            custom_html.push_str(&format!(
                                "<span style=\"color: #8b949e; margin-right: 16px; user-select: none; display: inline-block; text-align: right; min-width: 48px;\">{:>4}</span>{}\n",
                                line_num,
                                html_escape(line)
                            ));
                        }
                    } else {
                        custom_html.push_str(&html_escape(&code_content));
                    }

                    custom_html.push_str("</code></pre></div>");

                    events.push(Event::Html(CowStr::Boxed(custom_html.into_boxed_str())));
                } else {
                    events.push(Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(
                        CowStr::Borrowed(""),
                    ))));
                    events.push(Event::Text(CowStr::Boxed(
                        code_content.clone().into_boxed_str(),
                    )));
                    events.push(Event::End(TagEnd::CodeBlock));
                }

                code_block_info = None;
            }
            Event::Text(ref text) if in_code_block => {
                code_content.push_str(text);
            }
            _ if !in_code_block => {
                events.push(event);
            }
            _ => {}
        }
    }

    let mut html_output = String::new();
    html::push_html(&mut html_output, events.into_iter());

    // Build metadata section if front matter exists
    let metadata_html = if let Some(fm) = front_matter {
        let mut meta = String::from(
            "<div style=\"margin-bottom: 48px; padding-bottom: 32px; border-bottom: 3px solid #eaecef;\">",
        );

        if let Some(doc_title) = &fm.title {
            meta.push_str(&format!("<h1 style=\"font-size: 40px; margin-bottom: 8px; margin-top: 0; border-bottom: none; font-weight: 600; line-height: 1.25;\">{}</h1>", doc_title));
        }

        if fm.author.is_some() || fm.date.is_some() {
            meta.push_str(
                "<div style=\"display: flex; gap: 32px; color: #666; font-size: 15px;\">",
            );
            if let Some(author) = &fm.author {
                meta.push_str(&format!("<span>By {}</span>", author));
            }
            if let Some(date) = &fm.date {
                meta.push_str(&format!("<span>Date: {}</span>", date));
            }
            meta.push_str("</div>");
        }

        meta.push_str("</div>");
        meta
    } else {
        String::new()
    };

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
</head>
<body style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, sans-serif; line-height: 1.6; max-width: 900px; margin: 0 auto; padding: 32px; color: #333; background-color: #fff;">
<div style="font-size: 16px;">
{}{}
</div>
</body>
</html>"#,
        metadata_html, html_output
    )
}
