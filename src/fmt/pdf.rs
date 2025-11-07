use std::{
    io::Cursor,
    path::Path,
};

use lopdf::{
    Document,
    Object,
    ObjectId,
    Stream,
    content::{
        Content,
        Operation,
    },
    dictionary,
};
use pulldown_cmark::{
    CodeBlockKind,
    Event,
    Tag,
    TagEnd,
};
use syntect::{
    easy::HighlightLines,
    highlighting::ThemeSet,
    parsing::SyntaxSet,
};

use crate::{
    layout::LayoutItem,
    parse::{
        CodeBlockInfo,
        FrontMatter,
        MarkdownParser,
    },
};

/// PDF measurement unit (millimeters)
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
struct Mm(f32);

impl Mm {
    /// Convert millimeters to PDF points (1 mm = 2.83465 points)
    fn to_points(self) -> f32 {
        self.0 * 2.83465
    }
}

impl From<f32> for Mm {
    fn from(value: f32) -> Self {
        Mm(value)
    }
}

impl std::ops::Sub for Mm {
    type Output = Mm;
    fn sub(self, rhs: Mm) -> Mm {
        Mm(self.0 - rhs.0)
    }
}

impl std::ops::SubAssign for Mm {
    fn sub_assign(&mut self, rhs: Mm) {
        self.0 -= rhs.0;
    }
}

impl std::ops::Add for Mm {
    type Output = Mm;
    fn add(self, rhs: Mm) -> Mm {
        Mm(self.0 + rhs.0)
    }
}

impl std::ops::AddAssign for Mm {
    fn add_assign(&mut self, rhs: Mm) {
        self.0 += rhs.0;
    }
}

impl std::ops::Mul<f32> for Mm {
    type Output = Mm;
    fn mul(self, rhs: f32) -> Mm {
        Mm(self.0 * rhs)
    }
}

impl std::ops::Div<f32> for Mm {
    type Output = Mm;
    fn div(self, rhs: f32) -> Mm {
        Mm(self.0 / rhs)
    }
}

/// Slide theme configuration
#[derive(Clone, Debug)]
struct SlideTheme {
    background: BackgroundStyle,
    text_color: (f32, f32, f32),
    heading_color: (f32, f32, f32),
}

#[derive(Clone, Debug)]
enum BackgroundStyle {
    Solid((f32, f32, f32)),
    Gradient {
        from: (f32, f32, f32),
        to: (f32, f32, f32),
        direction: GradientDirection,
    },
    Radial {
        center_color: (f32, f32, f32),
        edge_color: (f32, f32, f32),
        center_x: f32, // 0.0 to 1.0 (percentage of width)
        center_y: f32, // 0.0 to 1.0 (percentage of height)
        radius: f32,   // 0.0 to 1.0 (percentage of diagonal)
    },
}

#[derive(Clone, Debug)]
enum GradientDirection {
    TopToBottom,
    BottomToTop,
    LeftToRight,
    RightToLeft,
    TopLeftToBottomRight,
    TopRightToBottomLeft,
    BottomLeftToTopRight,
    BottomRightToTopLeft,
}

impl SlideTheme {
    fn get_by_name(name: &str) -> Self {
        match name {
            "dark" => Self {
                background: BackgroundStyle::Solid((0.1, 0.1, 0.1)),
                text_color: (0.9, 0.9, 0.9),
                heading_color: (1.0, 1.0, 1.0),
            },
            "light" => Self {
                background: BackgroundStyle::Solid((1.0, 1.0, 1.0)),
                text_color: (0.0, 0.0, 0.0),
                heading_color: (0.0, 0.0, 0.0),
            },
            "blue" => Self {
                background: BackgroundStyle::Solid((0.1, 0.2, 0.3)),
                text_color: (0.9, 0.95, 1.0),
                heading_color: (0.4, 0.7, 1.0),
            },
            "gradient-blue" => Self {
                background: BackgroundStyle::Gradient {
                    from: (0.1, 0.2, 0.4),
                    to: (0.05, 0.1, 0.2),
                    direction: GradientDirection::TopToBottom,
                },
                text_color: (0.9, 0.95, 1.0),
                heading_color: (0.5, 0.8, 1.0),
            },
            "gradient-purple" => Self {
                background: BackgroundStyle::Gradient {
                    from: (0.3, 0.1, 0.4),
                    to: (0.15, 0.05, 0.25),
                    direction: GradientDirection::TopToBottom,
                },
                text_color: (0.95, 0.9, 1.0),
                heading_color: (0.8, 0.5, 1.0),
            },
            "gradient-sunset" => Self {
                background: BackgroundStyle::Gradient {
                    from: (0.4, 0.2, 0.3),
                    to: (0.2, 0.1, 0.2),
                    direction: GradientDirection::TopToBottom,
                },
                text_color: (1.0, 0.95, 0.9),
                heading_color: (1.0, 0.8, 0.6),
            },
            "radial-spotlight" => Self {
                background: BackgroundStyle::Radial {
                    center_color: (0.2, 0.25, 0.3),
                    edge_color: (0.05, 0.05, 0.1),
                    center_x: 0.5,
                    center_y: 0.5,
                    radius: 0.8,
                },
                text_color: (0.9, 0.95, 1.0),
                heading_color: (0.5, 0.8, 1.0),
            },
            "radial-vignette" => Self {
                background: BackgroundStyle::Radial {
                    center_color: (0.15, 0.15, 0.15),
                    edge_color: (0.0, 0.0, 0.0),
                    center_x: 0.5,
                    center_y: 0.5,
                    radius: 1.0,
                },
                text_color: (0.95, 0.95, 0.95),
                heading_color: (1.0, 1.0, 1.0),
            },
            "radial-corner" => Self {
                background: BackgroundStyle::Radial {
                    center_color: (0.3, 0.2, 0.4),
                    edge_color: (0.1, 0.05, 0.15),
                    center_x: 0.0,
                    center_y: 1.0,
                    radius: 1.2,
                },
                text_color: (0.95, 0.9, 1.0),
                heading_color: (0.8, 0.6, 1.0),
            },
            _ => Self::default(),
        }
    }

    fn with_direction(mut self, direction: GradientDirection) -> Self {
        if let BackgroundStyle::Gradient {
            direction: ref mut d,
            ..
        } = self.background
        {
            *d = direction;
        }
        self
    }
}

impl Default for SlideTheme {
    fn default() -> Self {
        Self {
            background: BackgroundStyle::Solid((1.0, 1.0, 1.0)),
            text_color: (0.0, 0.0, 0.0),
            heading_color: (0.0, 0.0, 0.0),
        }
    }
}

/// Built-in PDF font names
#[derive(Clone, Copy, Debug, PartialEq)]
enum BuiltinFont {
    Courier,
    Helvetica,
    HelveticaBold,
    HelveticaOblique,
    HelveticaBoldOblique,
}

impl BuiltinFont {
    fn to_pdf_name(self) -> &'static str {
        match self {
            BuiltinFont::Courier => "Courier",
            BuiltinFont::Helvetica => "Helvetica",
            BuiltinFont::HelveticaBold => "Helvetica-Bold",
            BuiltinFont::HelveticaOblique => "Helvetica-Oblique",
            BuiltinFont::HelveticaBoldOblique => "Helvetica-BoldOblique",
        }
    }

    fn to_font_key(self) -> String {
        format!("F{}", self as u8)
    }
}

/// PDF builder that manages page operations and layout
struct PdfBuilder {
    doc: Document,
    current_ops: Vec<Operation>,
    y_position: Mm,
    page_width: Mm,
    page_height: Mm,
    left_margin: Mm,
    right_margin: Mm,
    line_height: Mm,
    in_text_section: bool,
    font_ids: std::collections::HashMap<String, ObjectId>,
    shading_ids: std::collections::HashMap<String, ObjectId>,
    page_ids: Vec<ObjectId>,
    is_slide: bool,
    slide_theme: SlideTheme,
}

