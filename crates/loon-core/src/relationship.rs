use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::RelationshipId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RelationshipKind {
    Entails,
    Excludes,
    Dependency,
    Reevaluation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RelationshipEntityKind {
    Guideline,
    Tag,
    Tool,
    Journey,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelationshipEntity {
    pub kind: RelationshipEntityKind,
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Relationship {
    pub id: RelationshipId,
    pub source: RelationshipEntity,
    pub target: RelationshipEntity,
    pub kind: RelationshipKind,
    pub indirect: bool,
    pub creation_utc: DateTime<Utc>,
}

impl Relationship {
    pub fn new(source: RelationshipEntity, target: RelationshipEntity, kind: RelationshipKind) -> Self {
        Self {
            id: RelationshipId::new(),
            source,
            target,
            kind,
            indirect: false,
            creation_utc: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn relationship_default_not_indirect() {
        let r = Relationship::new(
            RelationshipEntity { kind: RelationshipEntityKind::Guideline, id: "a".into() },
            RelationshipEntity { kind: RelationshipEntityKind::Guideline, id: "b".into() },
            RelationshipKind::Excludes,
        );
        assert!(!r.indirect);
    }

    #[test]
    fn relationship_kind_distinct() {
        assert_ne!(RelationshipKind::Entails, RelationshipKind::Excludes);
    }
}
