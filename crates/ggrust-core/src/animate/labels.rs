//! Label templates (gganimate's glue strings) in the plot title, subtitle and
//! caption: `{frame}`, `{nframes}`, `{progress}` and the transition's variables
//! (`{current_frame}`, ...) are replaced per frame. `{{` and `}}` are literal
//! braces. gganimate evaluates any R expression and keeps a label whose glue
//! fails; ggrust takes variable names only and rejects an unknown one with a
//! close match.

use crate::suggest::did_you_mean;
use crate::{Error, Result};

/// Variables every animation has (gganimate `Scene$finish_data`).
pub(crate) const FRAME_VARIABLES: &[&str] = &["frame", "nframes", "progress"];

/// One piece of a template.
#[derive(Debug, Clone, PartialEq)]
enum Part {
    Text(String),
    Variable(String),
}

/// A parsed label template.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Template {
    parts: Vec<Part>,
}

impl Template {
    /// Parse `text`; `names` are the variables it may use.
    ///
    /// # Errors
    ///
    /// A spec error at `path` for an unknown variable, an expression or an
    /// unclosed brace.
    pub(crate) fn parse(text: &str, path: &str, names: &[&str]) -> Result<Template> {
        let mut parts = Vec::new();
        let mut literal = String::new();
        let mut characters = text.chars().peekable();
        let accepted = || {
            names
                .iter()
                .map(|name| format!("{{{name}}}"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        while let Some(character) = characters.next() {
            match character {
                '{' if characters.peek() == Some(&'{') => {
                    characters.next();
                    literal.push('{');
                }
                '}' if characters.peek() == Some(&'}') => {
                    characters.next();
                    literal.push('}');
                }
                '{' => {
                    let mut name = String::new();
                    let mut closed = false;
                    for inner in characters.by_ref() {
                        if inner == '}' {
                            closed = true;
                            break;
                        }
                        name.push(inner);
                    }
                    if !closed {
                        return Err(Error::spec(
                            path,
                            format!(
                                "unclosed '{{' in the label template {text:?}; write '{{{{' \
                                 for a literal brace"
                            ),
                        ));
                    }
                    let name = name.trim().to_owned();
                    if !names.contains(&name.as_str()) {
                        let is_name = !name.is_empty()
                            && name
                                .chars()
                                .all(|character| character.is_alphanumeric() || character == '_');
                        let reason = if is_name {
                            format!(
                                "unknown label variable '{{{name}}}'{}",
                                did_you_mean(&name, names.iter().copied())
                            )
                        } else {
                            format!("label templates take variable names only, got '{{{name}}}'")
                        };
                        return Err(Error::spec(
                            path,
                            format!("{reason}; this animation has {}", accepted()),
                        ));
                    }
                    if !literal.is_empty() {
                        parts.push(Part::Text(std::mem::take(&mut literal)));
                    }
                    parts.push(Part::Variable(name));
                }
                other => literal.push(other),
            }
        }
        if !literal.is_empty() {
            parts.push(Part::Text(literal));
        }
        Ok(Template { parts })
    }

    /// The text with every variable replaced by its value in `values`.
    pub(crate) fn fill(&self, values: &[(&str, String)]) -> String {
        let mut text = String::new();
        for part in &self.parts {
            match part {
                Part::Text(literal) => text.push_str(literal),
                Part::Variable(name) => {
                    if let Some((_, value)) = values.iter().find(|(known, _)| known == name) {
                        text.push_str(value);
                    }
                }
            }
        }
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NAMES: &[&str] = &["frame", "nframes", "progress", "current_frame"];

    #[test]
    fn variables_are_replaced_and_double_braces_kept() {
        let template = Template::parse(
            "Year {current_frame} ({frame}/{ nframes }) {{x}}",
            "title",
            NAMES,
        )
        .unwrap();
        let values = [
            ("frame", "2".to_owned()),
            ("nframes", "5".to_owned()),
            ("current_frame", "1957".to_owned()),
        ];
        assert_eq!(template.fill(&values), "Year 1957 (2/5) {x}");
        let plain = Template::parse("no braces", "title", NAMES).unwrap();
        assert_eq!(plain.fill(&values), "no braces");
    }

    #[test]
    fn unknown_names_and_expressions_are_spec_errors() {
        let error = Template::parse("{curent_frame}", "ggtitle.text", NAMES)
            .unwrap_err()
            .to_string();
        assert!(error.contains("ggtitle.text"), "{error}");
        assert!(error.contains("did you mean 'current_frame'"), "{error}");
        let error = Template::parse("{round(progress, 2)}", "title", NAMES)
            .unwrap_err()
            .to_string();
        assert!(error.contains("variable names only"), "{error}");
        let error = Template::parse("{frame", "title", NAMES)
            .unwrap_err()
            .to_string();
        assert!(error.contains("unclosed"), "{error}");
    }
}
