pub mod email;
pub mod html;
pub mod pdf;

use crate::parse::FrontMatter;

pub fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

pub fn build_github_url(
    filename: &str,
    line_number: Option<usize>,
    repo: &str,
    refspec: Option<&str>,
) -> String {
    let refspec = refspec.unwrap_or("main");
    let line_fragment = if let Some(line) = line_number {
        format!("#L{}", line)
    } else {
        String::new()
    };

    format!(
        "https://github.com/{}/blob/{}/{}{}",
        repo, refspec, filename, line_fragment
    )
}

/// Resolve which repository to use: code block repo or front matter default
pub fn resolve_repo<'a>(
    code_block_repo: Option<&'a String>,
    front_matter: Option<&'a FrontMatter>,
) -> Option<&'a String> {
    code_block_repo.or_else(|| front_matter.and_then(|fm| fm.repo.as_ref()))
}
