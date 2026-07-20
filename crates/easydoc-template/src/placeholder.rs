//! Placeholder detection and parsing for `{key}` and `{.field}` tokens.

/// A detected placeholder in a DOCX document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Placeholder {
    /// Scalar placeholder: `{name}`, `{date}`, etc.
    Scalar {
        /// The raw placeholder text including braces.
        raw: String,
        /// The key name (without braces).
        key: String,
    },
    /// Collection placeholder: `{.items}` — marks where list rows expand.
    Collection {
        /// The raw placeholder text including braces.
        raw: String,
        /// The field name (without dot and braces).
        field: String,
    },
    /// Named collection placeholder: `{prefix.field}`
    NamedCollection {
        /// The raw placeholder text including braces.
        raw: String,
        /// The prefix name.
        prefix: String,
        /// The field name.
        field: String,
    },
}

impl Placeholder {
    /// Detects all placeholders in a text string.
    #[must_use]
    pub fn find_all(text: &str) -> Vec<Self> {
        let mut result = Vec::new();
        let mut chars = text.char_indices().peekable();

        while let Some((start, c)) = chars.next() {
            if c == '{' {
                let content_start = start + 1;
                let mut content = String::new();
                let mut found_end = false;
                let mut end_pos = content_start;

                for (pos, next) in chars.by_ref() {
                    if next == '}' {
                        found_end = true;
                        end_pos = pos;
                        break;
                    }
                    content.push(next);
                }

                if found_end && !content.is_empty() {
                    let raw = text[start..=end_pos].to_owned();
                    let trimmed = content.trim();

                    if let Some(field) = trimmed.strip_prefix('.') {
                        result.push(Placeholder::Collection {
                            raw,
                            field: field.to_owned(),
                        });
                    } else if let Some((prefix, field)) = trimmed.split_once('.') {
                        result.push(Placeholder::NamedCollection {
                            raw,
                            prefix: prefix.to_owned(),
                            field: field.to_owned(),
                        });
                    } else {
                        result.push(Placeholder::Scalar {
                            raw,
                            key: trimmed.to_owned(),
                        });
                    }
                }
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scalar_placeholder() {
        let found = Placeholder::find_all("Hello {name}, welcome!");
        assert_eq!(found.len(), 1);
        assert_eq!(
            found[0],
            Placeholder::Scalar {
                raw: "{name}".to_owned(),
                key: "name".to_owned(),
            }
        );
    }

    #[test]
    fn test_collection_placeholder() {
        let found = Placeholder::find_all("Items: {.items}");
        assert_eq!(found.len(), 1);
        assert_eq!(
            found[0],
            Placeholder::Collection {
                raw: "{.items}".to_owned(),
                field: "items".to_owned(),
            }
        );
    }

    #[test]
    fn test_named_collection_placeholder() {
        let found = Placeholder::find_all("{user.name} {user.email}");
        assert_eq!(found.len(), 2);
    }

    #[test]
    fn test_multiple_placeholders() {
        let found = Placeholder::find_all("{greeting} {name}, your order {.items} is ready.");
        assert_eq!(found.len(), 3);
    }
}
