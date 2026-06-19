//! Plugin system — runtime injection of custom Tool / Guideline / Journey.
//!
//! A `Plugin` declares what it contributes to the system. The
//! `PluginRegistry` collects plugins and applies them at server
//! startup time. Phase 1: declarative contributions only (no live
//! `register_hook` / `on_event` callbacks yet — those land in a
//! follow-up phase).

use async_trait::async_trait;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

use crate::{AgentId, CoreResult, Guideline, Journey, Tool, ToolService};

/// What a plugin can contribute. Each variant is one of the entity
/// kinds the SDK accepts as a runtime injection point.
#[derive(Clone)]
pub enum PluginContribution {
    Tool {
        agent_id: AgentId,
        tool: Tool,
    },
    Guideline {
        agent_id: AgentId,
        guideline: Guideline,
    },
    Journey {
        agent_id: AgentId,
        journey: Journey,
    },
    ToolService {
        name: String,
        service: Arc<dyn ToolService>,
    },
}

/// A plugin is a named bundle of contributions that the server
/// applies at startup.
#[async_trait]
pub trait Plugin: Send + Sync {
    fn name(&self) -> &str;
    async fn contributions(&self) -> CoreResult<Vec<PluginContribution>>;
}

/// Registry that collects plugins and lets the server enumerate them
/// at startup. Phase 1: the server reads the contributions; persistence
/// and engine wiring happen elsewhere.
pub struct PluginRegistry {
    plugins: RwLock<Vec<Arc<dyn Plugin>>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: RwLock::new(Vec::new()),
        }
    }
    pub fn register(&self, plugin: Arc<dyn Plugin>) {
        self.plugins.write().push(plugin);
    }
    pub fn plugins(&self) -> Vec<Arc<dyn Plugin>> {
        self.plugins.read().clone()
    }
    pub async fn collect_all(&self) -> CoreResult<Vec<PluginContribution>> {
        let mut out = Vec::new();
        for p in self.plugins() {
            out.extend(p.contributions().await?);
        }
        Ok(out)
    }
    /// Group contributions by kind for ergonomic consumption.
    pub async fn grouped(&self) -> CoreResult<GroupedContributions> {
        let mut grouped = GroupedContributions::default();
        for c in self.collect_all().await? {
            match c {
                PluginContribution::Tool { agent_id, tool } => {
                    grouped.tools.entry(agent_id).or_default().push(tool);
                }
                PluginContribution::Guideline {
                    agent_id,
                    guideline,
                } => {
                    grouped
                        .guidelines
                        .entry(agent_id)
                        .or_default()
                        .push(guideline);
                }
                PluginContribution::Journey { agent_id, journey } => {
                    grouped.journeys.entry(agent_id).or_default().push(journey);
                }
                PluginContribution::ToolService { name, service } => {
                    grouped.tool_services.insert(name, service);
                }
            }
        }
        Ok(grouped)
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Default)]
pub struct GroupedContributions {
    pub tools: HashMap<AgentId, Vec<Tool>>,
    pub guidelines: HashMap<AgentId, Vec<Guideline>>,
    pub journeys: HashMap<AgentId, Vec<Journey>>,
    pub tool_services: HashMap<String, Arc<dyn ToolService>>,
}

/// Convenience `Plugin` impl that wraps a closure producing contributions.
/// Useful for inline plugins (e.g. test fixtures or quick scripts).
pub struct FunctionPlugin {
    name: String,
    func: Box<dyn Fn() -> Vec<PluginContribution> + Send + Sync>,
}

impl FunctionPlugin {
    pub fn new<F>(name: impl Into<String>, func: F) -> Self
    where
        F: Fn() -> Vec<PluginContribution> + Send + Sync + 'static,
    {
        Self {
            name: name.into(),
            func: Box::new(func),
        }
    }
}

#[async_trait]
impl Plugin for FunctionPlugin {
    fn name(&self) -> &str {
        &self.name
    }
    async fn contributions(&self) -> CoreResult<Vec<PluginContribution>> {
        Ok((self.func)())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AgentId, Criticality, Guideline, GuidelineContent, GuidelineId, Journey, JourneyId,
        JourneyNode, JourneyNodeId, NodeKind, Tool, ToolId, ToolKind,
    };

