use std::fs;
use std::path::PathBuf;

/// Exhaustive visual regression test that generates all combinations of mdreport outputs
/// This test is ignored by default and should be run explicitly in CI for visual regression testing
///
/// Run with: cargo test exhaustive_test -- --ignored --nocapture
#[test]
#[ignore]
fn exhaustive_test() {
    // Create output directory for all generated files
    let output_dir = PathBuf::from("visual_regression_output");
    if output_dir.exists() {
        fs::remove_dir_all(&output_dir).expect("Failed to clean output directory");
    }
    fs::create_dir_all(&output_dir).expect("Failed to create output directory");

    // Read the README.md as test input
    let readme_path = PathBuf::from("README.md");
    let readme_content = fs::read_to_string(&readme_path).expect("Failed to read README.md");

    println!("Starting exhaustive visual regression tests...");
    println!("Output directory: {}", output_dir.display());

    let mut test_count = 0;

    // Test HTML format
    test_count += test_html_variants(&readme_content, &output_dir);

    // Test PDF format
    test_count += test_pdf_variants(&readme_content, &output_dir, &readme_path);

    // Test Email format
    test_count += test_email_variants(&readme_content, &output_dir);

    // Test Slides format
    test_count += test_slides_variants(&readme_content, &output_dir, &readme_path);

    println!("\n===========================================");
    println!("Exhaustive test completed successfully!");
    println!("Total test variants generated: {}", test_count);
    println!("Output directory: {}", output_dir.display());
    println!("===========================================");
}

fn test_html_variants(markdown: &str, output_dir: &PathBuf) -> usize {
    println!("\n--- Testing HTML variants ---");
    let mut count = 0;

    // Basic HTML
    let output_path = output_dir.join("html_basic.html");
    let html = mdreport::fmt::html::to_html(markdown);
    fs::write(&output_path, html).expect("Failed to write HTML");
    println!("Generated: {}", output_path.display());
    count += 1;

    count
}

fn test_pdf_variants(markdown: &str, output_dir: &PathBuf, source_path: &PathBuf) -> usize {
    println!("\n--- Testing PDF variants ---");
    let mut count = 0;

    // Get available code themes
    let code_themes = get_sample_code_themes();

    // Test with no theme
    count += generate_pdf(
        markdown,
        output_dir,
        "pdf_no_theme_embed.pdf",
        false,
        None,
        true,
        Some(source_path),
    );

    count += generate_pdf(
        markdown,
        output_dir,
        "pdf_no_theme_no_embed.pdf",
        false,
        None,
        false,
        Some(source_path),
    );

    // Test with each sample theme
    for theme in &code_themes {
        let filename = format!("pdf_theme_{}_embed.pdf", sanitize_filename(theme));
        count += generate_pdf(
            markdown,
            output_dir,
            &filename,
            false,
            Some(theme),
            true,
            Some(source_path),
        );

        let filename = format!("pdf_theme_{}_no_embed.pdf", sanitize_filename(theme));
        count += generate_pdf(
            markdown,
            output_dir,
            &filename,
            false,
            Some(theme),
            false,
            Some(source_path),
        );
    }

    count
}

fn test_email_variants(markdown: &str, output_dir: &PathBuf) -> usize {
    println!("\n--- Testing Email variants ---");
    let mut count = 0;

    // Email HTML
    let output_path = output_dir.join("email.html");
    let email_html = mdreport::fmt::email::to_html(markdown);
    fs::write(&output_path, email_html).expect("Failed to write email HTML");
    println!("Generated: {}", output_path.display());
    count += 1;

    // Email plain text
    let output_path = output_dir.join("email.txt");
    let email_text = mdreport::fmt::email::to_plain_text(markdown);
    fs::write(&output_path, email_text).expect("Failed to write email text");
    println!("Generated: {}", output_path.display());
    count += 1;

    count
}

fn test_slides_variants(markdown: &str, output_dir: &PathBuf, source_path: &PathBuf) -> usize {
    println!("\n--- Testing Slides variants ---");
    let mut count = 0;

    // Get available slide themes
    let slide_themes = mdreport::fmt::pdf::get_slide_themes();

    // Test basic slides without theme override (uses front matter)
    count += generate_pdf(
        markdown,
        output_dir,
        "slides_default.pdf",
        true,
        None,
        true,
        Some(source_path),
    );

    // Test with different slide themes by modifying front matter
    for theme_info in slide_themes {
        // Create markdown with different slide themes
        let markdown_with_theme = prepend_slide_theme(markdown, &theme_info.name);

        let filename = format!("slides_{}_embed.pdf", sanitize_filename(&theme_info.name));
        count += generate_pdf(
            &markdown_with_theme,
            output_dir,
            &filename,
            true,
            None,
            true,
            Some(source_path),
        );

        let filename = format!(
            "slides_{}_no_embed.pdf",
            sanitize_filename(&theme_info.name)
        );
        count += generate_pdf(
            &markdown_with_theme,
            output_dir,
            &filename,
            true,
            None,
            false,
            Some(source_path),
        );
    }

    // Test gradient directions (for gradient themes)
    let gradient_directions = vec![
        "top-to-bottom",
        "bottom-to-top",
        "left-to-right",
        "right-to-left",
        "diagonal",
        "diagonal-reverse",
        "diagonal-alt",
        "diagonal-alt-reverse",
    ];

    for direction in &gradient_directions {
        let markdown_with_gradient =
            prepend_slide_theme_and_direction(markdown, "gradient-purple", direction);

        let filename = format!(
            "slides_gradient_purple_{}.pdf",
            sanitize_filename(direction)
        );
        count += generate_pdf(
            &markdown_with_gradient,
            output_dir,
            &filename,
            true,
            None,
            true,
            Some(source_path),
        );
    }

    count
}