impl PdfBuilder {
    fn new(title: &str, slide_theme: SlideTheme) -> Self {
        let mut doc = Document::with_version("1.5");

        // Set document info
        let info_id = doc.add_object(dictionary! {
            "Title" => Object::string_literal(title),
            "Creator" => Object::string_literal("mdreport"),
        });
        doc.trailer.set("Info", Object::Reference(info_id));

        Self {
            doc,
            current_ops: Vec::new(),
            y_position: Mm(270.0),
            page_width: Mm(210.0),
            page_height: Mm(297.0),
            left_margin: Mm(20.0),
            right_margin: Mm(190.0),
            line_height: Mm(6.0),
            in_text_section: false,
            font_ids: std::collections::HashMap::new(),
            shading_ids: std::collections::HashMap::new(),
            page_ids: Vec::new(),
            is_slide: false,
            slide_theme,
        }
    }

    fn new_slide(title: &str, slide_theme: SlideTheme) -> Self {
        let mut doc = Document::with_version("1.5");

        // Set document info
        let info_id = doc.add_object(dictionary! {
            "Title" => Object::string_literal(title),
            "Creator" => Object::string_literal("mdreport"),
        });
        doc.trailer.set("Info", Object::Reference(info_id));

        Self {
            doc,
            current_ops: Vec::new(),
            y_position: Mm(122.875),
            page_width: Mm(254.0),
            page_height: Mm(142.875),
            left_margin: Mm(15.0),
            right_margin: Mm(239.0),
            line_height: Mm(6.0),
            in_text_section: false,
            font_ids: std::collections::HashMap::new(),
            shading_ids: std::collections::HashMap::new(),
            page_ids: Vec::new(),
            is_slide: true,
            slide_theme,
        }
    }

    fn ensure_font(&mut self, font: BuiltinFont) -> String {
        let font_key = font.to_font_key();
        if !self.font_ids.contains_key(&font_key) {
            let font_dict = dictionary! {
                "Type" => "Font",
                "Subtype" => "Type1",
                "BaseFont" => font.to_pdf_name(),
            };
            let font_id = self.doc.add_object(font_dict);
            self.font_ids.insert(font_key.clone(), font_id);
        }
        font_key
    }

    fn start_text_section(&mut self) {
        if !self.in_text_section {
            self.current_ops.push(Operation::new("BT", vec![]));
            self.in_text_section = true;
        }
    }

    fn end_text_section(&mut self) {
        if self.in_text_section {
            self.current_ops.push(Operation::new("ET", vec![]));
            self.in_text_section = false;
        }
    }

    fn check_page_break(&mut self, needed_height: Mm) {
        if self.y_position - needed_height < Mm(30.0) {
            self.new_page();
        }
    }

    fn new_page(&mut self) {
        self.end_text_section();

        if !self.current_ops.is_empty() {
            self.add_page_to_doc();
        }

        self.y_position = if self.is_slide {
            Mm(122.875)
        } else {
            Mm(270.0)
        };
        self.in_text_section = false;

        // Draw background if in slide mode and not using white background
        if self.is_slide {
            self.draw_background();
        }
    }

    fn draw_background(&mut self) {
        let background = self.slide_theme.background.clone();
        match background {
            BackgroundStyle::Solid(color) => {
                // Only draw background if it's not white
                if color != (1.0, 1.0, 1.0) {
                    self.current_ops.push(Operation::new("q", vec![]));

                    // Set fill color
                    self.current_ops.push(Operation::new(
                        "rg",
                        vec![color.0.into(), color.1.into(), color.2.into()],
                    ));

                    // Draw filled rectangle covering entire page
                    self.current_ops.push(Operation::new(
                        "re",
                        vec![
                            0.0.into(),
                            0.0.into(),
                            self.page_width.to_points().into(),
                            self.page_height.to_points().into(),
                        ],
                    ));

                    self.current_ops.push(Operation::new("f", vec![])); // Fill
                    self.current_ops.push(Operation::new("Q", vec![]));
                }
            }
            BackgroundStyle::Gradient {
                from,
                to,
                direction,
            } => {
                self.draw_gradient(from, to, &direction);
            }
            BackgroundStyle::Radial {
                center_color,
                edge_color,
                center_x,
                center_y,
                radius,
            } => {
                self.draw_radial_gradient(center_color, edge_color, center_x, center_y, radius);
            }
        }
    }

    fn draw_gradient(
        &mut self,
        from: (f32, f32, f32),
        to: (f32, f32, f32),
        direction: &GradientDirection,
    ) {
        // Create a key for this gradient to reuse if already created
        let key = format!("{:?}_{:?}_{:?}", from, to, direction);

        if !self.shading_ids.contains_key(&key) {
            // Calculate coordinates based on direction
            let (x0, y0, x1, y1) = match direction {
                GradientDirection::TopToBottom => (0.0, self.page_height.to_points(), 0.0, 0.0),
                GradientDirection::BottomToTop => (0.0, 0.0, 0.0, self.page_height.to_points()),
                GradientDirection::LeftToRight => (0.0, 0.0, self.page_width.to_points(), 0.0),
                GradientDirection::RightToLeft => (self.page_width.to_points(), 0.0, 0.0, 0.0),
                GradientDirection::TopLeftToBottomRight => (
                    0.0,
                    self.page_height.to_points(),
                    self.page_width.to_points(),
                    0.0,
                ),
                GradientDirection::TopRightToBottomLeft => (
                    self.page_width.to_points(),
                    self.page_height.to_points(),
                    0.0,
                    0.0,
                ),
                GradientDirection::BottomLeftToTopRight => (
                    0.0,
                    0.0,
                    self.page_width.to_points(),
                    self.page_height.to_points(),
                ),
                GradientDirection::BottomRightToTopLeft => (
                    self.page_width.to_points(),
                    0.0,
                    0.0,
                    self.page_height.to_points(),
                ),
            };

            // Create the shading function (Type 2 = exponential interpolation)
            let function_dict = dictionary! {
                "FunctionType" => 2,
                "Domain" => Object::Array(vec![0.0.into(), 1.0.into()]),
                "C0" => Object::Array(vec![from.0.into(), from.1.into(), from.2.into()]),
                "C1" => Object::Array(vec![to.0.into(), to.1.into(), to.2.into()]),
                "N" => 1.0, // Linear interpolation
            };
            let function_id = self.doc.add_object(function_dict);

            // Create the shading dictionary (Type 2 = axial/linear gradient)
            let shading_dict = dictionary! {
                "ShadingType" => 2,
                "ColorSpace" => "DeviceRGB",
                "Coords" => Object::Array(vec![x0.into(), y0.into(), x1.into(), y1.into()]),
                "Function" => Object::Reference(function_id),
                "Extend" => Object::Array(vec![Object::Boolean(true), Object::Boolean(true)]), // Extend colors beyond gradient range
            };
            let shading_id = self.doc.add_object(shading_dict);
            self.shading_ids.insert(key.clone(), shading_id);
        }

        let _shading_id = self.shading_ids[&key];
        let shading_name = format!("Sh{}", self.shading_ids.len());

        // Use the shading operator to paint the gradient
        self.current_ops.push(Operation::new(
            "sh",
            vec![Object::Name(shading_name.as_bytes().to_vec())],
        ));
    }

