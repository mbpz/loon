//! `JourneyReachableNodesEvaluation` — walks the journey graph
//! from a starting node and returns all nodes reachable via BFS.

use std::collections::{HashMap, HashSet, VecDeque};

use loon_core::{Journey, JourneyEdge, JourneyId, JourneyNodeId};

use crate::error::EngineResult;

/// Performs BFS over a journey's edge list.
pub struct JourneyReachableNodesEvaluation;

impl JourneyReachableNodesEvaluation {
    pub fn new() -> Self {
        Self
    }
}

impl Default for JourneyReachableNodesEvaluation {
    fn default() -> Self {
        Self::new()
    }
}

/// Pure-function BFS helper used by the indexing layer. Walks the
/// directed graph defined by `edges` from `from` and returns the
/// set of nodes reachable.
pub fn reachable_nodes(
    _journey: &Journey,
    edges: &[JourneyEdge],
    from: &JourneyNodeId,
) -> HashSet<JourneyNodeId> {
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back(from.clone());

    let adj: HashMap<&JourneyNodeId, Vec<&JourneyNodeId>> =
        edges.iter().fold(HashMap::new(), |mut m, e| {
            m.entry(&e.source).or_default().push(&e.target);
            m
        });

    while let Some(cur) = queue.pop_front() {
        if !visited.insert(cur.clone()) {
            continue;
        }
        if let Some(neighbors) = adj.get(&cur) {
            for n in neighbors {
                if !visited.contains(n) {
                    queue.push_back((*n).clone());
                }
            }
        }
    }
    visited
}

impl JourneyReachableNodesEvaluation {
    /// Phase-1 stub: returns an empty node list. The pure-function
    /// `reachable_nodes` above is the real BFS implementation and is
    /// exercised directly by tests; this async wrapper will later
    /// persist results and load them asynchronously.
    pub async fn evaluate(
        &self,
        _journey_id: &JourneyId,
        _journey: &Journey,
        _edges: &[JourneyEdge],
    ) -> EngineResult<Vec<JourneyNodeId>> {
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use loon_core::{AgentId, Journey};

    fn make_edge(source: JourneyNodeId, target: JourneyNodeId) -> JourneyEdge {
        JourneyEdge::new(source, target, "always")
    }

    fn make_journey() -> Journey {
        Journey {
            id: JourneyId::new(),
            agent_id: AgentId::new(),
            title: "t".into(),
            description: "".into(),
            root_id: JourneyNodeId::new(),
            tags: vec![],
            creation_utc: chrono::Utc::now(),
        }
    }

    #[test]
    fn reachable_nodes_bfs_visits_direct_neighbours() {
        let a = JourneyNodeId::new();
        let b = JourneyNodeId::new();
        let c = JourneyNodeId::new();
        let edges = vec![make_edge(a.clone(), b.clone()), make_edge(b.clone(), c.clone())];
        let j = make_journey();
        let visited = reachable_nodes(&j, &edges, &a);
        assert_eq!(visited.len(), 3);
        assert!(visited.contains(&a));
        assert!(visited.contains(&b));
        assert!(visited.contains(&c));
    }

    #[test]
    fn reachable_nodes_handles_disconnected_components() {
        let a = JourneyNodeId::new();
        let b = JourneyNodeId::new();
        let c = JourneyNodeId::new();
        let edges = vec![make_edge(a.clone(), b.clone())];
        let j = make_journey();
        let visited = reachable_nodes(&j, &edges, &a);
        assert_eq!(visited.len(), 2);
        assert!(!visited.contains(&c));
    }

    #[tokio::test]
    async fn evaluate_returns_empty_in_phase1() {
        let eval = JourneyReachableNodesEvaluation::new();
        let j = make_journey();
        let res = eval.evaluate(&j.id, &j, &[]).await.unwrap();
        assert!(res.is_empty());
    }
}
