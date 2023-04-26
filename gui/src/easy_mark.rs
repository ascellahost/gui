//! A parser for `EasyMark`: a very simple markup language.
//!
//! WARNING: `EasyMark` is subject to change.
//
//! # `EasyMark` design goals:
//! 1. easy to parse
//! 2. easy to learn
//! 3. similar to markdown

//license MIT https://github.com/emilk/egui/blob/master/crates/egui_demo_lib/src/easy_mark/easy_mark_parser.rs

use eframe::egui::*;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Item<'a> {
    /// `\n`
    // TODO(emilk): add Style here so empty heading still uses up the right amount of space.
    Newline,

    ///
    Text(Style, &'a str),

    /// title, url
    Hyperlink(Style, &'a str, &'a str),

    /// leading space before e.g. a [`Self::BulletPoint`].
    Indentation(usize),

    /// >
    QuoteIndent,

    /// - a point well made.
    BulletPoint,

    /// 1. numbered list. The string is the number(s).
    NumberedPoint(&'a str),

    /// ---
    Separator,

    /// language, code
    CodeBlock(&'a str, &'a str),
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct Style {
    /// # heading (large text)
    pub heading: bool,

    /// > quoted (slightly dimmer color or other font style)
    pub quoted: bool,

    /// `code` (monospace, some other color)
    pub code: bool,

    /// self.strong* (emphasized, e.g. bold)
    pub strong: bool,

    /// _underline_
    pub underline: bool,

    /// ~strikethrough~
    pub strikethrough: bool,

    /// /italics/
    pub italics: bool,

    /// $small$
    pub small: bool,

    /// ^raised^
    pub raised: bool,
}

/// Parser for the `EasyMark` markup language.
///
/// See the module-level documentation for details.
///
/// # Example:
/// ```
/// # use egui_demo_lib::parser::Parser;
/// for item in Parser::new("Hello *world*!") {
/// }
/// ```
pub struct Parser<'a> {
    /// The remainder of the input text
    s: &'a str,

    /// Are we at the start of a line?
    start_of_line: bool,

    /// Current self.style. Reset after a newline.
    style: Style,
}

impl<'a> Parser<'a> {
    pub fn new(s: &'a str) -> Self {
        Self {
            s,
            start_of_line: true,
            style: Style::default(),
        }
    }

    /// `1. `, `42. ` etc.
    fn numbered_list(&mut self) -> Option<Item<'a>> {
        let n_digits = self.s.chars().take_while(|c| c.is_ascii_digit()).count();
        if n_digits > 0 && self.s.chars().skip(n_digits).take(2).eq(". ".chars()) {
            let number = &self.s[..n_digits];
            self.s = &self.s[(n_digits + 2)..];
            self.start_of_line = false;
            return Some(Item::NumberedPoint(number));
        }
        None
    }

    // ```{language}\n{code}```
    fn code_block(&mut self) -> Option<Item<'a>> {
        if let Some(language_start) = self.s.strip_prefix("```") {
            if let Some(newline) = language_start.find('\n') {
                let language = &language_start[..newline];
                let code_start = &language_start[newline + 1..];
                if let Some(end) = code_start.find("\n```") {
                    let code = &code_start[..end].trim();
                    self.s = &code_start[end + 4..];
                    self.start_of_line = false;
                    return Some(Item::CodeBlock(language, code));
                } else {
                    self.s = "";
                    return Some(Item::CodeBlock(language, code_start));
                }
            }
        }
        None
    }

    // `code`
    fn inline_code(&mut self) -> Option<Item<'a>> {
        if let Some(rest) = self.s.strip_prefix('`') {
            self.s = rest;
            self.start_of_line = false;
            self.style.code = true;
            let rest_of_line = &self.s[..self.s.find('\n').unwrap_or(self.s.len())];
            if let Some(end) = rest_of_line.find('`') {
                let item = Item::Text(self.style, &self.s[..end]);
                self.s = &self.s[end + 1..];
                self.style.code = false;
                return Some(item);
            } else {
                let end = rest_of_line.len();
                let item = Item::Text(self.style, rest_of_line);
                self.s = &self.s[end..];
                self.style.code = false;
                return Some(item);
            }
        }
        None
    }

    /// `<url>` or `[link](url)`
    fn url(&mut self) -> Option<Item<'a>> {
        if self.s.starts_with('<') {
            let this_line = &self.s[..self.s.find('\n').unwrap_or(self.s.len())];
            if let Some(url_end) = this_line.find('>') {
                let url = &self.s[1..url_end];
                self.s = &self.s[url_end + 1..];
                self.start_of_line = false;
                return Some(Item::Hyperlink(self.style, url, url));
            }
        }

        // [text](url)
        if self.s.starts_with('[') {
            let this_line = &self.s[..self.s.find('\n').unwrap_or(self.s.len())];
            if let Some(bracket_end) = this_line.find(']') {
                let text = &this_line[1..bracket_end];
                if this_line[bracket_end + 1..].starts_with('(') {
                    if let Some(parens_end) = this_line[bracket_end + 2..].find(')') {
                        let parens_end = bracket_end + 2 + parens_end;
                        let url = &self.s[bracket_end + 2..parens_end];
                        self.s = &self.s[parens_end + 1..];
                        self.start_of_line = false;
                        return Some(Item::Hyperlink(self.style, text, url));
                    }
                }
            }
        }
        None
    }
}

