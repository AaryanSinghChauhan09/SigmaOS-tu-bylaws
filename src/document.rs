//! AsciiDoc bylaws document model.
//!
//! Parses `tu-bylaws.adoc`-style documents: `=` title, `---` section
//! headers (level-2 `---` underlined or `==`/`---` style used upstream),
//! and section bodies. Upstream uses `Section\n-------` underline style.

/// A document section.
#[derive(Debug, Clone)]
pub struct Section {
    /// Section title.
    pub title: String,
    /// Body text.
    pub body: String,
    /// Line number where the section starts.
    pub line: usize,
}

/// Parsed bylaws document.
#[derive(Debug, Default)]
pub struct BylawsDocument {
    /// Document title (first `=` line).
    pub title: String,
    /// Version string (e.g. `1.3, 2013-08-07`).
    pub version: String,
    /// Sections in order.
    pub sections: Vec<Section>,
}

impl BylawsDocument {
    /// Find a section by title (case-insensitive contains).
    pub fn find_section(&self, needle: &str) -> Option<&Section> {
        let n = needle.to_lowercase();
        self.sections.iter().find(|s| s.title.to_lowercase().contains(&n))
    }

    /// All section titles.
    pub fn titles(&self) -> Vec<&str> {
        self.sections.iter().map(|s| s.title.as_str()).collect()
    }
}

/// Parse an AsciiDoc bylaws document.
pub fn parse_document(text: &str) -> BylawsDocument {
    let mut doc = BylawsDocument::default();
    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0;
    // Title: first line ending with '=' underline style "Title\n===..."
    while i < lines.len() {
        let line = lines[i].trim();
        if !line.is_empty() && i + 1 < lines.len() && lines[i + 1].chars().all(|c| c == '=') && !lines[i + 1].is_empty() {
            doc.title = line.to_string();
            i += 2;
            break;
        }
        i += 1;
    }
    // Version: look for "X.Y, date" pattern in first ~10 lines.
    for line in lines.iter().take(10) {
        let t = line.trim();
        if t.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false)
            && t.contains(',')
        {
            doc.version = t.to_string();
            break;
        }
    }

    // Sections: Title\n------- (dashes) or == Title.
    let mut cur: Option<Section> = None;
    let mut body = String::new();
    while i < lines.len() {
        let line = lines[i];
        let next_is_dashes = i + 1 < lines.len()
            && lines[i + 1].chars().all(|c| c == '-')
            && lines[i + 1].len() >= 3;
        if next_is_dashes && !line.trim().is_empty() {
            if let Some(s) = cur.take() {
                doc.sections.push(Section { body: std::mem::take(&mut body), ..s });
            }
            cur = Some(Section {
                title: line.trim().to_string(),
                body: String::new(),
                line: i + 1,
            });
            i += 2;
            continue;
        }
        if let Some(ref mut s) = cur {
            body.push_str(line);
            body.push('\n');
        }
        i += 1;
    }
    if let Some(s) = cur.take() {
        doc.sections.push(Section { body, ..s });
    }
    doc
}

/// Read and parse a bylaws file.
pub fn load_document(path: &std::path::Path) -> Result<BylawsDocument, crate::BylawsError> {
    let text = std::fs::read_to_string(path)?;
    Ok(parse_document(&text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_upstream_style() {
        let text = "Trusted User Bylaws\n===================\nTrusted Users <a@b>\n1.3, 2013-08-07\n\nSummary\n-------\nBody text.\n\nMission\n-------\nMore text.\n";
        let doc = parse_document(text);
        assert_eq!(doc.title, "Trusted User Bylaws");
        assert_eq!(doc.version, "1.3, 2013-08-07");
        assert_eq!(doc.sections.len(), 2);
        assert_eq!(doc.sections[0].title, "Summary");
        assert!(doc.find_section("mission").is_some());
    }
}
