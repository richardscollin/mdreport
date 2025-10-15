use std::str::FromStr;

use pulldown_cmark::Options;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, Default)]
pub struct FrontMatter {
    pub title: Option<String>,
    pub author: Option<String>,
    pub date: Option<String>,
    pub code_theme: Option<String>,
    pub slide_theme: Option<String>,
    pub gradient_direction: Option<String>,
    pub repo: Option<String>,
}

pub struct MarkdownParser<'input> {
    front_matter: Option<FrontMatter>,
    markdown_parser: pulldown_cmark::Parser<'input>,
}

impl<'input> MarkdownParser<'input> {
    pub fn new(markdown_content: &'input str) -> Result<Self, serde_yaml::Error> {
        let mut options = Options::empty();
        options.insert(Options::ENABLE_TABLES);
        options.insert(Options::ENABLE_STRIKETHROUGH);
        options.insert(Options::ENABLE_TASKLISTS);
        options.insert(Options::ENABLE_HEADING_ATTRIBUTES);
        options.insert(Options::ENABLE_GFM);

        let (front_matter, remaining) =
            if let Some((fm, rem)) = parse_front_matter(markdown_content)? {
                (Some(fm), rem)
            } else {
                (None, markdown_content)
            };

        Ok(Self {
            front_matter,
            markdown_parser: pulldown_cmark::Parser::new_ext(remaining, options),
        })
    }

    pub fn front_matter(&self) -> Option<&FrontMatter> {
        self.front_matter.as_ref()
    }

    pub fn into_inner(self) -> pulldown_cmark::Parser<'input> {
        self.markdown_parser
    }
}

pub fn parse_front_matter(content: &str) -> Result<Option<(FrontMatter, &str)>, serde_yaml::Error> {
    if !content.starts_with("---\n") {
        return Ok(None);
    }

    // Find the closing ---
    let after_opening = &content[4..]; // Skip "---\n"

    if let Some(closing_pos) = after_opening.find("\n---\n") {
        let yaml_content = &after_opening[..closing_pos];
        let front_matter: FrontMatter = serde_yaml::from_str(yaml_content)?;

        // Remaining content starts after "\n---\n"
        let remaining = &after_opening[closing_pos + 5..];

        Ok(Some((front_matter, remaining)))
    } else {
        Ok(None)
    }
}

#[derive(Debug, PartialEq)]
pub struct CodeBlockInfo {
    pub language: String,
    pub filename: Option<String>,
    pub start_line: Option<usize>,
    pub repo: Option<String>,
    pub refspec: Option<String>,
}

impl FromStr for CodeBlockInfo {
    type Err = ();

    /// Parse format: "path/to/file.rs:12 @ github-user/repo#refspec"
    /// Also supports:
    /// - "path/to/file.rs:12 @ github-user/repo"
    /// - "path/to/file.rs:12"
    /// - "path/to/file.rs @ github-user/repo#refspec"
    /// - "language" (plain language identifier)
    fn from_str(info: &str) -> Result<Self, Self::Err> {
        let mut repo = None;
        let mut refspec = None;
        let mut file_part = info;

        // Check for repo information (after @)
        if let Some(at_pos) = info.find(" @ ") {
            file_part = &info[..at_pos];
            let repo_part = &info[at_pos + 3..];

            // Check for refspec (after #)
            if let Some(hash_pos) = repo_part.find('#') {
                repo = Some(repo_part[..hash_pos].to_string());
                refspec = Some(repo_part[hash_pos + 1..].to_string());
            } else {
                repo = Some(repo_part.to_string());
            }
        }

        // Now parse the file part for filename and line number
        if let Some(colon_pos) = file_part.rfind(':') {
            // Check if the part after the colon is a number
            let after_colon = &file_part[colon_pos + 1..];
            if let Ok(line_num) = after_colon.parse::<usize>() {
                // It's a filename:line_number format
                let filename = file_part[..colon_pos].to_string();

                // Extract language from file extension
                let language = if let Some(dot_pos) = filename.rfind('.') {
                    filename[dot_pos + 1..].to_string()
                } else {
                    String::new()
                };

                return Ok(CodeBlockInfo {
                    language,
                    filename: Some(filename),
                    start_line: Some(line_num),
                    repo,
                    refspec,
                });
            }
        }

        // Check if it's just a filename without line number
        if file_part.contains('/') || file_part.contains('.') {
            let filename = file_part.to_string();
            let language = if let Some(dot_pos) = filename.rfind('.') {
                filename[dot_pos + 1..].to_string()
            } else {
                String::new()
            };

            return Ok(CodeBlockInfo {
                language,
                filename: Some(filename),
                start_line: None,
                repo,
                refspec,
            });
        }

        // Regular language identifier
        Ok(CodeBlockInfo {
            language: info.to_string(),
            filename: None,
            start_line: None,
            repo: None,
            refspec: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_code_block_info_simple_language() {
        assert_eq!(
            CodeBlockInfo::from_str("rust"),
            Ok(CodeBlockInfo {
                language: "rust".into(),
                filename: None,
                start_line: None,
                repo: None,
                refspec: None,
            })
        );
    }

    #[test]
    fn test_parse_code_block_info_with_line_number() {
        assert_eq!(
            CodeBlockInfo::from_str("src/main.rs:42"),
            Ok(CodeBlockInfo {
                language: "rs".into(),
                filename: Some("src/main.rs".into(),),
                start_line: Some(42,),
                repo: None,
                refspec: None,
            })
        );
    }

    #[test]
    fn test_parse_code_block_info_with_repo() {
        assert_eq!(
            CodeBlockInfo::from_str("src/main.rs:42 @ user/repo"),
            Ok(CodeBlockInfo {
                language: "rs".into(),
                filename: Some("src/main.rs".into(),),
                start_line: Some(42,),
                repo: Some("user/repo".into(),),
                refspec: None,
            })
        );
    }

    #[test]
    fn test_parse_code_block_info_with_repo_and_refspec() {
        assert_eq!(
            CodeBlockInfo::from_str("src/main.rs:42 @ user/repo#develop"),
            Ok(CodeBlockInfo {
                language: "rs".into(),
                filename: Some("src/main.rs".into(),),
                start_line: Some(42,),
                repo: Some("user/repo".into(),),
                refspec: Some("develop".into(),),
            })
        );
    }
}