    fn draw_radial_gradient(
        &mut self,
        center_color: (f32, f32, f32),
        edge_color: (f32, f32, f32),
        center_x: f32,
        center_y: f32,
        radius: f32,
    ) {
        // Create a key for this radial gradient
        let key = format!(
            "radial_{:?}_{:?}_{}_{}_{}",
            center_color, edge_color, center_x, center_y, radius
        );

        if !self.shading_ids.contains_key(&key) {
            // Calculate center position and radius in points
            let cx = self.page_width.to_points() * center_x;
            let cy = self.page_height.to_points() * center_y;

            // Calculate diagonal for radius scaling
            let diagonal =
                (self.page_width.to_points().powi(2) + self.page_height.to_points().powi(2)).sqrt();
            let r = diagonal * radius;

            // Create the shading function (Type 2 = exponential interpolation)
            let function_dict = dictionary! {
                "FunctionType" => 2,
                "Domain" => Object::Array(vec![0.0.into(), 1.0.into()]),
                "C0" => Object::Array(vec![center_color.0.into(), center_color.1.into(), center_color.2.into()]),
                "C1" => Object::Array(vec![edge_color.0.into(), edge_color.1.into(), edge_color.2.into()]),
                "N" => 1.0, // Linear interpolation
            };
            let function_id = self.doc.add_object(function_dict);

            // Create the radial shading dictionary (Type 3 = radial gradient)
            let shading_dict = dictionary! {
                "ShadingType" => 3,
                "ColorSpace" => "DeviceRGB",
                "Coords" => Object::Array(vec![
                    cx.into(), cy.into(), 0.0.into(),  // Start circle: center + radius 0
                    cx.into(), cy.into(), r.into(),     // End circle: center + outer radius
                ]),
                "Function" => Object::Reference(function_id),
                "Extend" => Object::Array(vec![Object::Boolean(true), Object::Boolean(true)]),
            };
            let shading_id = self.doc.add_object(shading_dict);
            self.shading_ids.insert(key.clone(), shading_id);
        }

        let _shading_id = self.shading_ids[&key];
        let shading_name = format!("Sh{}", self.shading_ids.len());

        // Use the shading operator to paint the radial gradient
        self.current_ops.push(Operation::new(
            "sh",
            vec![Object::Name(shading_name.as_bytes().to_vec())],
        ));
    }

    fn add_page_to_doc(&mut self) {
        // Create content stream
        let operations = std::mem::take(&mut self.current_ops);
        let content = Content { operations };
        let content_data = content.encode().unwrap();

        let content_stream = Stream::new(dictionary! {}, content_data);
        let content_id = self.doc.add_object(content_stream);

        // Create resources dictionary with fonts
        let mut fonts_dict = lopdf::Dictionary::new();
        for (font_key, font_id) in &self.font_ids {
            fonts_dict.set(font_key.as_str(), Object::Reference(*font_id));
        }

        // Create shading dictionary for resources
        let mut shading_dict = lopdf::Dictionary::new();
        for (idx, (_key, shading_id)) in self.shading_ids.iter().enumerate() {
            let shading_name = format!("Sh{}", idx + 1);
            shading_dict.set(shading_name.as_str(), Object::Reference(*shading_id));
        }

        let mut resources = dictionary! {
            "Font" => Object::Dictionary(fonts_dict),
        };

        if !shading_dict.is_empty() {
            resources.set("Shading", Object::Dictionary(shading_dict));
        }

        // Create page dictionary
        let page_dict = dictionary! {
            "Type" => "Page",
            "MediaBox" => vec![
                0.into(),
                0.into(),
                self.page_width.to_points().into(),
                self.page_height.to_points().into(),
            ],
            "Contents" => Object::Reference(content_id),
            "Resources" => resources,
        };

        let page_id = self.doc.add_object(page_dict);

        // Track page IDs
        self.page_ids.push(page_id);
    }

    fn finalize(mut self) -> Document {
        if !self.current_ops.is_empty() {
            self.end_text_section();
            self.add_page_to_doc();
        }

        // Build page tree
        let pages_refs: Vec<Object> = self
            .page_ids
            .iter()
            .map(|id| Object::Reference(*id))
            .collect();

        let pages_dict = dictionary! {
            "Type" => "Pages",
            "Count" => self.page_ids.len() as i64,
            "Kids" => pages_refs,
        };
        let pages_id = self.doc.add_object(pages_dict);

        // Update each page's Parent
        for page_id in &self.page_ids {
            if let Ok(page_obj) = self.doc.get_object_mut(*page_id)
                && let Object::Dictionary(dict) = page_obj
            {
                dict.set("Parent", Object::Reference(pages_id));
            }
        }

        // Set catalog
        let catalog = dictionary! {
            "Type" => "Catalog",
            "Pages" => Object::Reference(pages_id),
        };
        let catalog_id = self.doc.add_object(catalog);
        self.doc.trailer.set("Root", Object::Reference(catalog_id));

        self.doc
    }

    fn write_text_at(&mut self, text: &str, font: BuiltinFont, size: f32, x: Mm, y: Mm) {
        self.write_text_at_with_color(text, font, size, x, y, None);
    }

    fn write_text_at_with_color(
        &mut self,
        text: &str,
        font: BuiltinFont,
        size: f32,
        x: Mm,
        y: Mm,
        color_override: Option<(f32, f32, f32)>,
    ) {
        self.end_text_section();
        self.start_text_section();

        let font_key = self.ensure_font(font);

        // Set text position
        self.current_ops.push(Operation::new(
            "Td",
            vec![x.to_points().into(), y.to_points().into()],
        ));

        // Set text color from theme or override
        let color = color_override.unwrap_or({
            if self.is_slide {
                self.slide_theme.text_color
            } else {
                (0.0, 0.0, 0.0) // Black for regular PDFs
            }
        });
        self.current_ops.push(Operation::new(
            "rg",
            vec![color.0.into(), color.1.into(), color.2.into()],
        ));

        // Set font and size
        self.current_ops
            .push(Operation::new("Tf", vec![font_key.into(), size.into()]));

        // Write text
        self.current_ops
            .push(Operation::new("Tj", vec![Object::string_literal(text)]));

        self.end_text_section();
    }

    fn draw_checkbox(&mut self, x: Mm, y: Mm, checked: bool) {
        self.end_text_section();

        let box_size = Mm(3.5);

        // Draw box outline
        self.current_ops.push(Operation::new(
            "q", // Save graphics state
            vec![],
        ));

        // Set line width
        self.current_ops.push(Operation::new("w", vec![0.5.into()]));

        // Set stroke color from theme
        let color = if self.is_slide {
            self.slide_theme.text_color
        } else {
            (0.0, 0.0, 0.0)
        };
        self.current_ops.push(Operation::new(
            "RG",
            vec![color.0.into(), color.1.into(), color.2.into()],
        ));

        // Draw rectangle
        self.current_ops.push(Operation::new(
            "re",
            vec![
                x.to_points().into(),
                y.to_points().into(),
                box_size.to_points().into(),
                box_size.to_points().into(),
            ],
        ));

        self.current_ops.push(Operation::new("S", vec![])); // Stroke

        if checked {
            // Draw checkmark (X shape)
            let padding = Mm(0.7);
            let x1 = x + padding;
            let y1 = y + padding;
            let x2 = x + box_size - padding;
            let y2 = y + box_size - padding;

            // First diagonal line
            self.current_ops.push(Operation::new(
                "m",
                vec![x1.to_points().into(), y1.to_points().into()],
            ));
            self.current_ops.push(Operation::new(
                "l",
                vec![x2.to_points().into(), y2.to_points().into()],
            ));
            self.current_ops.push(Operation::new("S", vec![]));

            // Second diagonal line
            self.current_ops.push(Operation::new(
                "m",
                vec![x2.to_points().into(), y1.to_points().into()],
            ));
            self.current_ops.push(Operation::new(
                "l",
                vec![x1.to_points().into(), y2.to_points().into()],
            ));
            self.current_ops.push(Operation::new("S", vec![]));
        }

        self.current_ops.push(Operation::new(
            "Q", // Restore graphics state
            vec![],
        ));
    }