fn generate_pdf(
    markdown: &str,
    output_dir: &PathBuf,
    filename: &str,
    is_slides: bool,
    code_theme: Option<&str>,
    embed_source: bool,
    source_path: Option<&PathBuf>,
) -> usize {
    let output_path = output_dir.join(filename);
    let output_file = fs::File::create(&output_path).expect("Failed to create PDF file");
    let mut output = std::io::BufWriter::new(output_file);

    mdreport::fmt::pdf::to_pdf(
        markdown,
        &mut output,
        is_slides,
        code_theme,
        embed_source,
        source_path.map(|p| p.as_path()),
    )
    .expect("Failed to generate PDF");

    println!("Generated: {}", output_path.display());
    1
}

fn get_sample_code_themes() -> Vec<String> {
    // Return a subset of themes to test - testing all would create too many files
    // These represent a good variety: light, dark, different color schemes
    vec![
        "InspiredGitHub".to_string(),
        "Solarized (dark)".to_string(),
        "Solarized (light)".to_string(),
        "base16-ocean.dark".to_string(),
        "base16-ocean.light".to_string(),
    ]
}

fn sanitize_filename(s: &str) -> String {
    s.replace(" ", "_")
        .replace("(", "")
        .replace(")", "")
        .replace(".", "_")
        .to_lowercase()
}

fn prepend_slide_theme(markdown: &str, theme: &str) -> String {
    // Check if markdown already has front matter
    if markdown.trim_start().starts_with("---") {
        // Parse and modify existing front matter
        let parts: Vec<&str> = markdown.splitn(3, "---").collect();
        if parts.len() >= 3 {
            let lines: Vec<&str> = parts[1].lines().collect();
            let mut new_lines = Vec::new();
            let mut found_slide_theme = false;

            for line in lines {
                let trimmed = line.trim();
                // Skip commented out slide_theme lines
                if trimmed.starts_with('#') && trimmed.contains("slide_theme:") {
                    continue;
                }
                // Replace existing slide_theme
                if trimmed.starts_with("slide_theme:") {
                    new_lines.push(format!("slide_theme: \"{}\"", theme));
                    found_slide_theme = true;
                } else if !line.is_empty() || !new_lines.is_empty() {
                    new_lines.push(line.to_string());
                }
            }

            // Add slide_theme if not found
            if !found_slide_theme {
                new_lines.push(format!("slide_theme: \"{}\"", theme));
            }

            format!("---\n{}\n---{}", new_lines.join("\n"), parts[2])
        } else {
            markdown.to_string()
        }
    } else {
        // Add new front matter
        format!("---\nslide_theme: \"{}\"\n---\n{}", theme, markdown)
    }
}

fn prepend_slide_theme_and_direction(markdown: &str, theme: &str, direction: &str) -> String {
    // Check if markdown already has front matter
    if markdown.trim_start().starts_with("---") {
        let parts: Vec<&str> = markdown.splitn(3, "---").collect();
        if parts.len() >= 3 {
            let lines: Vec<&str> = parts[1].lines().collect();
            let mut new_lines = Vec::new();
            let mut found_slide_theme = false;
            let mut found_gradient_direction = false;

            for line in lines {
                let trimmed = line.trim();
                // Skip commented out lines
                if trimmed.starts_with('#')
                    && (trimmed.contains("slide_theme:") || trimmed.contains("gradient_direction:"))
                {
                    continue;
                }
                // Replace existing slide_theme
                if trimmed.starts_with("slide_theme:") {
                    new_lines.push(format!("slide_theme: \"{}\"", theme));
                    found_slide_theme = true;
                } else if trimmed.starts_with("gradient_direction:") {
                    new_lines.push(format!("gradient_direction: \"{}\"", direction));
                    found_gradient_direction = true;
                } else if !line.is_empty() || !new_lines.is_empty() {
                    new_lines.push(line.to_string());
                }
            }

            // Add missing fields
            if !found_slide_theme {
                new_lines.push(format!("slide_theme: \"{}\"", theme));
            }
            if !found_gradient_direction {
                new_lines.push(format!("gradient_direction: \"{}\"", direction));
            }

            format!("---\n{}\n---{}", new_lines.join("\n"), parts[2])
        } else {
            markdown.to_string()
        }
    } else {
        format!(
            "---\nslide_theme: \"{}\"\ngradient_direction: \"{}\"\n---\n{}",
            theme, direction, markdown
        )
    }
}
