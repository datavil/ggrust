//! Tooltip line templates (lets-plot `LineSpecConfig`): `"label|text @field more"`.
//!
//! A line is an optional label (before the first `|`, trimmed) and a pattern of
//! literal text and fields. Fields follow lets-plot's `SOURCE_RE_PATTERN`:
//! `^aes` (word characters), `@name` (word characters, `^` and `@`, so `@x^2` is
//! one name and a dot ends it), `@{any name}`, `@..stat..`. `\@` and `\^` are a
//! literal `@` / `^`. An `@` that starts no field (e.g. `@{` without a closing `}`)
//! stays text, and later fields still parse.

/// One field reference.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Field {
    /// `^color`: the value mapped to an aesthetic.
    Aesthetic(String),
    /// `@name`: a data or stat variable (`..count..` keeps its dots).
    Variable(String),
}

/// A part of a line's pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Part {
    /// Literal text.
    Text(String),
    /// A field's formatted value.
    Field(Field),
}

/// A parsed line template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    /// Text before `|`, trimmed; `None` without a `|`.
    pub label: Option<String>,
    /// Literal text and fields.
    pub parts: Vec<Part>,
}

impl Line {
    /// Parse `"label|pattern"` or `"pattern"`.
    pub fn parse(line: &str) -> Line {
        let (label, pattern) = match line.split_once('|') {
            Some((label, pattern)) => (Some(label.trim().to_owned()), pattern),
            None => (None, line),
        };
        Line {
            label,
            parts: parse_pattern(pattern),
        }
    }

    /// The fields of the line, in order.
    pub fn fields(&self) -> impl Iterator<Item = &Field> {
        self.parts.iter().filter_map(|part| match part {
            Part::Field(field) => Some(field),
            Part::Text(_) => None,
        })
    }
}

/// A `format()` field (lets-plot `prepareFormats`): `^x` / `^X` / `^Y` name
/// aesthetics (`X` and `Y` every x / y position aesthetic); anything else is a
/// variable, with or without `@` and braces.
pub fn format_field(field: &str) -> Field {
    match field.strip_prefix('^') {
        Some(aesthetic) => Field::Aesthetic(aesthetic.to_owned()),
        None => {
            let name = field.strip_prefix('@').unwrap_or(field);
            let name = name
                .strip_prefix('{')
                .and_then(|inner| inner.strip_suffix('}'))
                .unwrap_or(name);
            Field::Variable(name.to_owned())
        }
    }
}

fn is_word(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}

fn parse_pattern(pattern: &str) -> Vec<Part> {
    let characters: Vec<char> = pattern.chars().collect();
    let mut parts = Vec::new();
    let mut text = String::new();
    let mut index = 0;
    let push_field = |text: &mut String, parts: &mut Vec<Part>, field: Field| {
        if !text.is_empty() {
            parts.push(Part::Text(std::mem::take(text)));
        }
        parts.push(Part::Field(field));
    };
    while index < characters.len() {
        let character = characters[index];
        let next = characters.get(index + 1).copied();
        match (character, next) {
            ('\\', Some(escaped @ ('@' | '^'))) => {
                text.push(escaped);
                index += 2;
            }
            ('^', Some(first)) if is_word(first) => {
                let end = scan(&characters, index + 1, is_word);
                let name: String = characters[index + 1..end].iter().collect();
                push_field(&mut text, &mut parts, Field::Aesthetic(name));
                index = end;
            }
            ('@', Some('{')) => {
                match (index + 2..characters.len()).find(|&at| characters[at] == '}') {
                    Some(close) => {
                        let name: String = characters[index + 2..close].iter().collect();
                        push_field(&mut text, &mut parts, Field::Variable(name));
                        index = close + 1;
                    }
                    None => {
                        text.push('@');
                        index += 1;
                    }
                }
            }
            ('@', Some('.')) => {
                // `@..name..`
                let inner_start = index + 3;
                let ok = characters.get(index + 2) == Some(&'.');
                let inner_end = scan(&characters, inner_start.min(characters.len()), is_word);
                let closes = ok
                    && inner_end > inner_start
                    && characters.get(inner_end) == Some(&'.')
                    && characters.get(inner_end + 1) == Some(&'.');
                if closes {
                    let name: String = characters[index + 1..inner_end + 2].iter().collect();
                    push_field(&mut text, &mut parts, Field::Variable(name));
                    index = inner_end + 2;
                } else {
                    text.push('@');
                    index += 1;
                }
            }
            ('@', Some(first)) if is_word(first) || first == '^' || first == '@' => {
                let end = scan(&characters, index + 1, |character| {
                    is_word(character) || character == '^' || character == '@'
                });
                let name: String = characters[index + 1..end].iter().collect();
                push_field(&mut text, &mut parts, Field::Variable(name));
                index = end;
            }
            _ => {
                text.push(character);
                index += 1;
            }
        }
    }
    if !text.is_empty() {
        parts.push(Part::Text(text));
    }
    parts
}

fn scan(characters: &[char], start: usize, accept: impl Fn(char) -> bool) -> usize {
    (start..characters.len())
        .find(|&at| !accept(characters[at]))
        .unwrap_or(characters.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn variable(name: &str) -> Part {
        Part::Field(Field::Variable(name.into()))
    }

    fn text(value: &str) -> Part {
        Part::Text(value.into())
    }

    #[test]
    fn label_and_fields() {
        let line = Line::parse(" person |@name is @age years");
        assert_eq!(line.label.as_deref(), Some("person"));
        assert_eq!(
            line.parts,
            vec![
                variable("name"),
                text(" is "),
                variable("age"),
                text(" years")
            ]
        );
        assert_eq!(Line::parse("@x").label, None);
        assert_eq!(Line::parse("a|b|@x").parts, vec![text("b|"), variable("x")]);
    }

    #[test]
    fn names_follow_lets_plot() {
        // `^` belongs to a variable name, a dot ends it.
        assert_eq!(Line::parse("@x^2").parts, vec![variable("x^2")]);
        assert_eq!(
            Line::parse("@y.bad").parts,
            vec![variable("y"), text(".bad")]
        );
        assert_eq!(
            Line::parse("@{gene name}!").parts,
            vec![variable("gene name"), text("!")]
        );
        assert_eq!(
            Line::parse("n=@..count..").parts,
            vec![text("n="), variable("..count..")]
        );
        assert_eq!(
            Line::parse("^color").parts,
            vec![Part::Field(Field::Aesthetic("color".into()))]
        );
    }

    #[test]
    fn escapes_and_dangling_markers_stay_text() {
        assert_eq!(Line::parse(r"a\@b \^c").parts, vec![text("a@b ^c")]);
        // An unclosed `@{` does not hide later fields.
        assert_eq!(
            Line::parse("@{x and @y").parts,
            vec![text("@{x and "), variable("y")]
        );
        assert_eq!(Line::parse("50 @ 3").parts, vec![text("50 @ 3")]);
        assert_eq!(Line::parse("@..x").parts, vec![text("@..x")]);
        assert_eq!(Line::parse("^ ").parts, vec![text("^ ")]);
    }

    #[test]
    fn format_fields_may_omit_the_at() {
        assert_eq!(format_field("@x"), Field::Variable("x".into()));
        assert_eq!(format_field("x"), Field::Variable("x".into()));
        assert_eq!(format_field("@{a b}"), Field::Variable("a b".into()));
        assert_eq!(format_field("^Y"), Field::Aesthetic("Y".into()));
    }
}