    fn move_down(&mut self, amount: Mm) {
        self.y_position -= amount;
    }

    /// Render wrapped text in a table cell and return the height used
    fn write_wrapped_cell(&mut self, words: &[Word], x: Mm, size: f32, column_width: Mm) -> Mm {
        if words.is_empty() {
            return Mm(0.0);
        }

        use crate::layout::find_line_breaks;

        let ideal_width = column_width * 0.95;
        let breaks = find_line_breaks(words, ideal_width.0, column_width.0);

        let mut line_start = 0;
        let mut break_indices = breaks.clone();
        break_indices.push(words.len());

        let start_y = self.y_position;

        for &break_idx in &break_indices {
            let line_words = &words[line_start..break_idx];

            if line_words.is_empty() {
                continue;
            }

            self.end_text_section();
            self.start_text_section();

            self.current_ops.push(Operation::new(
                "Td",
                vec![x.to_points().into(), self.y_position.to_points().into()],
            ));

            // Set text color from theme
            let color = if self.is_slide {
                self.slide_theme.text_color
            } else {
                (0.0, 0.0, 0.0)
            };
            self.current_ops.push(Operation::new(
                "rg",
                vec![color.0.into(), color.1.into(), color.2.into()],
            ));

            for (idx, word) in line_words.iter().enumerate() {
                let font = word.segment_type.as_font();
                let font_key = self.ensure_font(font);

                self.current_ops
                    .push(Operation::new("Tf", vec![font_key.into(), size.into()]));

                self.current_ops.push(Operation::new(
                    "Tj",
                    vec![Object::string_literal(word.text.as_str())],
                ));

                if idx < line_words.len() - 1 {
                    self.current_ops
                        .push(Operation::new("Tj", vec![Object::string_literal(" ")]));
                }
            }

            self.end_text_section();
            self.move_down(self.line_height * 0.8);
            line_start = break_idx;
        }

        start_y - self.y_position
    }

    /// Render wrapped text using Knuth-Plass line breaking
    fn write_wrapped_text(&mut self, words: &[Word], x: Mm, size: f32) {
        if words.is_empty() {
            return;
        }

        use crate::layout::find_line_breaks;

        let max_width = self.right_margin - x;
        let ideal_width = max_width * 0.95;

        let breaks = find_line_breaks(words, ideal_width.0, max_width.0);

        let mut line_start = 0;
        let mut break_indices = breaks.clone();
        break_indices.push(words.len());

        for &break_idx in &break_indices {
            let line_words = &words[line_start..break_idx];

            if line_words.is_empty() {
                continue;
            }

            self.check_page_break(self.line_height);

            self.end_text_section();
            self.start_text_section();

            self.current_ops.push(Operation::new(
                "Td",
                vec![x.to_points().into(), self.y_position.to_points().into()],
            ));

            // Set text color from theme
            let color = if self.is_slide {
                self.slide_theme.text_color
            } else {
                (0.0, 0.0, 0.0)
            };
            self.current_ops.push(Operation::new(
                "rg",
                vec![color.0.into(), color.1.into(), color.2.into()],
            ));

            for (idx, word) in line_words.iter().enumerate() {
                let font = word.segment_type.as_font();
                let font_key = self.ensure_font(font);

                self.current_ops
                    .push(Operation::new("Tf", vec![font_key.into(), size.into()]));

                self.current_ops.push(Operation::new(
                    "Tj",
                    vec![Object::string_literal(word.text.as_str())],
                ));

                if idx < line_words.len() - 1 {
                    self.current_ops
                        .push(Operation::new("Tj", vec![Object::string_literal(" ")]));
                }
            }

            self.end_text_section();

            self.move_down(self.line_height);
            line_start = break_idx;
        }
    }
}

/// Text segment with different formatting types
#[derive(Clone, Debug)]
enum TextSegment {
    Normal(String),
    Bold(String),
    Italic(String),
    BoldItalic(String),
    Code(String),
}

impl TextSegment {
    fn new(text_buffer: String, in_strong: bool, in_emphasis: bool) -> TextSegment {
        match (in_strong, in_emphasis) {
            (false, false) => TextSegment::Normal(text_buffer),
            (false, true) => TextSegment::Italic(text_buffer),
            (true, false) => TextSegment::Bold(text_buffer),
            (true, true) => TextSegment::BoldItalic(text_buffer),
        }
    }

    fn as_parts(&self) -> (&str, TextSegmentType) {
        match self {
            TextSegment::Normal(s) => (s.as_str(), TextSegmentType::Normal),
            TextSegment::Bold(s) => (s.as_str(), TextSegmentType::Bold),
            TextSegment::Italic(s) => (s.as_str(), TextSegmentType::Italic),
            TextSegment::BoldItalic(s) => (s.as_str(), TextSegmentType::BoldItalic),
            TextSegment::Code(s) => (s.as_str(), TextSegmentType::Code),
        }
    }
}

/// Get the relative width factor for a specific character in a proportional font
fn get_char_relative_width(c: char) -> f32 {
    match c {
        'i' | 'l' | 'I' | '!' | '|' | '.' | ',' | ';' | ':' | '\'' | '`' => 0.5,
        'j' | 'f' | 't' | 'r' | 'J' | '(' | ')' | '[' | ']' | '{' | '}' | '"' => 0.7,
        'm' | 'w' => 1.3,
        'M' | 'W' => 1.4,
        'A' | 'C' | 'D' | 'G' | 'H' | 'N' | 'O' | 'Q' | 'U' | 'V' | 'X' | 'Y' | 'Z' => 1.1,
        '0' => 1.1,
        _ => 1.0,
    }
}

/// Calculate approximate text width in millimeters for a given font and size
fn calculate_text_width(text: &str, font: BuiltinFont, size: f32) -> Mm {
    let base_width_factor = match font {
        BuiltinFont::Courier => {
            return Mm(text.len() as f32 * size * 0.6 / 2.83465);
        }
        BuiltinFont::Helvetica | BuiltinFont::HelveticaOblique => 0.52,
        BuiltinFont::HelveticaBold | BuiltinFont::HelveticaBoldOblique => 0.55,
    };

    let total_width: f32 = text.chars().map(get_char_relative_width).sum();
    Mm(total_width * size * base_width_factor / 2.83465)
}

/// A word with formatting information for layout
#[derive(Clone, Debug)]
struct Word {
    text: String,
    segment_type: TextSegmentType,
    width: Mm,
}

/// Type of text segment (without the content)
#[derive(Copy, Clone, Debug, PartialEq)]
enum TextSegmentType {
    Normal,
    Bold,
    Italic,
    BoldItalic,
    Code,
}

impl TextSegmentType {
    fn as_font(&self) -> BuiltinFont {
        match self {
            TextSegmentType::Normal => BuiltinFont::Helvetica,
            TextSegmentType::Bold => BuiltinFont::HelveticaBold,
            TextSegmentType::Italic => BuiltinFont::HelveticaOblique,
            TextSegmentType::BoldItalic => BuiltinFont::HelveticaBoldOblique,
            TextSegmentType::Code => BuiltinFont::Courier,
        }
    }
}

impl Word {
    fn new(text: String, segment_type: TextSegmentType, font_size: f32) -> Self {
        let font = segment_type.as_font();
        let width = calculate_text_width(&text, font, font_size);
        Self {
            text,
            segment_type,
            width,
        }
    }
}

impl LayoutItem for Word {
    fn width(&self) -> f32 {
        self.width.0
    }
}