impl<'a> Iterator for Parser<'a> {
    type Item = Item<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.s.is_empty() {
                return None;
            }

            // \n
            if self.s.starts_with('\n') {
                self.s = &self.s[1..];
                self.start_of_line = true;
                self.style = Style::default();
                return Some(Item::Newline);
            }

            // Ignore line break (continue on the same line)
            if self.s.starts_with("\\\n") && self.s.len() >= 2 {
                self.s = &self.s[2..];
                self.start_of_line = false;
                continue;
            }

            // \ escape (to show e.g. a backtick)
            if self.s.starts_with('\\') && self.s.len() >= 2 {
                let text = &self.s[1..2];
                self.s = &self.s[2..];
                self.start_of_line = false;
                return Some(Item::Text(self.style, text));
            }

            if self.start_of_line {
                // leading space (indentation)
                if self.s.starts_with(' ') {
                    let length = self.s.find(|c| c != ' ').unwrap_or(self.s.len());
                    self.s = &self.s[length..];
                    self.start_of_line = true; // indentation doesn't count
                    return Some(Item::Indentation(length));
                }

                // # Heading
                if let Some(after) = self.s.strip_prefix("# ") {
                    self.s = after;
                    self.start_of_line = false;
                    self.style.heading = true;
                    continue;
                }

                // > quote
                if let Some(after) = self.s.strip_prefix("> ") {
                    self.s = after;
                    self.start_of_line = true; // quote indentation doesn't count
                    self.style.quoted = true;
                    return Some(Item::QuoteIndent);
                }

                // - bullet point
                if self.s.starts_with("- ") {
                    self.s = &self.s[2..];
                    self.start_of_line = false;
                    return Some(Item::BulletPoint);
                }

                // `1. `, `42. ` etc.
                if let Some(item) = self.numbered_list() {
                    return Some(item);
                }

                // --- separator
                if let Some(after) = self.s.strip_prefix("---") {
                    self.s = after.trim_start_matches('-'); // remove extra dashes
                    self.s = self.s.strip_prefix('\n').unwrap_or(self.s); // remove trailing newline
                    self.start_of_line = false;
                    return Some(Item::Separator);
                }

                // ```{language}\n{code}```
                if let Some(item) = self.code_block() {
                    return Some(item);
                }
            }

            // `code`
            if let Some(item) = self.inline_code() {
                return Some(item);
            }

            if let Some(rest) = self.s.strip_prefix('*') {
                self.s = rest;
                self.start_of_line = false;
                self.style.strong = !self.style.strong;
                continue;
            }
            if let Some(rest) = self.s.strip_prefix('_') {
                self.s = rest;
                self.start_of_line = false;
                self.style.underline = !self.style.underline;
                continue;
            }
            if let Some(rest) = self.s.strip_prefix('~') {
                self.s = rest;
                self.start_of_line = false;
                self.style.strikethrough = !self.style.strikethrough;
                continue;
            }
            if let Some(rest) = self.s.strip_prefix('/') {
                self.s = rest;
                self.start_of_line = false;
                self.style.italics = !self.style.italics;
                continue;
            }
            if let Some(rest) = self.s.strip_prefix('$') {
                self.s = rest;
                self.start_of_line = false;
                self.style.small = !self.style.small;
                continue;
            }
            if let Some(rest) = self.s.strip_prefix('^') {
                self.s = rest;
                self.start_of_line = false;
                self.style.raised = !self.style.raised;
                continue;
            }

            // `<url>` or `[link](url)`
            if let Some(item) = self.url() {
                return Some(item);
            }

            // Swallow everything up to the next special character:
            let end = self
                .s
                .find(&['*', '`', '~', '_', '/', '$', '^', '\\', '<', '[', '\n'][..])
                .map_or_else(|| self.s.len(), |special| special.max(1));

            let item = Item::Text(self.style, &self.s[..end]);
            self.s = &self.s[end..];
            self.start_of_line = false;
            return Some(item);
        }
    }
}

#[test]
fn test_easy_mark_parser() {
    let items: Vec<_> = Parser::new("~strikethrough `code`~").collect();
    assert_eq!(
        items,
        vec![
            Item::Text(
                Style {
                    strikethrough: true,
                    ..Default::default()
                },
                "strikethrough "
            ),
            Item::Text(
                Style {
                    code: true,
                    strikethrough: true,
                    ..Default::default()
                },
                "code"
            ),
        ]
    );
}

// https://github.com/emilk/egui/blob/master/crates/egui_demo_lib/src/easy_mark/easy_mark_viewer.rs

/// Parse and display a VERY simple and small subset of Markdown.
pub fn easy_mark(ui: &mut Ui, easy_mark: &str) {
    easy_mark_it(ui, Parser::new(easy_mark));
}

