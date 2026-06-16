use serde::{Deserialize, Serialize};
use crate::GlossaryTermId;
use crate::TagId;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Term {
    pub id: GlossaryTermId,
    pub name: String,
    pub description: String,
    pub synonyms: Vec<String>,
    pub tags: Vec<TagId>,
}

impl Term {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            id: GlossaryTermId::new(),
            name: name.into(),
            description: description.into(),
            synonyms: vec![],
            tags: vec![],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Glossary {
    pub terms: Vec<Term>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn term_default_no_synonyms() {
        let t = Term::new("x", "y");
        assert!(t.synonyms.is_empty());
        assert!(t.tags.is_empty());
        assert_eq!(t.name, "x");
    }

    #[test]
    fn glossary_default_is_empty() {
        let g = Glossary::default();
        assert!(g.terms.is_empty());
    }
}