/// Convert TextSegments into Words for line breaking
fn segments_to_words(segments: &[TextSegment], font_size: f32) -> Vec<Word> {
    let mut words = Vec::new();

    for segment in segments {
        let (text, seg_type) = segment.as_parts();

        for word_text in text.split_whitespace() {
            if !word_text.is_empty() {
                words.push(Word::new(word_text.to_string(), seg_type, font_size));
            }
        }
    }

    words
}

fn embed_file_attachment(doc: &mut Document, content: &str) -> Result<(), std::io::Error> {
    let filename = "source";

    // Create embedded file stream with the markdown content
    let mut file_stream = Stream::new(
        dictionary! {
            "Type" => "EmbeddedFile",
            "Subtype" => "text/markdown",
        },
        content.as_bytes().to_vec(),
    );
    let _ = file_stream.compress();
    let file_stream_id = doc.add_object(file_stream);

    // Create FileSpec dictionary
    let filespec = dictionary! {
        "Type" => "Filespec",
        "F" => Object::String(filename.as_bytes().to_vec(), lopdf::StringFormat::Literal),
        "UF" => Object::String(filename.as_bytes().to_vec(), lopdf::StringFormat::Literal),
        "EF" => dictionary! {
            "F" => Object::Reference(file_stream_id),
        },
    };
    let filespec_id = doc.add_object(filespec);

    // Create the EmbeddedFiles name tree dictionary
    let embedded_files_dict = dictionary! {
        "Names" => Object::Array(vec![
            Object::String(filename.as_bytes().to_vec(), lopdf::StringFormat::Literal),
            Object::Reference(filespec_id),
        ]),
    };
    let embedded_files_id = doc.add_object(embedded_files_dict);

    // Create the Names dictionary for the catalog
    let catalog_names_dict = dictionary! {
        "EmbeddedFiles" => Object::Reference(embedded_files_id),
    };
    let catalog_names_id = doc.add_object(catalog_names_dict);

    // Get the catalog object and update it with the Names dictionary
    let catalog = doc
        .catalog_mut()
        .map_err(|e| std::io::Error::other(format!("Failed to get catalog: {}", e)))?;

    catalog.set("Names", Object::Reference(catalog_names_id));

    Ok(())
}

