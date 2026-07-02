//! Mini template language for agent-panel sidebar rows.
//!
//! A row template is a sequence of chunks. Each chunk is either literal text or
//! a `{...}` field reference. A field reference has an optional literal prefix
//! and an optional style: `{field}`, `{field:style}`, or `{ literal field}`.
//!
//! A field chunk whose value is empty is dropped entirely, including its
//! literal prefix. That is how `{ · tab}` renders nothing (no dangling
//! separator) when a pane has no tab label.
//!
//! Styling is expressed as `color[+modifier...]`, e.g. `{agent:overlay0}`,
//! `{agent:#f5c2e7}`, or `{space:text+bold}`. A color is either a theme palette
//! token (`text`, `overlay0`, `green`, `accent`, ...) that adapts to the active
//! theme, or a fixed literal color (hex, named, or `rgb(...)`). When a color
//! name matches both, the palette token wins. Without an explicit style each
//! field uses a sensible default derived from the active [`Palette`] and the
//! entry's active state. The parser and renderer are pure and unit-tested
//! without any terminal.

use std::fmt;

use ratatui::{
    style::{Color, Modifier, Style},
    text::Span,
};

use super::text::{display_width, truncate_end};
use crate::app::state::Palette;

/// Fields a row template can reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Field {
    /// Status icon/spinner. Carries the state-derived intrinsic style.
    Icon,
    /// Workspace (space) label. Always present.
    Space,
    /// Tab label. Empty on single-tab workspaces.
    Tab,
    /// Status text (done/idle/working/blocked), honoring custom state labels.
    Status,
    /// Agent name. Empty when no agent is detected.
    Agent,
    /// Custom status string. Empty when none is set.
    Custom,
}

impl Field {
    fn parse(name: &str) -> Option<Self> {
        match name {
            "icon" => Some(Self::Icon),
            "space" => Some(Self::Space),
            "tab" => Some(Self::Tab),
            "status" => Some(Self::Status),
            "agent" => Some(Self::Agent),
            "custom" => Some(Self::Custom),
            _ => None,
        }
    }
}

/// A theme palette token usable as a color in a row style. Tokens resolve
/// against the active [`Palette`] at render time so rows follow theme changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Token {
    Accent,
    PanelBg,
    Surface0,
    Surface1,
    SurfaceDim,
    Overlay0,
    Overlay1,
    Text,
    Subtext0,
    Mauve,
    Green,
    Yellow,
    Red,
    Blue,
    Teal,
    Peach,
}

impl Token {
    fn parse(name: &str) -> Option<Self> {
        let token = match name {
            "accent" => Self::Accent,
            "panel_bg" => Self::PanelBg,
            "surface0" => Self::Surface0,
            "surface1" => Self::Surface1,
            "surface_dim" => Self::SurfaceDim,
            "overlay0" => Self::Overlay0,
            "overlay1" => Self::Overlay1,
            "text" => Self::Text,
            "subtext0" => Self::Subtext0,
            "mauve" => Self::Mauve,
            "green" => Self::Green,
            "yellow" => Self::Yellow,
            "red" => Self::Red,
            "blue" => Self::Blue,
            "teal" => Self::Teal,
            "peach" => Self::Peach,
            _ => return None,
        };
        Some(token)
    }

    fn resolve(self, p: &Palette) -> Color {
        match self {
            Self::Accent => p.accent,
            Self::PanelBg => p.panel_bg,
            Self::Surface0 => p.surface0,
            Self::Surface1 => p.surface1,
            Self::SurfaceDim => p.surface_dim,
            Self::Overlay0 => p.overlay0,
            Self::Overlay1 => p.overlay1,
            Self::Text => p.text,
            Self::Subtext0 => p.subtext0,
            Self::Mauve => p.mauve,
            Self::Green => p.green,
            Self::Yellow => p.yellow,
            Self::Red => p.red,
            Self::Blue => p.blue,
            Self::Teal => p.teal,
            Self::Peach => p.peach,
        }
    }
}

/// A color in a row style: either a theme palette token or a fixed literal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ColorSpec {
    /// A palette token, resolved against the active theme at render time.
    Token(Token),
    /// A fixed literal color (hex, named, or `rgb(...)`).
    Fixed(Color),
}

impl ColorSpec {
    fn resolve(self, p: &Palette) -> Color {
        match self {
            ColorSpec::Token(token) => token.resolve(p),
            ColorSpec::Fixed(color) => color,
        }
    }
}

/// An explicit field style: a foreground color plus optional modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StyleSpec {
    color: ColorSpec,
    modifiers: Modifier,
}

