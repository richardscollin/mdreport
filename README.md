---
title: markdown-report
description: A command-line tool to generate HTML or PDF reports from Markdown files
code_theme: "Solarized (light)"
# slide_theme: "gradient-purple"
---

# markdown-report

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
markdown-report -i input.md

# Generate PDF report
markdown-report -i input.md -f pdf

# Specify output file
markdown-report -i input.md -o report.html

# Set custom title
markdown-report -i input.md -t "My Report"
```

### Options

- `-i, --input <FILE>` - Input markdown file (required unless using --list-themes)
- `-o, --output <FILE>` - Output file (defaults to input filename with new extension)
- `-f, --format <FORMAT>` - Output format: html or pdf (default: html)
- `--list-themes` - List all available syntax highlighting themes
- `--no-embed-source` - Do not embed the source markdown file in the PDF (embedding is enabled by default)
- `--extract` - Extract embedded markdown from a PDF file
- `-h, --help` - Print help information

## Examples

### Generate HTML Report

```bash
markdown-report -i documentation.md -o report.html -t "Project Documentation"
```

### Generate PDF Report

```bash
markdown-report -i notes.md -f pdf -t "Meeting Notes"
```

### List Available Themes

```bash
markdown-report --list-themes
```

### Embed and Extract Markdown Source

By default, the source markdown is embedded in generated PDF files, allowing you to extract it later:

```bash
# Generate PDF with embedded source (default behavior)
markdown-report -i notes.md -f pdf

# Generate PDF without embedding source
markdown-report -i notes.md -f pdf --no-embed-source

# Extract embedded markdown from a PDF
markdown-report -i report.pdf --extract -o extracted.md

# Extract with default output name (input.pdf -> input.md)
markdown-report -i report.pdf --extract
```

This feature is useful for:
- Version control: Keep the original markdown alongside the PDF
- Editing: Extract and modify the source from a PDF
- Archival: Ensure the source is never lost

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
- **slides_theme**: Slide theme in PDF

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

