//! Criterion benchmarks for `AlphaEngine::process`.
//!
//! Establishes a baseline for engine performance. The fixtures use
//! `FakeNlpService` (no real LLM round-trip) so we measure the
//! engine's own overhead, not network latency.
//!
//! Run: `cargo bench -p loon-engine --bench alpha_engine`
//! Report: `target/criterion/alpha_engine/report/index.html` (when
//! `html_reports` feature is enabled).

use std::sync::Arc;

use criterion::{criterion_group, criterion_main, Criterion};
use tokio::runtime::Runtime;

use loon_core::{
    Agent, AgentId, Criticality, Guideline, GuidelineContent, GuidelineId, Relationship,
    RelationshipEntity, RelationshipEntityKind, RelationshipId, RelationshipKind,
    Session, SessionId, SessionMode,
};
use loon_core::entity_cq::{EntityCommands, EntityQueries};
use loon_core::stores::in_memory::InMemoryRelationshipStore;
use loon_core::stores::{RelationshipStore, SessionStore};

use loon_emission::EventBuffer;

use loon_engine::{
    alpha_engine::AlphaEngine,
    canned_response_generator::CannedResponseGenerator,
    engine::Engine,
    engine_context::{Context, EngineContext, GuidelineMatch, ToolInsights},
    hooks::EngineHooks,
    message_generator::MessageGenerator,
    optimization_policy::DefaultOptimizationPolicy,
    perceived_performance_policy::PerceivedPerformancePolicy,
    planner::{Plan, Planner},
    prompt_builder::PromptBuilder,
    relational_resolver::RelationalResolver,
    tool_calling::caller::{ToolCaller, ToolExecutionResult},
};

use loon_nlp::NlpService;
use loon_nlp::test_utils::FakeNlpService;

fn make_engine() -> AlphaEngine {
    let nlp: Arc<dyn NlpService> = Arc::new(FakeNlpService::new());
    let queries = EntityQueries::in_memory();
    let commands = Arc::new(EntityCommands {
        session_store: queries.session_store.clone(),
        context_variable_store: queries.context_variable_store.clone(),
    });

    // Stub NlpService-shaped types for engine dependencies
    struct NoopMatcher;
    #[async_trait::async_trait]
    impl loon_engine::guideline_matching::GuidelineMatcher for NoopMatcher {
        async fn match_guidelines(
            &self,
            _ctx: &loon_engine::guideline_matching::GuidelineMatchingContext,
        ) -> loon_engine::error::EngineResult<Vec<GuidelineMatch>> {
            Ok(vec![])
        }
    }

    struct NoopToolCaller;
    #[async_trait::async_trait]
    impl ToolCaller for NoopToolCaller {
        async fn generate_insights(
            &self,
            _ctx: &EngineContext,
            _guidelines: &[GuidelineMatch],
        ) -> loon_engine::error::EngineResult<ToolInsights> {
            Ok(ToolInsights::default())
        }
        async fn call_tools(
            &self,
            _ctx: &EngineContext,
            _insights: &ToolInsights,
        ) -> loon_engine::error::EngineResult<Vec<ToolExecutionResult>> {
            Ok(vec![])
        }
    }

    struct DonePlanner;
    #[async_trait::async_trait]
    impl Planner for DonePlanner {
        async fn plan(
            &self,
            _ctx: &EngineContext,
        ) -> loon_engine::error::EngineResult<Plan> {
            Ok(Plan::Done)
        }
    }

    struct WsTok;
    #[async_trait::async_trait]
    impl loon_nlp::Tokenizer for WsTok {
        async fn count_tokens(
            &self,
            text: &str,
        ) -> loon_nlp::NlpResult<u32> {
            Ok(text.split_whitespace().count() as u32)
        }
    }

    let prompt_builder = Arc::new(PromptBuilder::new(
        Arc::new(WsTok) as Arc<dyn loon_nlp::Tokenizer>,
        1000,
    ));
    let crg = Arc::new(CannedResponseGenerator::new(nlp.clone()));
    let message_generator = Arc::new(MessageGenerator::new(
        nlp.clone(),
        prompt_builder,
        crg,
    ));

    let matcher: Arc<dyn loon_engine::guideline_matching::GuidelineMatcher> =
        Arc::new(NoopMatcher);
    let tool_caller: Arc<dyn ToolCaller> = Arc::new(NoopToolCaller);
    let planner: Arc<dyn Planner> = Arc::new(DonePlanner);
    let rel_store: Arc<dyn loon_core::stores::RelationshipStore> =
        Arc::new(InMemoryRelationshipStore::new());
    let resolver = Arc::new(RelationalResolver::new(rel_store));
    let optimization_policy: Arc<dyn loon_engine::optimization_policy::OptimizationPolicy> =
        Arc::new(DefaultOptimizationPolicy);
    let performance_policy = Arc::new(PerceivedPerformancePolicy::new());
    let session_store: Arc<dyn SessionStore> = queries.session_store.clone();

    AlphaEngine {
        queries,
        commands,
        matcher,
        tool_caller,
        planner,
        message_generator,
        relational_resolver: resolver,
        hooks: EngineHooks::default(),
        optimization_policy,
        performance_policy,
        session_store,
        nlp,
    }
}