pub fn to_pdf<W: std::io::Write>(
    markdown_content: &str,
    mut output: W,
    is_slide: bool,
    theme_override: Option<&str>,
    embed_source: bool,
    _source_path: Option<&std::path::Path>,
) -> Result<(), std::io::Error> {
    let parser = MarkdownParser::new(markdown_content).unwrap();
    let front_matter: Option<&FrontMatter> = parser.front_matter();

    // Initialize syntax highlighting
    let syntax_set = SyntaxSet::load_defaults_newlines();
    let theme_set = ThemeSet::load_defaults();

    // Determine code syntax highlighting theme
    let code_theme_name = theme_override
        .or_else(|| front_matter.and_then(|fm| fm.code_theme.as_deref()))
        .unwrap_or("InspiredGitHub");
    let theme = theme_set
        .themes
        .get(code_theme_name)
        .unwrap_or(&theme_set.themes["InspiredGitHub"]);

    // Determine slide theme (only for slide mode)
    let slide_theme = if is_slide {
        let slide_theme_name = front_matter
            .and_then(|fm| fm.slide_theme.as_deref())
            .unwrap_or("light");
        let mut theme = SlideTheme::get_by_name(slide_theme_name);

        // Apply custom gradient direction if specified
        if let Some(direction_str) = front_matter.and_then(|fm| fm.gradient_direction.as_ref()) {
            let direction = match direction_str.as_str() {
                "top-to-bottom" => GradientDirection::TopToBottom,
                "bottom-to-top" => GradientDirection::BottomToTop,
                "left-to-right" => GradientDirection::LeftToRight,
                "right-to-left" => GradientDirection::RightToLeft,
                "top-left-to-bottom-right" | "diagonal" => GradientDirection::TopLeftToBottomRight,
                "top-right-to-bottom-left" => GradientDirection::TopRightToBottomLeft,
                "bottom-left-to-top-right" => GradientDirection::BottomLeftToTopRight,
                "bottom-right-to-top-left" => GradientDirection::BottomRightToTopLeft,
                _ => GradientDirection::TopToBottom,
            };
            theme = theme.with_direction(direction);
        }

        theme
    } else {
        SlideTheme::default()
    };

    let mut builder = if is_slide {
        PdfBuilder::new_slide("", slide_theme)
    } else {
        PdfBuilder::new("", slide_theme)
    };

    // Draw background for first page in slide mode
    if is_slide {
        builder.draw_background();
    }

    // Render front matter if present
    if let Some(fm) = front_matter {
        if let Some(doc_title) = &fm.title {
            builder.check_page_break(Mm(15.0));
            builder.write_text_at(
                doc_title,
                BuiltinFont::HelveticaBold,
                28.0,
                builder.left_margin,
                builder.y_position,
            );
            builder.move_down(builder.line_height * 2.5);
        }

        if let Some(author) = &fm.author {
            builder.check_page_break(Mm(10.0));
            let author_text = format!("By {}", author);
            builder.write_text_at(
                &author_text,
                BuiltinFont::Helvetica,
                14.0,
                builder.left_margin,
                builder.y_position,
            );
            builder.move_down(builder.line_height * 1.2);
        }

        if let Some(date) = &fm.date {
            builder.check_page_break(Mm(10.0));
            let date_text = format!("Date: {}", date);
            builder.write_text_at(
                &date_text,
                BuiltinFont::Helvetica,
                14.0,
                builder.left_margin,
                builder.y_position,
            );
            builder.move_down(builder.line_height * 1.5);
        }

        builder.move_down(builder.line_height);
    }

    #[derive(Default)]
    struct State {
        text_buffer: String,
        text_segments: Vec<TextSegment>,
        current_cell_segments: Vec<TextSegment>,
        in_strong: bool,
        in_emphasis: bool,
        in_table: bool,
        in_code_block: bool,
        in_table_head: bool,
        task_list_marker: Option<bool>,
        list_depth: usize,
        item_depth: usize,
        prev_heading_level: Option<u8>,
    }

    impl State {
        fn clear(&mut self) {
            self.text_buffer.clear();
            self.text_segments.clear();
        }

        fn flush(&mut self) {
            if !self.text_buffer.is_empty() {
                let segment = TextSegment::new(
                    std::mem::take(&mut self.text_buffer),
                    self.in_strong,
                    self.in_emphasis,
                );
                if self.in_table {
                    self.current_cell_segments.push(segment);
                } else {
                    self.text_segments.push(segment);
                }
            }
        }
    }

    let mut state = State::default();

    let mut heading_level = 0u8;
    let mut code_buffer = String::new();
    let mut code_lang = String::new();
    let mut table_rows: Vec<Vec<Vec<TextSegment>>> = Vec::new();
    let mut current_row: Vec<Vec<TextSegment>> = Vec::new();

    for event in parser.into_inner() {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                heading_level = level as u8;
                state.text_buffer.clear();
            }
            Event::End(TagEnd::Heading(_)) => {
                if !state.text_buffer.is_empty() {
                    // In slide mode: check if we need a page break
                    if builder.is_slide {
                        // Auto page break before h2
                        if heading_level == 2 {
                            builder.new_page();
                        }
                        // Page break when going from lower level to higher level heading
                        // (e.g., h3 to h1, or h4 to h2)
                        else if let Some(prev_level) = state.prev_heading_level
                            && heading_level < prev_level
                        {
                            builder.new_page();
                        }
                    }

                    let font_size = match heading_level {
                        1 => 24.0,
                        2 => 20.0,
                        3 => 16.0,
                        _ => 14.0,
                    };

                    let spacing_before = match heading_level {
                        1 => builder.line_height * 1.5,
                        2 => builder.line_height * 1.25,
                        _ => builder.line_height * 1.0,
                    };

                    let spacing_after = match heading_level {
                        1 => builder.line_height * 1.5,
                        2 => builder.line_height * 1.25,
                        3 => builder.line_height * 1.5,
                        _ => builder.line_height * 1.0,
                    };

                    builder.move_down(spacing_before);
                    builder.check_page_break(Mm(font_size * 0.5));

                    // Use heading color for slide mode
                    let heading_color = if builder.is_slide {
                        Some(builder.slide_theme.heading_color)
                    } else {
                        None
                    };

                    builder.write_text_at_with_color(
                        &state.text_buffer,
                        BuiltinFont::HelveticaBold,
                        font_size,
                        builder.left_margin,
                        builder.y_position,
                        heading_color,
                    );
                    builder.move_down(spacing_after);
                    state.text_buffer.clear();

                    // Update the previous heading level
                    state.prev_heading_level = Some(heading_level);
                }
            }
            Event::Start(Tag::Paragraph) => {
                builder.move_down(builder.line_height * 0.5);
                state.clear();
            }
            Event::End(TagEnd::Paragraph) => {
                state.flush();

                if !state.text_segments.is_empty() {
                    let words = segments_to_words(&state.text_segments, 12.0);
                    builder.write_wrapped_text(&words, builder.left_margin, 12.0);
                    builder.move_down(builder.line_height * 0.5);
                    state.text_segments.clear();
                }
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                state.in_code_block = true;
                code_buffer.clear();
                code_lang = match kind {
                    CodeBlockKind::Fenced(lang) => lang.to_string(),
                    CodeBlockKind::Indented => String::new(),
                };
            }
            Event::End(TagEnd::CodeBlock) => {
                if !code_buffer.is_empty() {
                    builder.move_down(builder.line_height * 0.5);

                    let code_info: CodeBlockInfo = code_lang.parse().unwrap();

                    if let Some(filename) = code_info.filename {
                        builder.check_page_break(builder.line_height * 2.0);
                        builder.write_text_at(
                            &filename,
                            BuiltinFont::Courier,
                            10.0,
                            builder.left_margin + Mm(5.0),
                            builder.y_position,
                        );
                        builder.move_down(builder.line_height * 1.5);
                    }

                    let syntax = syntax_set
                        .find_syntax_by_token(&code_info.language)
                        .unwrap_or_else(|| syntax_set.find_syntax_plain_text());

                    let mut highlighter = HighlightLines::new(syntax, theme);

                    for line in code_buffer.lines() {
                        builder.check_page_break(builder.line_height);

                        let highlighted = highlighter
                            .highlight_line(line, &syntax_set)
                            .unwrap_or_else(|_| vec![]);

                        builder.end_text_section();
                        builder.start_text_section();

                        builder.current_ops.push(Operation::new(
                            "Td",
                            vec![
                                (builder.left_margin + Mm(5.0)).to_points().into(),
                                builder.y_position.to_points().into(),
                            ],
                        ));

                        let courier_key = builder.ensure_font(BuiltinFont::Courier);

                        for (style, text) in highlighted {
                            let fg = style.foreground;
                            builder.current_ops.push(Operation::new(
                                "rg",
                                vec![
                                    (fg.r as f32 / 255.0).into(),
                                    (fg.g as f32 / 255.0).into(),
                                    (fg.b as f32 / 255.0).into(),
                                ],
                            ));

                            builder.current_ops.push(Operation::new(
                                "Tf",
                                vec![courier_key.clone().into(), 10.0.into()],
                            ));

                            builder
                                .current_ops
                                .push(Operation::new("Tj", vec![Object::string_literal(text)]));
                        }

                        builder.end_text_section();
                        builder.move_down(builder.line_height * 0.8);
                    }

                    builder.move_down(builder.line_height * 0.75);
                    code_buffer.clear();
                }
                state.in_code_block = false;
            }
            Event::Start(Tag::List(_)) => {
                state.list_depth += 1;
                if state.list_depth == 1 {
                    builder.move_down(builder.line_height * 0.5);
                } else if state.list_depth == 2 && state.item_depth == 1 {
                    // Nested list within an item - render parent item's text first
                    state.flush();
                    if !state.text_segments.is_empty() {
                        builder.check_page_break(builder.line_height * 1.5);

                        let indent = builder.left_margin + Mm(5.0);
                        let text_indent = indent + Mm(6.0);

                        match state.task_list_marker {
                            Some(checked) => {
                                builder.draw_checkbox(
                                    indent,
                                    builder.y_position - Mm(0.4),
                                    checked,
                                );
                            }
                            None => {
                                builder.write_text_at(
                                    "- ",
                                    BuiltinFont::Helvetica,
                                    12.0,
                                    indent,
                                    builder.y_position,
                                );
                            }
                        }

                        let words = segments_to_words(&state.text_segments, 12.0);
                        builder.write_wrapped_text(&words, text_indent, 12.0);
                        state.text_segments.clear();
                    }
                }
            }
            Event::End(TagEnd::List(_)) => {
                if state.list_depth == 1 {
                    builder.move_down(builder.line_height * 0.5);
                }
                state.list_depth = state.list_depth.saturating_sub(1);
            }
            Event::Start(Tag::Item) => {
                state.item_depth += 1;
                state.clear();
                if state.item_depth == 1 {
                    state.task_list_marker = None;
                }
            }
            Event::TaskListMarker(checked) => {
                if state.item_depth == 1 {
                    state.task_list_marker = Some(checked);
                }
            }
            Event::End(TagEnd::Item) => {
                state.flush();

                // Only render if: (1) at depth 1, or (2) at depth > 1 with non-empty text
                // (depth 1 items with nested lists already rendered their text at Start(List))
                let should_render = if state.item_depth == 1 && state.list_depth > 1 {
                    // Parent item with nested list - already rendered
                    false
                } else {
                    !state.text_segments.is_empty()
                };

                if should_render {
                    builder.check_page_break(builder.line_height * 1.5);

                    // Calculate indentation based on depth
                    let indent =
                        builder.left_margin + Mm(5.0) + Mm(5.0 * (state.item_depth - 1) as f32);
                    let text_indent = indent + Mm(6.0);

                    // Only use task list marker at depth 1
                    if state.item_depth == 1 && state.task_list_marker.is_some() {
                        let checked = state.task_list_marker.unwrap();
                        builder.draw_checkbox(indent, builder.y_position - Mm(0.4), checked);
                    } else {
                        builder.write_text_at(
                            "- ",
                            BuiltinFont::Helvetica,
                            12.0,
                            indent,
                            builder.y_position,
                        );
                    }

                    let words = segments_to_words(&state.text_segments, 12.0);
                    builder.write_wrapped_text(&words, text_indent, 12.0);
                }

                state.text_segments.clear();
                if state.item_depth == 1 {
                    state.task_list_marker = None;
                }

                state.item_depth = state.item_depth.saturating_sub(1);
            }
            Event::Start(Tag::Strong) => {
                state.flush();
                state.in_strong = true;
            }
            Event::End(TagEnd::Strong) => {
                state.flush();
                state.in_strong = false;
            }
            Event::Start(Tag::Emphasis) => {
                state.flush();
                state.in_emphasis = true;
            }
            Event::End(TagEnd::Emphasis) => {
                state.flush();
                state.in_emphasis = false;
            }
            Event::Text(text) => {
                if state.in_code_block {
                    code_buffer.push_str(&text);
                } else {
                    state.text_buffer.push_str(&text);
                }
            }
            Event::Code(code) => {
                if !state.in_code_block {
                    if !state.text_buffer.is_empty() {
                        if state.in_table {
                            state
                                .current_cell_segments
                                .push(TextSegment::Normal(std::mem::take(&mut state.text_buffer)));
                        } else {
                            state
                                .text_segments
                                .push(TextSegment::Normal(std::mem::take(&mut state.text_buffer)));
                        }
                    }

                    if state.in_table {
                        state
                            .current_cell_segments
                            .push(TextSegment::Code(code.to_string()));
                    } else {
                        state
                            .text_segments
                            .push(TextSegment::Code(code.to_string()));
                    }
                }
            }
            Event::Start(Tag::Table(_)) => {
                state.in_table = true;
                table_rows.clear();
            }
            Event::End(TagEnd::Table) => {
                if !table_rows.is_empty() {
                    let num_cols = table_rows.iter().map(|row| row.len()).max().unwrap_or(0);
                    let mut col_widths = vec![0; num_cols];

                    for row in table_rows.iter() {
                        for (col_idx, cell) in row.iter().enumerate() {
                            let weighted_chars: usize = cell
                                .iter()
                                .map(|seg| match seg {
                                    TextSegment::Normal(t)
                                    | TextSegment::Bold(t)
                                    | TextSegment::Italic(t)
                                    | TextSegment::BoldItalic(t) => t.len(),
                                    TextSegment::Code(t) => (t.len() as f32 * 1.5) as usize,
                                })
                                .sum();
                            col_widths[col_idx] = col_widths[col_idx].max(weighted_chars);
                        }
                    }

                    let available_width = builder.right_margin - builder.left_margin - Mm(10.0);
                    let column_spacing = Mm(5.0);
                    let total_spacing = column_spacing * (num_cols - 1) as f32;
                    let usable_width = available_width - total_spacing;

                    let total_chars: usize = col_widths.iter().sum();
                    let column_widths: Vec<Mm> = if total_chars > 0 {
                        col_widths
                            .iter()
                            .map(|&chars| Mm((chars as f32 / total_chars as f32) * usable_width.0))
                            .collect()
                    } else {
                        vec![usable_width / num_cols as f32; num_cols]
                    };

                    for row in table_rows.iter() {
                        builder.check_page_break(builder.line_height * 1.5);

                        let row_start_y = builder.y_position;
                        let mut max_cell_height = Mm(0.0);

                        let mut x_offset = builder.left_margin + Mm(5.0);
                        for (col_idx, cell_segments) in row.iter().enumerate() {
                            builder.y_position = row_start_y;

                            let words = segments_to_words(cell_segments, 10.0);
                            let col_width = column_widths.get(col_idx).copied().unwrap_or(Mm(50.0));

                            let cell_height =
                                builder.write_wrapped_cell(&words, x_offset, 10.0, col_width);
                            max_cell_height = Mm(max_cell_height.0.max(cell_height.0));

                            x_offset += col_width + column_spacing;
                        }

                        builder.y_position = row_start_y - max_cell_height;
                    }

                    builder.move_down(builder.line_height * 0.5);
                    table_rows.clear();
                }
                state.in_table = false;
            }
            Event::Start(Tag::TableHead) => {
                state.in_table_head = true;
            }
            Event::End(TagEnd::TableHead) => {
                state.in_table_head = false;
            }
            Event::Start(Tag::TableRow) => {
                current_row.clear();
            }
            Event::End(TagEnd::TableRow) => {
                if !current_row.is_empty() {
                    table_rows.push(current_row.clone());
                    current_row.clear();
                }
            }
            Event::Start(Tag::TableCell) => {
                state.clear();
            }
            Event::End(TagEnd::TableCell) => {
                state.flush();
                current_row.push(std::mem::take(&mut state.current_cell_segments));
            }
            Event::SoftBreak | Event::HardBreak => {
                if state.in_code_block {
                    code_buffer.push('\n');
                } else {
                    state.text_buffer.push(' ');
                }
            }
            _ => {}
        }
    }

    let mut doc = builder.finalize();

    // Embed source markdown file if requested
    if embed_source {
        embed_file_attachment(&mut doc, markdown_content)?;
    }

    doc.save_to(&mut output)
        .map_err(|e| std::io::Error::other(format!("PDF save error: {}", e)))?;

    Ok(())
}

