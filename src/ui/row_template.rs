//! Mini template language for agent-panel sidebar rows.
//!
//! A row template is a sequence of chunks. Each chunk is either literal text or
//! a `{...}` field reference. A field reference has an optional literal prefix
//! and an optional style role: `{field}`, `{field:role}`, or `{ literal field}`.
//!
//! A field chunk whose value is empty is dropped entirely, including its
//! literal prefix. That is how `{ · tab}` renders nothing (no dangling
//! separator) when a pane has no tab label.
//!
//! Styling is expressed with named roles that resolve against the active
//! [`Palette`], keeping templated rows consistent with the rest of the theme.
//! The parser and renderer are pure and unit-tested without any terminal.

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

    fn default_role(self) -> Role {
        match self {
            Field::Icon => Role::Icon,
            Field::Space | Field::Tab => Role::Name,
            Field::Status => Role::Status,
            Field::Agent | Field::Custom => Role::Muted,
        }
    }
}

/// Named style roles that map to palette colors and modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Role {
    /// Primary label text: subtext0 + bold (text + bold when the row is active).
    Name,
    /// Status text: the state color (dimmed when the row is inactive).
    Status,
    /// Secondary detail text: overlay0 + dim.
    Muted,
    /// The field's own intrinsic style (only meaningful for `icon`). Text
    /// fields fall back to [`Role::Name`].
    Icon,
}

impl Role {
    fn parse(name: &str) -> Option<Self> {
        match name {
            "name" => Some(Self::Name),
            "status" => Some(Self::Status),
            "muted" => Some(Self::Muted),
            "icon" => Some(Self::Icon),
            _ => None,
        }
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

    fn style(&self, field: Field, role: Role, p: &Palette) -> Style {
        if field == Field::Icon {
            return self.icon_style;
        }
        match role {
            Role::Name | Role::Icon => {
                if self.is_active {
                    Style::default().fg(p.text).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(p.subtext0).add_modifier(Modifier::BOLD)
                }
            }
            Role::Status => {
                if self.is_active {
                    Style::default().fg(self.status_color)
                } else {
                    Style::default()
                        .fg(self.status_color)
                        .add_modifier(Modifier::DIM)
                }
            }
            Role::Muted => Style::default().fg(p.overlay0).add_modifier(Modifier::DIM),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Chunk {
    /// Literal text outside any `{}` reference. Rendered with the default
    /// (unstyled) foreground.
    Literal(String),
    /// A field reference with an optional literal prefix and a resolved style
    /// role. The prefix and value share the field's style and are dropped
    /// together when the value is empty.
    Field {
        prefix: String,
        field: Field,
        role: Role,
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
    /// trailing identifier (optionally `:role`) is the field and whose leading
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
                    role,
                } => {
                    let Some(value) = ctx.value(*field) else {
                        continue;
                    };
                    let style = ctx.style(*field, *role, p);
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
    // The field spec is the maximal trailing run of identifier/`:` characters.
    // Everything before it is a literal prefix.
    let spec_start = body
        .char_indices()
        .rev()
        .take_while(|(_, ch)| ch.is_ascii_alphanumeric() || *ch == '_' || *ch == ':')
        .last()
        .map(|(idx, _)| idx);

    let Some(spec_start) = spec_start else {
        return Err(TemplateError(format!(
            "field reference '{{{body}}}' has no field name"
        )));
    };

    let (prefix, spec) = body.split_at(spec_start);
    let (field_name, role) = match spec.split_once(':') {
        Some((field_name, role_name)) => {
            let role = Role::parse(role_name).ok_or_else(|| {
                TemplateError(format!(
                    "unknown style role '{role_name}' in '{{{body}}}' (expected name, status, muted, or icon)"
                ))
            })?;
            (field_name, Some(role))
        }
        None => (spec, None),
    };

    let field = Field::parse(field_name).ok_or_else(|| {
        TemplateError(format!(
            "unknown field '{field_name}' in '{{{body}}}' (expected icon, space, tab, status, agent, or custom)"
        ))
    })?;

    Ok(Chunk::Field {
        prefix: prefix.to_string(),
        field,
        role: role.unwrap_or_else(|| field.default_role()),
    })
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
                role: Role::Name,
            }]
        );
    }

    #[test]
    fn parses_explicit_role() {
        let template = RowTemplate::parse("{agent:muted}").expect("parse");
        assert_eq!(
            template.chunks,
            vec![Chunk::Field {
                prefix: String::new(),
                field: Field::Agent,
                role: Role::Muted,
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
        assert!(err.to_string().contains("unknown field 'bogus'"));
    }

    #[test]
    fn unknown_role_is_rejected() {
        let err = RowTemplate::parse("{space:rainbow}").expect_err("should reject");
        assert!(err.to_string().contains("unknown style role 'rainbow'"));
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
