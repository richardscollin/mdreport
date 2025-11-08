mod fmt;
mod layout;
mod parse;

use std::path::PathBuf;

use clap::{
    ArgAction,
    Parser,
    ValueEnum,
};
use syntect::highlighting::ThemeSet;

#[derive(Debug, Clone, ValueEnum)]
enum OutputFormat {
    Html,
    Pdf,
    Email,
    Slides,
}

#[derive(Parser, Debug)]
#[command(name = "markdown-report")]
#[command(about = "Generate HTML, PDF, or email reports from Markdown files", long_about = None)]
struct Args {
    /// Input markdown file
    input: Option<PathBuf>,

    /// Output file
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Output format (inferred from output file extension if not specified)
    #[arg(short, long, value_enum)]
    format: Option<OutputFormat>,

    /// List all available syntax highlighting themes and slide themes
    #[arg(long)]
    list_themes: bool,

    /// Code syntax highlighting theme to use
    #[arg(long, value_name = "THEME")]
    code_theme: Option<String>,

    /// Do not embed the source markdown file in the PDF
    #[arg(long = "no-embed-source", action = ArgAction::SetFalse, default_value = "true")]
    embed_source: bool,

    /// Extract embedded markdown from a PDF file
    #[arg(long)]
    extract: bool,
}

fn main() {
    let args = Args::parse();

    if args.list_themes {
        // List code syntax highlighting themes
        let theme_set = ThemeSet::load_defaults();
        let mut theme_names: Vec<&str> = theme_set.themes.keys().map(|s| s.as_str()).collect();
        theme_names.sort();

        println!("Available code syntax highlighting themes:");
        println!("  (Use with --code-theme or code_theme front matter)\n");
        for theme_name in theme_names {
            println!("  {}", theme_name);
        }

        // List slide themes
        println!("\n\nAvailable slide themes:");
        println!("  (Use with slide_theme front matter in slides format)\n");

        println!("  Solid themes:");
        println!("    light           - White background with dark text (default)");
        println!("    dark            - Dark gray background with light text");
        println!("    blue            - Dark blue background with light blue text");

        println!("\n  Gradient themes:");
        println!("    gradient-blue   - Light to dark blue gradient");
        println!("    gradient-purple - Light to dark purple gradient");
        println!("    gradient-sunset - Warm sunset color gradient");

        println!("\n  Radial themes:");
        println!("    radial-spotlight - Spotlight effect centered on page");
        println!("    radial-vignette  - Vignette effect with dark edges");
        println!("    radial-corner    - Radial gradient from corner");

        return;
    }

    let input = args.input.expect("Input file is required"); // if not listing themes

    // Handle extraction mode
    if args.extract {
        match crate::fmt::pdf::extract_markdown_from_pdf(&input) {
            Ok(markdown) => {
                let output_path = args.output.unwrap_or_else(|| {
                    let mut output = input.clone();
                    output.set_extension("md");
                    output
                });
                std::fs::write(&output_path, markdown).unwrap_or_else(|_| {
                    panic!("Failed to write output file: {}", output_path.display())
                });
                println!("Extracted markdown to: {}", output_path.display());
            }
            Err(e) => {
                eprintln!("Error extracting markdown: {}", e);
                std::process::exit(1);
            }
        }
        return;
    }
    let markdown_content = std::fs::read_to_string(&input)
        .unwrap_or_else(|_| panic!("Failed to read input file: {}", input.display()));

    let format = args.format.unwrap_or_else(|| {
        match args
            .output
            .as_ref()
            .and_then(|output| output.extension().and_then(|s| s.to_str()))
        {
            Some("pdf") => OutputFormat::Pdf,
            Some("html") => OutputFormat::Html,
            Some("email") => OutputFormat::Email,
            Some("slides") => OutputFormat::Slides,
            _ => OutputFormat::Pdf, // Default to PDF for unknown extensions
        }
    });

    let output_path = args.output.unwrap_or_else(|| {
        let mut output = input.clone();
        output.set_extension(match format {
            OutputFormat::Html => "html",
            OutputFormat::Pdf => "pdf",
            OutputFormat::Email => "txt",
            OutputFormat::Slides => "pdf",
        });
        output
    });

    match format {
        OutputFormat::Html => {
            let html_content = crate::fmt::html::to_html(&markdown_content);
            std::fs::write(&output_path, html_content)
                .unwrap_or_else(|_| panic!("Failed to write HTML file: {}", output_path.display()));
            println!("HTML report generated: {}", output_path.display());
        }
        OutputFormat::Pdf => {
            let output = std::fs::File::create(&output_path).unwrap();
            let mut output = std::io::BufWriter::new(output);
            crate::fmt::pdf::to_pdf(
                &markdown_content,
                &mut output,
                false,
                args.code_theme.as_deref(),
                args.embed_source,
                Some(&input),
            )
            .unwrap();
            println!("PDF report generated: {}", output_path.display());
        }
        OutputFormat::Slides => {
            let output = std::fs::File::create(&output_path).unwrap();
            let mut output = std::io::BufWriter::new(output);
            crate::fmt::pdf::to_pdf(
                &markdown_content,
                &mut output,
                true,
                args.code_theme.as_deref(),
                args.embed_source,
                Some(&input),
            )
            .unwrap();
            println!("Slides PDF generated: {}", output_path.display());
        }
        OutputFormat::Email => {
            // Generate HTML email
            let email_html = crate::fmt::email::to_html(&markdown_content);
            let html_path = output_path.clone();
            std::fs::write(&html_path, email_html).unwrap_or_else(|_| {
                panic!("Failed to write email HTML file: {}", html_path.display())
            });
            println!("Email HTML generated: {}", html_path.display());

            // Generate plain text email
            let email_text = crate::fmt::email::to_plain_text(&markdown_content);
            let mut text_path = output_path.clone();
            text_path.set_extension("txt");
            std::fs::write(&text_path, email_text).unwrap_or_else(|_| {
                panic!("Failed to write email text file: {}", text_path.display())
            });
            println!("Email text generated: {}", text_path.display());
        }
    }
}
