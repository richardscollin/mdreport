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
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info))) => {
                in_code_block = true;
                code_block_info = Some(CodeBlockInfo::from_str(&info).unwrap());
                code_content.clear();
            }
            Event::End(TagEnd::CodeBlock) if in_code_block => {
                in_code_block = false;

                if let Some(info) = code_block_info {
                    // Generate custom HTML for code block with filename and line numbers
                    let mut custom_html = String::new();

                    custom_html.push_str("<div class=\"code-block-container\">");

                    if let Some(filename) = info.filename {
                        // Determine which repo to use: code block repo or frontmatter default
                        let repo_to_use = resolve_repo(info.repo.as_ref(), front_matter.as_ref());

                        if let Some(repo) = repo_to_use {
                            // Build GitHub URL and make filename clickable
                            let github_url = build_github_url(
                                &filename,
                                info.start_line,
                                repo,
                                info.refspec.as_deref(),
                            );
                            custom_html.push_str(&format!(
                                "<div class=\"code-filename\"><a href=\"{}\" target=\"_blank\">{}</a></div>",
                                html_escape(&github_url),
                                html_escape(&filename)
                            ));
                        } else {
                            // No repo info, just display filename as text
                            custom_html.push_str(&format!(
                                "<div class=\"code-filename\">{}</div>",
                                html_escape(&filename)
                            ));
                        }
                    }

                    custom_html.push_str("<pre><code");
                    if !info.language.is_empty() {
                        custom_html.push_str(&format!(" class=\"language-{}\"", info.language));
                    }
                    custom_html.push('>');

                    // Add line numbers if start_line is specified
                    if let Some(start_line) = info.start_line {
                        let lines: Vec<&str> = code_content.lines().collect();
                        for (idx, line) in lines.iter().enumerate() {
                            let line_num = start_line + idx;
                            custom_html.push_str(&format!(
                                "<span class=\"line-number\">{:>4}</span> {}\n",
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
                    // Shouldn't happen, but fallback
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
            _ => {
                if !in_code_block {
                    events.push(event);
                }
            }
        }
    }

    let mut html_output = String::new();
    html::push_html(&mut html_output, events.into_iter());

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, sans-serif;
            line-height: 1.6;
            max-width: 900px;
            margin: 0 auto;
            padding: 2rem;
            color: #333;
            background-color: #fff;
        }}
        .document-metadata {{
            margin-bottom: 3rem;
            padding-bottom: 2rem;
            border-bottom: 3px solid #eaecef;
        }}
        .doc-title {{
            font-size: 2.5em;
            margin-bottom: 0.5rem;
            margin-top: 0;
            border-bottom: none;
        }}
        .meta-info {{
            display: flex;
            gap: 2rem;
            color: #666;
            font-size: 0.95em;
        }}
        .author::before {{
            content: "By ";
        }}
        .date::before {{
            content: "Date: ";
        }}
        h1, h2, h3, h4, h5, h6 {{
            margin-top: 2.5em;
            margin-bottom: 0.5em;
            font-weight: 600;
            line-height: 1.25;
        }}
        h1 {{ font-size: 2em; border-bottom: 2px solid #eaecef; padding-bottom: 0.3em; margin-top: 3em; }}
        h2 {{ font-size: 1.5em; border-bottom: 1px solid #eaecef; padding-bottom: 0.3em; margin-top: 2.5em; }}
        h3 {{ font-size: 1.25em; margin-top: 2em; }}
        code {{
            background-color: #f6f8fa;
            padding: 0.2em 0.4em;
            border-radius: 3px;
            font-family: 'Courier New', Courier, monospace;
            font-size: 0.9em;
        }}
        pre {{
            background-color: #f6f8fa;
            padding: 1em;
            border-radius: 5px;
            overflow-x: auto;
        }}
        pre code {{
            background-color: transparent;
            padding: 0;
        }}
        .code-block-container {{
            margin: 1em 0;
        }}
        .code-filename {{
            background-color: #e1e4e8;
            color: #24292e;
            padding: 0.5em 1em;
            font-family: 'Courier New', Courier, monospace;
            font-size: 0.9em;
            font-weight: 600;
            border-radius: 5px 5px 0 0;
            border-bottom: 1px solid #d0d7de;
        }}
        .code-filename a {{
            color: #24292e;
            text-decoration: none;
        }}
        .code-filename a:hover {{
            color: #0366d6;
            text-decoration: underline;
        }}
        .code-block-container .code-filename + pre {{
            margin-top: 0;
            border-radius: 0 0 5px 5px;
        }}
        .line-number {{
            color: #8b949e;
            margin-right: 1em;
            user-select: none;
            display: inline-block;
            text-align: right;
            min-width: 3em;
        }}
        blockquote {{
            border-left: 4px solid #dfe2e5;
            padding-left: 1em;
            margin-left: 0;
            color: #6a737d;
        }}
        table {{
            border-collapse: collapse;
            width: 100%;
            margin: 1em 0;
        }}
        table th, table td {{
            border: 1px solid #dfe2e5;
            padding: 0.6em 1em;
            text-align: left;
        }}
        table th {{
            background-color: #f6f8fa;
            font-weight: 600;
        }}
        table tr:nth-child(even) {{
            background-color: #f6f8fa;
        }}
        a {{
            color: #0366d6;
            text-decoration: none;
        }}
        a:hover {{
            text-decoration: underline;
        }}
        img {{
            max-width: 100%;
            height: auto;
        }}
        ul, ol {{
            padding-left: 2em;
        }}
        li {{
            margin: 0.25em 0;
        }}
        hr {{
            border: 0;
            border-top: 2px solid #eaecef;
            margin: 2em 0;
        }}
    </style>
</head>
<body>
{html_output}
</body>
</html>"#,
    )
}