/// Extract embedded markdown from PDF bytes
pub fn extract_markdown_from_pdf_bytes(pdf_bytes: &[u8]) -> Result<String, std::io::Error> {
    // Load the PDF document from bytes using a cursor
    let cursor = Cursor::new(pdf_bytes);
    let doc = Document::load_from(cursor)
        .map_err(|e| std::io::Error::other(format!("Failed to load PDF: {}", e)))?;

    // Get the catalog
    let catalog = doc
        .catalog()
        .map_err(|e| std::io::Error::other(format!("Failed to get catalog: {}", e)))?;

    // Extract Names dictionary from catalog
    let names_ref = catalog
        .get(b"Names")
        .map_err(|e| std::io::Error::other(format!("No Names dictionary in catalog: {}", e)))?;

    // Resolve the Names dictionary reference
    let names_id = if let Object::Reference(id) = names_ref {
        *id
    } else {
        return Err(std::io::Error::other("Names is not a reference"));
    };

    let names_obj = doc
        .get_object(names_id)
        .map_err(|e| std::io::Error::other(format!("Failed to get Names object: {}", e)))?;

    // Get EmbeddedFiles from Names
    let embedded_files_ref = if let Object::Dictionary(dict) = names_obj {
        dict.get(b"EmbeddedFiles").map_err(|e| {
            std::io::Error::other(format!("No EmbeddedFiles in Names dictionary: {}", e))
        })?
    } else {
        return Err(std::io::Error::other("Names object is not a dictionary"));
    };

    // Resolve EmbeddedFiles reference
    let embedded_files_id = if let Object::Reference(id) = embedded_files_ref {
        *id
    } else {
        return Err(std::io::Error::other("EmbeddedFiles is not a reference"));
    };

    let embedded_files_obj = doc
        .get_object(embedded_files_id)
        .map_err(|e| std::io::Error::other(format!("Failed to get EmbeddedFiles object: {}", e)))?;

    // Get the Names array from EmbeddedFiles
    let names_array = if let Object::Dictionary(dict) = embedded_files_obj {
        dict.get(b"Names")
            .map_err(|e| std::io::Error::other(format!("No Names array in EmbeddedFiles: {}", e)))?
    } else {
        return Err(std::io::Error::other(
            "EmbeddedFiles object is not a dictionary",
        ));
    };

    // Parse the Names array to find the filespec
    let filespec_id = if let Object::Array(arr) = names_array {
        // Names array is in format: [name1, ref1, name2, ref2, ...]
        // We're looking for the "source" file
        let mut found_id = None;
        for i in (0..arr.len()).step_by(2) {
            if let Some(Object::String(name_bytes, _)) = arr.get(i) {
                let name = String::from_utf8_lossy(name_bytes);
                if name == "source" {
                    if let Some(Object::Reference(id)) = arr.get(i + 1) {
                        found_id = Some(*id);
                        break;
                    }
                }
            }
        }
        found_id.ok_or_else(|| std::io::Error::other("Source file not found in embedded files"))?
    } else {
        return Err(std::io::Error::other("Names is not an array"));
    };

    // Get the filespec object
    let filespec_obj = doc
        .get_object(filespec_id)
        .map_err(|e| std::io::Error::other(format!("Failed to get filespec object: {}", e)))?;

    // Get the EF (embedded file) dictionary from filespec
    let ef_ref = if let Object::Dictionary(dict) = filespec_obj {
        dict.get(b"EF")
            .map_err(|e| std::io::Error::other(format!("No EF dictionary in filespec: {}", e)))?
    } else {
        return Err(std::io::Error::other("Filespec is not a dictionary"));
    };

    // Get the F (file) reference from EF
    let file_stream_id = if let Object::Dictionary(ef_dict) = ef_ref {
        if let Object::Reference(id) = ef_dict
            .get(b"F")
            .map_err(|e| std::io::Error::other(format!("No F reference in EF dictionary: {}", e)))?
        {
            *id
        } else {
            return Err(std::io::Error::other("F is not a reference"));
        }
    } else {
        return Err(std::io::Error::other("EF is not a dictionary"));
    };

    // Get the embedded file stream
    let file_stream_obj = doc
        .get_object(file_stream_id)
        .map_err(|e| std::io::Error::other(format!("Failed to get file stream object: {}", e)))?;

    // Extract stream data (try decompression first, fall back to raw content)
    let content = if let Object::Stream(stream) = file_stream_obj {
        // Try to decompress if the stream has a Filter
        if stream.dict.get(b"Filter").is_ok() {
            stream
                .decompressed_content()
                .map_err(|e| std::io::Error::other(format!("Failed to decompress stream: {}", e)))?
        } else {
            // No filter, use raw content
            stream.content.clone()
        }
    } else {
        return Err(std::io::Error::other("Embedded file is not a stream"));
    };

    // Convert bytes to string
    String::from_utf8(content)
        .map_err(|e| std::io::Error::other(format!("Failed to convert to UTF-8: {}", e)))
}

