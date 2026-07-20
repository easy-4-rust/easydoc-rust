/// Document-level metadata for DOCX files.
#[derive(Debug, Clone, Default)]
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