impl StyleSpec {
    /// Parse a `color[+modifier...]` style spec, e.g. `overlay0`, `#f5c2e7`, or
    /// `text+bold`. A color name matching a palette token resolves to the token.
    fn parse(spec: &str, body: &str) -> Result<Self, TemplateError> {
        let mut parts = spec.split('+');
        let color_str = parts.next().unwrap_or("").trim();
        if color_str.is_empty() {
            return Err(TemplateError(format!(
                "missing color in '{{{body}}}' (expected a palette token, hex, named color, or rgb(...))"
            )));
        }

        let lowered = color_str.to_lowercase();
        let color = if let Some(token) = Token::parse(&lowered) {
            ColorSpec::Token(token)
        } else if let Some(color) = crate::config::parse_color_checked(color_str) {
            ColorSpec::Fixed(color)
        } else {
            return Err(TemplateError(format!(
                "unknown color '{color_str}' in '{{{body}}}' (expected a palette token, hex, named color, or rgb(...))"
            )));
        };

        let mut modifiers = Modifier::empty();
        for part in parts {
            let name = part.trim();
            let modifier = match name {
                "bold" => Modifier::BOLD,
                "dim" => Modifier::DIM,
                "italic" => Modifier::ITALIC,
                "underline" => Modifier::UNDERLINED,
                _ => {
                    return Err(TemplateError(format!(
                        "unknown modifier '{name}' in '{{{body}}}' (expected bold, dim, italic, or underline)"
                    )))
                }
            };
            modifiers |= modifier;
        }

        Ok(StyleSpec { color, modifiers })
    }

    fn to_style(self, p: &Palette) -> Style {
        Style::default()
            .fg(self.color.resolve(p))
            .add_modifier(self.modifiers)
    }
}

/// Runtime values and state-derived styles for a single agent-panel entry.
pub(crate) struct RowContext<'a> {
    pub icon: &'a str,
    pub icon_style: Style,
    pub space: &'a str,
    pub tab: Option<&'a str>,
    pub status: &'a str,
    pub status_color: Color,
    pub agent: Option<&'a str>,
    pub custom: Option<&'a str>,
    pub is_active: bool,
}