/// Extract embedded markdown from a PDF file
pub fn extract_markdown_from_pdf(pdf_path: &Path) -> Result<String, std::io::Error> {
    // Read the PDF file into memory
    let pdf_bytes = std::fs::read(pdf_path)?;
    // Use the bytes-based extraction
    extract_markdown_from_pdf_bytes(&pdf_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that simple markdown can be embedded and extracted
    #[test]
    fn test_roundtrip_simple_markdown() {
        let markdown = "# Hello World\n\nThis is a test.";
        let mut pdf_output = Vec::new();

        // Generate PDF with embedded source
        to_pdf(markdown, &mut pdf_output, false, None, true, None).unwrap();

        // Extract the markdown back
        let extracted = extract_markdown_from_pdf_bytes(&pdf_output).unwrap();

        assert_eq!(markdown, extracted);
    }

    /// Test that markdown with front matter can be embedded and extracted
    #[test]
    fn test_roundtrip_with_frontmatter() {
        let markdown = r#"---
title: Test Document
author: Test Author
date: 2025-01-01
code_theme: InspiredGitHub
---

# Main Title

This is the content."#;
        let mut pdf_output = Vec::new();

        to_pdf(markdown, &mut pdf_output, false, None, true, None).unwrap();
        let extracted = extract_markdown_from_pdf_bytes(&pdf_output).unwrap();

        assert_eq!(markdown, extracted);
    }

    /// Test that complex markdown with code blocks can be embedded and extracted
    #[test]
    fn test_roundtrip_with_code_blocks() {
        let markdown = r#"# Code Example

Here's some Rust code:

```rust
fn main() {
    println!("Hello, world!");
}
```

And some Python:

```python
def hello():
    print("Hello, world!")
```"#;
        let mut pdf_output = Vec::new();

        to_pdf(markdown, &mut pdf_output, false, None, true, None).unwrap();
        let extracted = extract_markdown_from_pdf_bytes(&pdf_output).unwrap();

        assert_eq!(markdown, extracted);
    }

    /// Test that markdown with lists and formatting can be embedded and extracted
    #[test]
    fn test_roundtrip_with_lists_and_formatting() {
        let markdown = r#"# Features

## Unordered List
- Item 1
- Item 2
  - Nested item
- Item 3

## Ordered List
1. First
2. Second
3. Third

## Formatting
This has **bold**, *italic*, and `code` text.

## Task List
- [x] Completed task
- [ ] Incomplete task"#;
        let mut pdf_output = Vec::new();

        to_pdf(markdown, &mut pdf_output, false, None, true, None).unwrap();
        let extracted = extract_markdown_from_pdf_bytes(&pdf_output).unwrap();

        assert_eq!(markdown, extracted);
    }

    /// Test that markdown with tables can be embedded and extracted
    #[test]
    fn test_roundtrip_with_tables() {
        let markdown = r#"# Table Example

| Column 1 | Column 2 | Column 3 |
|----------|----------|----------|
| A        | B        | C        |
| D        | E        | F        |"#;
        let mut pdf_output = Vec::new();

        to_pdf(markdown, &mut pdf_output, false, None, true, None).unwrap();
        let extracted = extract_markdown_from_pdf_bytes(&pdf_output).unwrap();

        assert_eq!(markdown, extracted);
    }

    /// Test that special characters and unicode are preserved
    #[test]
    fn test_roundtrip_with_unicode() {
        let markdown = r#"# Unicode Test

This has special characters: é, ñ, 中文, 日本語, 한국어

And symbols: © ™ ® → ← ↔ ✓ ✗

Math-like: ∀ ∃ ∈ ∉ ⊂ ⊃ ∪ ∩"#;
        let mut pdf_output = Vec::new();

        to_pdf(markdown, &mut pdf_output, false, None, true, None).unwrap();
        let extracted = extract_markdown_from_pdf_bytes(&pdf_output).unwrap();

        assert_eq!(markdown, extracted);
    }

    /// Test that markdown with links can be embedded and extracted
    #[test]
    fn test_roundtrip_with_links() {
        let markdown = r#"# Links

[Google](https://google.com)

[GitHub](https://github.com)

Reference style: [link][ref]

[ref]: https://example.com"#;
        let mut pdf_output = Vec::new();

        to_pdf(markdown, &mut pdf_output, false, None, true, None).unwrap();
        let extracted = extract_markdown_from_pdf_bytes(&pdf_output).unwrap();

        assert_eq!(markdown, extracted);
    }

    /// Test that when embed_source is false, extraction fails appropriately
    #[test]
    fn test_extraction_fails_when_not_embedded() {
        let markdown = "# Test";
        let mut pdf_output = Vec::new();

        // Generate PDF WITHOUT embedded source
        to_pdf(markdown, &mut pdf_output, false, None, false, None).unwrap();

        // Extraction should fail
        let result = extract_markdown_from_pdf_bytes(&pdf_output);
        assert!(result.is_err());
    }

    /// Test that slide mode PDFs can also embed and extract
    #[test]
    fn test_roundtrip_slide_mode() {
        let markdown = r#"# Slide 1

Content for first slide.

## Slide 2

Content for second slide."#;
        let mut pdf_output = Vec::new();

        // Generate slides with embedded source
        to_pdf(markdown, &mut pdf_output, true, None, true, None).unwrap();
        let extracted = extract_markdown_from_pdf_bytes(&pdf_output).unwrap();

        assert_eq!(markdown, extracted);
    }

    /// Test that very large markdown content can be embedded and extracted
    #[test]
    fn test_roundtrip_large_content() {
        // Generate a large markdown document
        let mut markdown = String::from("# Large Document\n\n");
        for i in 0..100 {
            markdown.push_str(&format!("## Section {}\n\n", i));
            markdown.push_str("Lorem ipsum dolor sit amet, consectetur adipiscing elit. ");
            markdown
                .push_str("Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.\n\n");
            markdown.push_str(&format!(
                "```rust\nfn function_{}() {{\n    println!(\"test\");\n}}\n```\n\n",
                i
            ));
        }

        let mut pdf_output = Vec::new();
        to_pdf(&markdown, &mut pdf_output, false, None, true, None).unwrap();
        let extracted = extract_markdown_from_pdf_bytes(&pdf_output).unwrap();

        assert_eq!(markdown, extracted);
    }
}