    struct StaticPlugin {
        name: String,
        contribs: Vec<PluginContribution>,
    }
    #[async_trait]
    impl Plugin for StaticPlugin {
        fn name(&self) -> &str {
            &self.name
        }
        async fn contributions(&self) -> CoreResult<Vec<PluginContribution>> {
            Ok(self.contribs.clone())
        }
    }

    fn make_tool(name: &str) -> Tool {
        Tool {
            id: ToolId::new(),
            name: name.into(),
            description: "test".into(),
            parameters_schema: serde_json::json!({}),
            kind: ToolKind::Local,
            creation_utc: chrono::Utc::now(),
        }
    }
    fn make_guideline() -> Guideline {
        Guideline {
            id: GuidelineId::new(),
            agent_id: AgentId::new(),
            content: GuidelineContent {
                condition: "x".into(),
                action: "y".into(),
                description: None,
            },
            criticality: Criticality::Low,
            enabled: true,
            tags: vec![],
            creation_utc: chrono::Utc::now(),
            metadata: serde_json::Value::Null,
        }
    }
    fn make_journey() -> Journey {
        let root = JourneyNode {
            id: JourneyNodeId::new(),
            kind: NodeKind::Initial,
            action: "start".into(),
            description: None,
            tools: vec![],
            labels: Default::default(),
            metadata: serde_json::Value::Null,
        };
        Journey {
            id: JourneyId::new(),
            agent_id: AgentId::new(),
            title: "j".into(),
            description: "d".into(),
            root_id: root.id,
            tags: vec![],
            creation_utc: chrono::Utc::now(),
        }
    }

    #[tokio::test]
    async fn registry_collects_all_contributions() {
        let reg = PluginRegistry::new();
        let agent = AgentId::new();
        reg.register(Arc::new(StaticPlugin {
            name: "p1".into(),
            contribs: vec![
                PluginContribution::Tool {
                    agent_id: agent.clone(),
                    tool: make_tool("t1"),
                },
                PluginContribution::Guideline {
                    agent_id: agent.clone(),
                    guideline: make_guideline(),
                },
            ],
        }));
        reg.register(Arc::new(StaticPlugin {
            name: "p2".into(),
            contribs: vec![PluginContribution::Journey {
                agent_id: agent.clone(),
                journey: make_journey(),
            }],
        }));
        let all = reg.collect_all().await.unwrap();
        assert_eq!(all.len(), 3);
    }

    #[tokio::test]
    async fn registry_groups_by_kind() {
        let reg = PluginRegistry::new();
        let agent = AgentId::new();
        reg.register(Arc::new(StaticPlugin {
            name: "p1".into(),
            contribs: vec![
                PluginContribution::Tool {
                    agent_id: agent.clone(),
                    tool: make_tool("t1"),
                },
                PluginContribution::Tool {
                    agent_id: agent.clone(),
                    tool: make_tool("t2"),
                },
                PluginContribution::Guideline {
                    agent_id: agent.clone(),
                    guideline: make_guideline(),
                },
            ],
        }));
        let grouped = reg.grouped().await.unwrap();
        assert_eq!(grouped.tools.get(&agent).unwrap().len(), 2);
        assert_eq!(grouped.guidelines.get(&agent).unwrap().len(), 1);
        assert!(grouped.journeys.is_empty());
    }

    #[test]
    fn registry_default_is_empty() {
        let reg = PluginRegistry::default();
        assert_eq!(reg.plugins().len(), 0);
    }

    #[test]
    fn function_plugin_invokes_closure() {
        let p = FunctionPlugin::new("inline", Vec::new);
        assert_eq!(p.name(), "inline");
    }

    #[tokio::test]
    async fn function_plugin_contributions_empty() {
        let p = FunctionPlugin::new("inline", Vec::new);
        assert!(p.contributions().await.unwrap().is_empty());
    }
}
