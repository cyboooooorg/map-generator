//! Every page of the science reference cites its sources (No invented
//! data; architecture rewrite design, writing rules for the science
//! reference).

use std::fs;
use std::path::Path;

/// Heading of the section that lists a page's references.
pub const HEADING: &str = "## Sources";

/// Files of `docs/science` that are not articles: the book's landing page
/// and its table of contents.
pub const NOT_ARTICLES: [&str; 2] = ["README.md", "SUMMARY.md"];

/// Articles allowed without the section for now, each with why. An entry
/// that gains its section fails the check, so the list only shrinks.
pub const EXEMPT: [(&str, &str); 1] = [(
    "planet-atmospheres.md",
    "its citations are inline; the planet model 2c branch rewrites them and \
     gathers them into a Sources section",
)];

/// Whether `page` (Markdown) has a [`HEADING`] section holding at least
/// one line of text other than a heading. The section ends at the next
/// heading of level 1 or 2; a `### ` subheading stays inside it. Lines in
/// fenced code blocks are neither headings nor text.
pub fn has_sources(page: &str) -> bool {
    let mut fence: Option<&str> = None;
    let mut in_section = false;
    for line in page.lines() {
        let trimmed = line.trim_start();
        let marker = ["```", "~~~"]
            .into_iter()
            .find(|marker| trimmed.starts_with(marker));
        match (fence, marker) {
            (None, Some(marker)) => {
                fence = Some(marker);
                continue;
            }
            (Some(open), Some(marker)) if open == marker => {
                fence = None;
                continue;
            }
            (Some(_), _) => {
                if in_section {
                    return true;
                }
                continue;
            }
            (None, None) => {}
        }
        if line.trim_end() == HEADING {
            in_section = true;
        } else if line.starts_with("# ") || line.starts_with("## ") {
            in_section = false;
        } else if in_section && !trimmed.is_empty() && !trimmed.starts_with('#') {
            return true;
        }
    }
    false
}

/// One line per article of `dir` (its `*.md` files, minus
/// [`NOT_ARTICLES`]) without a non-empty [`HEADING`] section, or listed in
/// `exempt` while it has one, sorted; and the number of articles read.
pub fn without_sources(dir: &Path, exempt: &[&str]) -> (Vec<String>, usize) {
    let mut pages: Vec<_> = fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| path.extension().is_some_and(|ext| ext == "md"))
                .filter(|path| {
                    !path
                        .file_name()
                        .is_some_and(|name| NOT_ARTICLES.iter().any(|skip| name == *skip))
                })
                .collect()
        })
        .unwrap_or_default();
    pages.sort();
    let found = pages
        .iter()
        .filter_map(|path| {
            let cited = fs::read_to_string(path).is_ok_and(|text| has_sources(&text));
            let exempted = path
                .file_name()
                .is_some_and(|name| exempt.iter().any(|page| name == *page));
            match (cited, exempted) {
                (false, false) => Some(format!(
                    "{}: no non-empty `{HEADING}` section",
                    path.display()
                )),
                (true, true) => Some(format!(
                    "{}: has its `{HEADING}` section, remove it from the exemptions",
                    path.display()
                )),
                _ => None,
            }
        })
        .collect();
    (found, pages.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_section_with_a_reference_passes() {
        assert!(has_sources(
            "# Page\n\nText.\n\n## Sources\n\n- Author (2001). *Title*.\n"
        ));
    }

    #[test]
    fn a_missing_section_fails() {
        assert!(!has_sources("# Page\n\nSee Author (2001).\n"));
    }

    #[test]
    fn an_empty_section_fails_at_the_end_and_before_a_heading() {
        assert!(!has_sources("# Page\n\n## Sources\n\n"));
        assert!(!has_sources("# Page\n\n## Sources\n\n## Next\n\nText.\n"));
        assert!(!has_sources(
            "# Page\n\n## Sources\n\n# Appendix\n\nText.\n"
        ));
    }

    #[test]
    fn a_subsection_stays_inside_the_section() {
        assert!(!has_sources("## Sources\n\n### Papers\n\n"));
        assert!(has_sources(
            "## Sources\n\n### Papers\n\n- Author (2001).\n"
        ));
    }

    #[test]
    fn another_heading_name_does_not_count() {
        assert!(!has_sources("## References\n\n- Author (2001).\n"));
        assert!(!has_sources("## Sources and limits\n\n- Author (2001).\n"));
    }

    #[test]
    fn a_heading_inside_a_code_block_does_not_count() {
        assert!(!has_sources(
            "# Page\n\n```markdown\n## Sources\n\n- Author (2001).\n```\n"
        ));
        assert!(!has_sources("~~~text\n## Sources\n~~~\n\nText.\n"));
    }

    #[test]
    fn a_code_block_in_the_section_is_content() {
        assert!(has_sources("## Sources\n\n```bibtex\n@article{a}\n```\n"));
    }

    #[test]
    fn pages_are_listed_and_non_articles_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path();
        fs::write(path.join("README.md"), "# Book\n").unwrap();
        fs::write(path.join("SUMMARY.md"), "- [A](a.md)\n").unwrap();
        fs::write(path.join("a.md"), "# A\n\n## Sources\n\n- X (2001).\n").unwrap();
        fs::write(path.join("b.md"), "# B\n\n## Sources\n").unwrap();
        fs::write(path.join("notes.txt"), "").unwrap();
        let (found, read) = without_sources(path, &[]);
        assert_eq!(read, 2);
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].ends_with("b.md: no non-empty `## Sources` section"));
    }

    #[test]
    fn an_exemption_holds_until_the_page_cites_its_sources() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path();
        fs::write(
            path.join("b.md"),
            "# B

See X (2001).
",
        )
        .unwrap();
        assert_eq!(without_sources(path, &["b.md"]).0, Vec::<String>::new());
        fs::write(
            path.join("b.md"),
            "# B

## Sources

- X (2001).
",
        )
        .unwrap();
        let (found, _) = without_sources(path, &["b.md"]);
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].ends_with("remove it from the exemptions"));
    }
}