impl RowContext<'_> {
    /// Field value, or `None` when the field is empty and its chunk should be
    /// dropped.
    fn value(&self, field: Field) -> Option<&str> {
        let raw = match field {
            Field::Icon => Some(self.icon),
            Field::Space => Some(self.space),
            Field::Tab => self.tab,
            Field::Status => Some(self.status),
            Field::Agent => self.agent,
            Field::Custom => self.custom,
        };
        raw.filter(|value| !value.is_empty())
    }

    /// Resolve the style for a field chunk. The `icon` field always keeps its
    /// intrinsic state color. Other fields use their explicit style when set,
    /// otherwise a default derived from the palette and the entry's state.
    fn style(&self, field: Field, style: Option<StyleSpec>, p: &Palette) -> Style {
        if field == Field::Icon {
            return self.icon_style;
        }
        match style {
            Some(spec) => spec.to_style(p),
            None => self.default_style(field, p),
        }
    }

    /// The built-in style a field uses when the template gives no explicit one.
    fn default_style(&self, field: Field, p: &Palette) -> Style {
        match field {
            // Handled by the caller, but kept exhaustive for clarity.
            Field::Icon => self.icon_style,
            // Primary label: bold, brighter when the row is active.
            Field::Space | Field::Tab => {
                let fg = if self.is_active { p.text } else { p.subtext0 };
                Style::default().fg(fg).add_modifier(Modifier::BOLD)
            }
            // Status text: the state color, dimmed when the row is inactive.
            Field::Status => {
                let style = Style::default().fg(self.status_color);
                if self.is_active {
                    style
                } else {
                    style.add_modifier(Modifier::DIM)
                }
            }
            // Secondary detail text: dim overlay.
            Field::Agent | Field::Custom => {
                Style::default().fg(p.overlay0).add_modifier(Modifier::DIM)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Chunk {
    /// Literal text outside any `{}` reference. Rendered with the default
    /// (unstyled) foreground.
    Literal(String),
    /// A field reference with an optional literal prefix and an optional
    /// explicit style. The prefix and value share the field's style and are
    /// dropped together when the value is empty.
    Field {
        prefix: String,
        field: Field,
        style: Option<StyleSpec>,
    },
}

/// A parsed, reusable row template.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RowTemplate {
    chunks: Vec<Chunk>,
}

/// Error produced while parsing a row template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TemplateError(String);

impl fmt::Display for TemplateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for TemplateError {}

impl RowTemplate {
    /// Parse a template string into a reusable [`RowTemplate`].
    ///
    /// `{{` and `}}` are literal braces. `{...}` is a field reference whose
    /// trailing identifier (optionally `:style`) is the field and whose leading
    /// text is a literal prefix.
    pub(crate) fn parse(input: &str) -> Result<Self, TemplateError> {
        let mut chunks = Vec::new();
        let mut literal = String::new();
        let mut chars = input.chars().peekable();

        while let Some(ch) = chars.next() {
            match ch {
                '{' if chars.peek() == Some(&'{') => {
                    chars.next();
                    literal.push('{');
                }
                '}' if chars.peek() == Some(&'}') => {
                    chars.next();
                    literal.push('}');
                }
                '{' => {
                    if !literal.is_empty() {
                        chunks.push(Chunk::Literal(std::mem::take(&mut literal)));
                    }
                    let mut body = String::new();
                    let mut closed = false;
                    for inner in chars.by_ref() {
                        if inner == '}' {
                            closed = true;
                            break;
                        }
                        body.push(inner);
                    }
                    if !closed {
                        return Err(TemplateError(format!(
                            "unterminated field reference: '{{{body}'"
                        )));
                    }
                    chunks.push(parse_field_chunk(&body)?);
                }
                '}' => {
                    return Err(TemplateError(
                        "unexpected '}'; use '}}' for a literal brace".to_string(),
                    ));
                }
                _ => literal.push(ch),
            }
        }

        if !literal.is_empty() {
            chunks.push(Chunk::Literal(literal));
        }

        Ok(Self { chunks })
    }

    /// The built-in agent-panel row templates, matching Herdr's default look.
    pub(crate) fn default_agent_panel_rows() -> [RowTemplate; 2] {
        let rows = crate::config::DEFAULT_AGENT_PANEL_ROWS;
        // The built-in defaults are known-valid; fall back to an empty template
        // rather than panicking if they are ever edited incorrectly. A unit test
        // asserts they parse.
        [
            RowTemplate::parse(rows[0]).unwrap_or_default(),
            RowTemplate::parse(rows[1]).unwrap_or_default(),
        ]
    }

    /// Render this template for one entry, truncated to `max_width` columns.
    pub(crate) fn render(
        &self,
        ctx: &RowContext<'_>,
        p: &Palette,
        max_width: usize,
    ) -> Vec<Span<'static>> {
        let mut spans = Vec::new();
        for chunk in &self.chunks {
            match chunk {
                Chunk::Literal(text) => {
                    spans.push(Span::styled(text.clone(), Style::default()));
                }
                Chunk::Field {
                    prefix,
                    field,
                    style,
                } => {
                    let Some(value) = ctx.value(*field) else {
                        continue;
                    };
                    let style = ctx.style(*field, *style, p);
                    if !prefix.is_empty() {
                        spans.push(Span::styled(prefix.clone(), style));
                    }
                    spans.push(Span::styled(value.to_string(), style));
                }
            }
        }
        truncate_spans(spans, max_width)
    }
}

fn parse_field_chunk(body: &str) -> Result<Chunk, TemplateError> {
    // Grammar: `[prefix]<field>[:<style>]`. The style spec (a color plus
    // optional modifiers) never contains a colon, so a colon separates the
    // style only when the text before it ends in a known field name. Otherwise
    // the colon is part of the literal prefix.
    let (head, style_spec) = match body.rsplit_once(':') {
        Some((head, spec)) if split_prefix_field(head).is_some() => (head, Some(spec)),
        _ => (body, None),
    };

    let Some((prefix, field)) = split_prefix_field(head) else {
        return Err(TemplateError(format!(
            "unknown field in '{{{body}}}' (expected icon, space, tab, status, agent, or custom)"
        )));
    };

    let style = match style_spec {
        Some(spec) => Some(StyleSpec::parse(spec, body)?),
        None => None,
    };

    if field == Field::Icon && style.is_some() {
        return Err(TemplateError(format!(
            "the icon field keeps its intrinsic color and cannot be styled in '{{{body}}}'"
        )));
    }

    Ok(Chunk::Field {
        prefix: prefix.to_string(),
        field,
        style,
    })
}

/// Split a chunk head into its literal prefix and trailing field name. The
/// field name is the maximal trailing run of identifier characters. Returns
/// `None` when that trailing run is not a known field.
fn split_prefix_field(head: &str) -> Option<(&str, Field)> {
    let field_start = head
        .char_indices()
        .rev()
        .take_while(|(_, ch)| ch.is_ascii_alphanumeric() || *ch == '_')
        .last()
        .map(|(idx, _)| idx)?;
    let (prefix, name) = head.split_at(field_start);
    Field::parse(name).map(|field| (prefix, field))
}

/// Trim a span list so its combined display width does not exceed `max_width`,
/// eliding the boundary span with an ellipsis when it overflows.
fn truncate_spans(spans: Vec<Span<'static>>, max_width: usize) -> Vec<Span<'static>> {
    let mut out = Vec::with_capacity(spans.len());
    let mut used = 0usize;
    for span in spans {
        let width = display_width(&span.content);
        if used + width <= max_width {
            used += width;
            out.push(span);
            continue;
        }
        let remaining = max_width.saturating_sub(used);
        if remaining > 0 {
            let truncated = truncate_end(&span.content, remaining);
            out.push(Span::styled(truncated, span.style));
        }
        break;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn palette() -> Palette {
        Palette::catppuccin()
    }

    fn ctx() -> RowContext<'static> {
        RowContext {
            icon: "✓",
            icon_style: Style::default().fg(Color::Green),
            space: "herdr",
            tab: Some("main"),
            status: "idle",
            status_color: Color::Green,
            agent: Some("claude"),
            custom: None,
            is_active: false,
        }
    }

    fn rendered_text(spans: &[Span<'_>]) -> String {
        spans.iter().map(|s| s.content.as_ref()).collect()
    }

    #[test]
    fn default_rows_parse() {
        for row in crate::config::DEFAULT_AGENT_PANEL_ROWS {
            assert!(
                RowTemplate::parse(row).is_ok(),
                "default row must parse: {row}"
            );
        }
    }

    #[test]
    fn parses_literal_prefix_and_field() {
        let template = RowTemplate::parse("{ · tab}").expect("parse");
        assert_eq!(
            template.chunks,
            vec![Chunk::Field {
                prefix: " · ".to_string(),
                field: Field::Tab,
                style: None,
            }]
        );
    }

    #[test]
    fn parses_palette_token_color() {
        let template = RowTemplate::parse("{agent:overlay0}").expect("parse");
        assert_eq!(
            template.chunks,
            vec![Chunk::Field {
                prefix: String::new(),
                field: Field::Agent,
                style: Some(StyleSpec {
                    color: ColorSpec::Token(Token::Overlay0),
                    modifiers: Modifier::empty(),
                }),
            }]
        );
    }

    #[test]
    fn parses_literal_hex_color() {
        let template = RowTemplate::parse("{agent:#f5c2e7}").expect("parse");
        assert_eq!(
            template.chunks,
            vec![Chunk::Field {
                prefix: String::new(),
                field: Field::Agent,
                style: Some(StyleSpec {
                    color: ColorSpec::Fixed(Color::Rgb(0xf5, 0xc2, 0xe7)),
                    modifiers: Modifier::empty(),
                }),
            }]
        );
    }

    #[test]
    fn parses_prefix_with_color() {
        let template = RowTemplate::parse("{ · agent:blue+bold+dim}").expect("parse");
        assert_eq!(
            template.chunks,
            vec![Chunk::Field {
                prefix: " · ".to_string(),
                field: Field::Agent,
                style: Some(StyleSpec {
                    color: ColorSpec::Token(Token::Blue),
                    modifiers: Modifier::BOLD | Modifier::DIM,
                }),
            }]
        );
    }

    #[test]
    fn palette_token_wins_over_named_color() {
        // `green` names both a palette token and a literal color; the token wins.
        let template = RowTemplate::parse("{status:green}").expect("parse");
        assert_eq!(
            template.chunks,
            vec![Chunk::Field {
                prefix: String::new(),
                field: Field::Status,
                style: Some(StyleSpec {
                    color: ColorSpec::Token(Token::Green),
                    modifiers: Modifier::empty(),
                }),
            }]
        );
        let spans = template.render(&ctx(), &palette(), 80);
        assert_eq!(spans[0].style.fg, Some(palette().green));
    }

    #[test]
    fn named_color_without_token_stays_literal() {
        let template = RowTemplate::parse("{agent:cyan}").expect("parse");
        assert_eq!(
            template.chunks,
            vec![Chunk::Field {
                prefix: String::new(),
                field: Field::Agent,
                style: Some(StyleSpec {
                    color: ColorSpec::Fixed(Color::Cyan),
                    modifiers: Modifier::empty(),
                }),
            }]
        );
    }

    #[test]
    fn escaped_braces_are_literal() {
        let template = RowTemplate::parse("{{{space}}}").expect("parse");
        let text = rendered_text(&template.render(&ctx(), &palette(), 80));
        assert_eq!(text, "{herdr}");
    }

    #[test]
    fn unknown_field_is_rejected() {
        let err = RowTemplate::parse("{bogus}").expect_err("should reject");
        assert!(err.to_string().contains("unknown field"));
    }

    #[test]
    fn unknown_color_is_rejected() {
        let err = RowTemplate::parse("{space:rainbow}").expect_err("should reject");
        assert!(err.to_string().contains("unknown color 'rainbow'"));
    }

    #[test]
    fn unknown_modifier_is_rejected() {
        let err = RowTemplate::parse("{space:text+wobble}").expect_err("should reject");
        assert!(err.to_string().contains("unknown modifier 'wobble'"));
    }

    #[test]
    fn empty_color_is_rejected() {
        let err = RowTemplate::parse("{space:}").expect_err("should reject");
        assert!(err.to_string().contains("missing color"));
    }

    #[test]
    fn icon_cannot_be_styled() {
        let err = RowTemplate::parse("{icon:red}").expect_err("should reject");
        assert!(err
            .to_string()
            .contains("icon field keeps its intrinsic color"));
    }

    #[test]
    fn explicit_color_and_modifiers_render() {
        let template = RowTemplate::parse("{space:#ff0000+bold}").expect("parse");
        let spans = template.render(&ctx(), &palette(), 80);
        assert_eq!(spans[0].style.fg, Some(Color::Rgb(0xff, 0, 0)));
        assert!(spans[0].style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn explicit_color_ignores_active_state() {
        let template = RowTemplate::parse("{space:overlay0}").expect("parse");
        let mut context = ctx();
        context.is_active = true;
        let active = template.render(&context, &palette(), 80);
        context.is_active = false;
        let inactive = template.render(&context, &palette(), 80);
        assert_eq!(active[0].style.fg, Some(palette().overlay0));
        assert_eq!(active[0].style.fg, inactive[0].style.fg);
    }

    #[test]
    fn empty_field_reference_is_rejected() {
        assert!(RowTemplate::parse("{ · }").is_err());
    }

    #[test]
    fn unterminated_reference_is_rejected() {
        assert!(RowTemplate::parse("{space").is_err());
    }

    #[test]
    fn empty_field_drops_whole_chunk() {
        let mut context = ctx();
        context.tab = None;
        let template = RowTemplate::parse("{space}{ · tab}").expect("parse");
        let text = rendered_text(&template.render(&context, &palette(), 80));
        assert_eq!(text, "herdr");
    }

    #[test]
    fn present_field_includes_prefix() {
        let template = RowTemplate::parse("{space}{ · tab}").expect("parse");
        let text = rendered_text(&template.render(&ctx(), &palette(), 80));
        assert_eq!(text, "herdr · main");
    }

    #[test]
    fn default_row_matches_current_layout() {
        let [row0, row1] = RowTemplate::default_agent_panel_rows();
        let context = ctx();
        assert_eq!(
            rendered_text(&row0.render(&context, &palette(), 80)),
            " ✓ herdr · main"
        );
        assert_eq!(
            rendered_text(&row1.render(&context, &palette(), 80)),
            "   idle · claude"
        );
    }

    #[test]
    fn custom_status_appended_when_present() {
        let mut context = ctx();
        context.custom = Some("deploying");
        let [_, row1] = RowTemplate::default_agent_panel_rows();
        assert_eq!(
            rendered_text(&row1.render(&context, &palette(), 80)),
            "   idle · claude · deploying"
        );
    }

    #[test]
    fn truncates_to_max_width() {
        let template = RowTemplate::parse("{space}").expect("parse");
        let mut context = ctx();
        context.space = "a-very-long-workspace-name";
        let text = rendered_text(&template.render(&context, &palette(), 6));
        assert_eq!(display_width(&text), 6);
        assert!(text.ends_with('…'));
    }

    #[test]
    fn icon_field_keeps_intrinsic_style() {
        let template = RowTemplate::parse("{icon}").expect("parse");
        let spans = template.render(&ctx(), &palette(), 80);
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].style.fg, Some(Color::Green));
    }

    #[test]
    fn active_row_uses_bold_text_for_name() {
        let mut context = ctx();
        context.is_active = true;
        let template = RowTemplate::parse("{space}").expect("parse");
        let spans = template.render(&context, &palette(), 80);
        assert_eq!(spans[0].style.fg, Some(palette().text));
        assert!(spans[0].style.add_modifier.contains(Modifier::BOLD));
    }
}