pub fn easy_mark_it<'em>(ui: &mut Ui, items: impl Iterator<Item = Item<'em>>) {
    let initial_size = vec2(
        ui.available_width(),
        ui.spacing().interact_size.y, // Assume there will be
    );

    let layout = Layout::left_to_right(Align::BOTTOM).with_main_wrap(true);

    ui.allocate_ui_with_layout(initial_size, layout, |ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        let row_height = ui.text_style_height(&TextStyle::Body);
        ui.set_row_height(row_height);

        for item in items {
            item_ui(ui, item);
        }
    });
}

pub fn item_ui(ui: &mut Ui, item: Item<'_>) {
    let row_height = ui.text_style_height(&TextStyle::Body);
    let one_indent = row_height / 2.0;

    match item {
        Item::Newline => {
            // ui.label("\n"); // too much spacing (paragraph spacing)
            ui.allocate_exact_size(vec2(0.0, row_height), Sense::hover()); // make sure we take up some height
            ui.end_row();
            ui.set_row_height(row_height);
        }

        Item::Text(style, text) => {
            let label = rich_text_from_style(text, &style);
            if style.small && !style.raised {
                ui.with_layout(Layout::left_to_right(Align::BOTTOM), |ui| {
                    ui.set_min_height(row_height);
                    ui.label(label);
                });
            } else {
                ui.label(label);
            }
        }
        Item::Hyperlink(style, text, url) => {
            let label = rich_text_from_style(text, &style);
            if style.small && !style.raised {
                ui.with_layout(Layout::left_to_right(Align::BOTTOM), |ui| {
                    ui.set_height(row_height);
                    ui.add(Hyperlink::from_label_and_url(label, url));
                });
            } else {
                ui.add(Hyperlink::from_label_and_url(label, url));
            }
        }

        Item::Separator => {
            ui.add(Separator::default().horizontal());
        }
        Item::Indentation(indent) => {
            let indent = indent as f32 * one_indent;
            ui.allocate_exact_size(vec2(indent, row_height), Sense::hover());
        }
        Item::QuoteIndent => {
            let rect = ui
                .allocate_exact_size(vec2(2.0 * one_indent, row_height), Sense::hover())
                .0;
            let rect = rect.expand2(ui.style().spacing.item_spacing * 0.5);
            ui.painter().line_segment(
                [rect.center_top(), rect.center_bottom()],
                (1.0, ui.visuals().weak_text_color()),
            );
        }
        Item::BulletPoint => {
            ui.allocate_exact_size(vec2(one_indent, row_height), Sense::hover());
            bullet_point(ui, one_indent);
            ui.allocate_exact_size(vec2(one_indent, row_height), Sense::hover());
        }
        Item::NumberedPoint(number) => {
            let width = 3.0 * one_indent;
            numbered_point(ui, width, number);
            ui.allocate_exact_size(vec2(one_indent, row_height), Sense::hover());
        }
        Item::CodeBlock(_language, code) => {
            let where_to_put_background = ui.painter().add(Shape::Noop);
            let mut rect = ui.monospace(code).rect;
            rect = rect.expand(1.0); // looks better
            rect.max.x = ui.max_rect().max.x;
            let code_bg_color = ui.visuals().code_bg_color;
            ui.painter()
                .set(where_to_put_background, Shape::rect_filled(rect, 1.0, code_bg_color));
        }
    };
}

fn rich_text_from_style(text: &str, style: &Style) -> RichText {
    let Style {
        heading,
        quoted,
        code,
        strong,
        underline,
        strikethrough,
        italics,
        small,
        raised,
    } = *style;

    let small = small || raised; // Raised text is also smaller

    let mut rich_text = RichText::new(text);
    if heading && !small {
        rich_text = rich_text.heading().strong();
    }
    if small && !heading {
        rich_text = rich_text.small();
    }
    if code {
        rich_text = rich_text.code();
    }
    if strong {
        rich_text = rich_text.strong();
    } else if quoted {
        rich_text = rich_text.weak();
    }
    if underline {
        rich_text = rich_text.underline();
    }
    if strikethrough {
        rich_text = rich_text.strikethrough();
    }
    if italics {
        rich_text = rich_text.italics();
    }
    if raised {
        rich_text = rich_text.raised();
    }
    rich_text
}

fn bullet_point(ui: &mut Ui, width: f32) -> Response {
    let row_height = ui.text_style_height(&TextStyle::Body);
    let (rect, response) = ui.allocate_exact_size(vec2(width, row_height), Sense::hover());
    ui.painter()
        .circle_filled(rect.center(), rect.height() / 8.0, ui.visuals().strong_text_color());
    response
}

fn numbered_point(ui: &mut Ui, width: f32, number: &str) -> Response {
    let font_id = TextStyle::Body.resolve(ui.style());
    let row_height = ui.fonts(|f| f.row_height(&font_id));
    let (rect, response) = ui.allocate_exact_size(vec2(width, row_height), Sense::hover());
    let text = format!("{}.", number);
    let text_color = ui.visuals().strong_text_color();
    ui.painter()
        .text(rect.right_center(), Align2::RIGHT_CENTER, text, font_id, text_color);
    response
}