async fn seed(engine: &AlphaEngine) -> (AgentId, SessionId) {
    let queries = &engine.queries;
    let agent = Agent::new("bench", "benchmark agent");
    let agent_id = agent.id.clone();
    queries.agent_store.create(agent).await.unwrap();
    let mut session = Session::new(&agent_id);
    session.mode = SessionMode::Auto;
    let session_id = session.id.clone();
    queries.session_store.create(session).await.unwrap();
    (agent_id, session_id)
}

fn bench_process(c: &mut Criterion) {
    let runtime = Runtime::new().unwrap();
    let engine = make_engine();
    let (agent_id, session_id) = runtime.block_on(seed(&engine));
    let ctx = Context { session_id, agent_id };
    let emitter = EventBuffer::new(Agent::new("bench", "x"));

    c.bench_function("alpha_engine_process", |b| {
        b.iter(|| {
            runtime.block_on(async {
                let result = engine.process(&ctx, &emitter).await.unwrap();
                assert!(result);
            });
        });
    });
}

fn bench_engine_context_construction(c: &mut Criterion) {
    c.bench_function("engine_context_placeholder", |b| {
        b.iter(|| {
            let _ = EngineContext::placeholder();
        });
    });
}

fn bench_relational_resolver(c: &mut Criterion) {
    let runtime = Runtime::new().unwrap();
    let store: Arc<dyn RelationshipStore> = Arc::new(InMemoryRelationshipStore::new());
    let resolver = RelationalResolver::new(store.clone());

    let mut handles = Vec::new();
    for _ in 0..100 {
        handles.push(GuidelineId::new());
    }
    for i in 0..50 {
        let rel = Relationship {
            id: RelationshipId::new(),
            source: RelationshipEntity {
                kind: RelationshipEntityKind::Guideline,
                id: handles[i].0.clone(),
            },
            target: RelationshipEntity {
                kind: RelationshipEntityKind::Guideline,
                id: handles[i + 50].0.clone(),
            },
            kind: RelationshipKind::Excludes,
            indirect: false,
            creation_utc: chrono::Utc::now(),
        };
        runtime.block_on(store.create(rel)).unwrap();
    }
    let matches: Vec<GuidelineMatch> = (0..100)
        .map(|i| GuidelineMatch {
            guideline: Guideline {
                id: handles[i].clone(),
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
            },
            confidence: 1.0,
            rationale: "test".into(),
        })
        .collect();

    c.bench_function("relational_resolver_exclusions_100g_50r", |b| {
        b.iter(|| {
            runtime.block_on(async {
                let _ = resolver.resolve_exclusions(matches.clone()).await.unwrap();
            });
        });
    });
}

criterion_group!(
    benches,
    bench_process,
    bench_engine_context_construction,
    bench_relational_resolver,
);
criterion_main!(benches);
