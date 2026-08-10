/// Document-level metadata for DOCX files.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DocumentMeta {
    /// Document title (maps to `dc:title`).
    pub title: Option<String>,
    /// Author name (maps to `dc:creator`).
    pub author: Option<String>,
    /// Subject / description (maps to `dc:description`).
    pub subject: Option<String>,
    /// Keywords, comma or semicolon separated (maps to `cp:keywords`).
    pub keywords: Option<String>,
    /// Page width in twips (default: A4 portrait = 11906).
    pub page_width: Option<u32>,
    /// Page height in twips (default: A4 portrait = 16838).
    pub page_height: Option<u32>,
    /// Page orientation: `true` = landscape.
    pub landscape: bool,
}

impl DocumentMeta {
    /// Creates default document metadata.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the document title.
    #[must_use]
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Sets the author name.
    #[must_use]
    pub fn author(mut self, author: impl Into<String>) -> Self {
        self.author = Some(author.into());
        self
    }

    /// Sets the subject.
    #[must_use]
    pub fn subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = Some(subject.into());
        self
    }

    /// Sets keywords.
    #[must_use]
    pub fn keywords(mut self, keywords: impl Into<String>) -> Self {
        self.keywords = Some(keywords.into());
        self
    }

    /// Sets landscape orientation.
    #[must_use]
    pub fn landscape(mut self, landscape: bool) -> Self {
        self.landscape = landscape;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn document_meta_builder_chain() {
        let meta = DocumentMeta::new()
            .title("Test Document")
            .author("Author Name")
            .subject("Test Subject")
            .keywords("rust,docx")
            .landscape(true);
        assert_eq!(meta.title.as_deref(), Some("Test Document"));
        assert_eq!(meta.author.as_deref(), Some("Author Name"));
        assert_eq!(meta.subject.as_deref(), Some("Test Subject"));
        assert_eq!(meta.keywords.as_deref(), Some("rust,docx"));
        assert!(meta.landscape);
    }

    #[test]
    fn document_meta_default() {
        let meta = DocumentMeta::default();
        assert!(meta.title.is_none());
        assert!(meta.author.is_none());
        assert!(meta.subject.is_none());
        assert!(meta.keywords.is_none());
        assert!(meta.page_width.is_none());
        assert!(meta.page_height.is_none());
        assert!(!meta.landscape);
    }

    #[test]
    fn document_meta_clone_eq() {
        let meta = DocumentMeta::new().title("Test");
        let meta2 = meta.clone();
        assert_eq!(meta, meta2);
    }

    #[test]
    fn document_meta_debug() {
        let meta = DocumentMeta::new().title("Test");
        let dbg = format!("{:?}", meta);
        assert!(dbg.contains("Test"));
    }

    #[test]
    fn document_meta_page_dimensions() {
        let mut meta = DocumentMeta::new();
        meta.page_width = Some(11906);
        meta.page_height = Some(16838);
        assert_eq!(meta.page_width, Some(11906));
        assert_eq!(meta.page_height, Some(16838));
    }
}
