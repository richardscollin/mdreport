---
title: mdreport
description: A command-line tool to generate HTML or PDF reports from Markdown files
code_theme: "Solarized (light)"
# slide_theme: "gradient-purple"
---

# mdreport

A command-line tool to generate HTML or PDF reports from Markdown files.
This project was primarily generated with claude code using agentic coding.

## Installation

```bash
cargo install --locked --git https://github.com/richardscollin/mdreport
```

## Usage

### Basic Usage

```bash
# Generate HTML report (default)
mdreport -i input.md

# Generate PDF report
mdreport -i input.md -f pdf

# Specify output file
mdreport -i input.md -o report.html

# Set custom title
mdreport -i input.md -t "My Report"
```

### Options

- `-i, --input <FILE>` - Input markdown file (required unless using --list-themes)
- `-o, --output <FILE>` - Output file (defaults to input filename with new extension)
- `-f, --format <FORMAT>` - Output format: html, pdf, or slides (default: html)
- `--list-themes` - List all available syntax highlighting themes and slide themes
- `--no-embed-source` - Do not embed the source markdown file in the PDF (embedding is enabled by default)
- `--extract` - Extract embedded markdown from a PDF file
- `-h, --help` - Print help information

## Examples

### Generate HTML Report

```bash
mdreport -i documentation.md -o report.html -t "Project Documentation"
```

### Generate PDF Report

```bash
mdreport -i notes.md -f pdf -t "Meeting Notes"
```

### List Available Themes

```bash
mdreport --list-themes
```

### Embed and Extract Markdown Source

By default, the source markdown is embedded in generated PDF files, allowing you to extract it later:

```bash
# Generate PDF with embedded source (default behavior)
mdreport -i notes.md -f pdf

# Generate PDF without embedding source
mdreport -i notes.md -f pdf --no-embed-source

# Extract embedded markdown from a PDF
mdreport -i report.pdf --extract -o extracted.md

# Extract with default output name (input.pdf -> input.md)
mdreport -i report.pdf --extract
```

This feature is useful for:
- Version control: Keep the original markdown alongside the PDF
- Editing: Extract and modify the source from a PDF
- Archival: Ensure the source is never lost

### Presentation Slides

Generate beautiful presentation slides from Markdown with customizable themes. The slides format creates a PDF with each H2 heading starting a new slide.

```bash
# Generate slides (auto-detected from .slides.pdf extension)
mdreport -i presentation.md -o slides.slides.pdf

# Or explicitly specify slides format
mdreport -i presentation.md -f slides -o presentation.pdf
```

#### Available Slide Themes

Control the visual appearance of your slides using the `slide_theme` front matter option. Use `--list-themes` to see all available themes.

**Solid Themes:**
- `light` - White background with dark text (default)
- `dark` - Dark gray background with light text
- `blue` - Dark blue background with light blue text

**Gradient Themes:**
- `gradient-blue` - Light to dark blue gradient
- `gradient-purple` - Light to dark purple gradient
- `gradient-sunset` - Warm sunset color gradient

**Radial Themes:**
- `radial-spotlight` - Spotlight effect centered on page
- `radial-vignette` - Vignette effect with dark edges
- `radial-corner` - Radial gradient from corner

#### Customizing Gradient Direction

For gradient themes, you can specify the direction using the `gradient_direction` front matter option:

Available directions: `top-to-bottom`, `bottom-to-top`, `left-to-right`, `right-to-left`, `diagonal`, `diagonal-reverse`, `diagonal-alt`, `diagonal-alt-reverse`

#### Example Presentation

````markdown
---
title: My Presentation
author: Jane Doe
date: 2025-11-08
slide_theme: "gradient-purple"
gradient_direction: "diagonal"
---

# Welcome to My Presentation

This content appears on the title slide.

## First Topic

Each H2 heading creates a new slide automatically.

- Bullet points work great
- Multiple levels supported

## Second Topic

You can include code blocks:

```rust
fn main() {
    println!("Hello from slides!");
}
```

## Conclusion

Thank you!
````

### Syntax Highlighting

PDF output supports syntax highlighting for code blocks. Simply specify the language after the opening backticks:

````markdown
```rust
fn main() {
    println!("Hello, world!");
}
```
````

Supported languages include Rust, Python, JavaScript, Java, C, C++, Go, Ruby, PHP, and many more.

#### Code Blocks with Filenames and Line Numbers

You can display code blocks with filenames and line numbers by using the format `filename:line_number`:

````markdown
```src/main.rs:39
#[derive(Debug, Deserialize, Default)]
struct FrontMatter {
    title: Option<String>,
    author: Option<String>,
}
```
````

This will:
- Display the filename above the code block
- Show line numbers starting from the specified line (39 in this example)
- Automatically detect the language from the file extension

Works in both HTML and PDF output!

### Front Matter

Add YAML front matter at the beginning of your markdown file to include document metadata:

````markdown
---
title: My Document Title
author: John Doe
date: 2025-10-16
code_theme: base16-ocean.dark
---

# Your content starts here
````

The front matter supports:
- **title**: Document title (displayed prominently in both HTML and PDF)
- **author**: Author name
- **date**: Document date
- **code_theme**: Syntax highlighting theme for code blocks in PDF (use `--list-themes` to see options)
- **slide_theme**: Slide theme for presentation slides (see [Presentation Slides](#presentation-slides) section)
- **gradient_direction**: Direction for gradient slide themes (see [Presentation Slides](#presentation-slides) section)

## Examples / Tests

### Table

| Function                              | Purpose                                      |
|---------------------------------------|----------------------------------------------|
| `setupterm(term, filedes, errret)`    | Initialize terminal for given terminal type  |
| `tiparm_s(expected, mask, str, ...)`  | Safe parameter formatting (modern)           |
| `tiparm(str, ...)`                    | Parameter formatting (newer)                 |
| `tparm(str, ...)`                     | Parameter formatting (legacy)                |
| `tigetflag(cap_code)`                 | Retrieve boolean terminal capability         |
| `tigetnum(cap_code)`                  | Retrieve numeric terminal capability         |
| `tigetstr(cap_code)`                  | Retrieve string terminal capability          |
| `del_curterm(oterm)`                  | Free terminal data structure                 |

### Task Lists

Test checkbox rendering:

- [ ] Unchecked task 1
- [x] Checked task 2
- [ ] Unchecked task 3
- [x] Checked task 4

