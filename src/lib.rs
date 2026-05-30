use std::collections::HashMap;
use serde::{Deserialize, Serialize};

// ============================================================================
// 1. IntentionId
// ============================================================================

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct IntentionId(String);

impl IntentionId {
    pub fn new(s: &str) -> Self {
        Self(s.to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for IntentionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for IntentionId {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

// ============================================================================
// 2. IntentionOrigin
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IntentionOrigin {
    Human(String),
    Agent(String),
    System,
    Emergent(String),
}

impl std::fmt::Display for IntentionOrigin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IntentionOrigin::Human(name) => write!(f, "Human({})", name),
            IntentionOrigin::Agent(name) => write!(f, "Agent({})", name),
            IntentionOrigin::System => write!(f, "System"),
            IntentionOrigin::Emergent(label) => write!(f, "Emergent({})", label),
        }
    }
}

// ============================================================================
// 3. IntentionStatus
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IntentionStatus {
    Forming,
    Ready,
    Executing,
    Completed,
    Failed,
    Transformed(IntentionId),
}

impl std::fmt::Display for IntentionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IntentionStatus::Forming => write!(f, "Forming"),
            IntentionStatus::Ready => write!(f, "Ready"),
            IntentionStatus::Executing => write!(f, "Executing"),
            IntentionStatus::Completed => write!(f, "Completed"),
            IntentionStatus::Failed => write!(f, "Failed"),
            IntentionStatus::Transformed(id) => write!(f, "Transformed({})", id),
        }
    }
}

// ============================================================================
// 4. Intention — the core primitive
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intention {
    pub id: IntentionId,
    pub goal: String,
    pub origin: IntentionOrigin,
    pub priority: f64,
    pub tick_created: u64,
    pub required_capabilities: Vec<String>,
    pub conservation_budget: f64,
    pub status: IntentionStatus,
}

static INTENTION_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

impl Intention {
    pub fn new(goal: &str, origin: IntentionOrigin, priority: f64, tick: u64) -> Self {
        let seq = INTENTION_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let id = IntentionId::new(&format!("int-{:016x}-{:010x}", tick, seq));
        Self {
            id,
            goal: goal.to_string(),
            origin,
            priority: priority.clamp(0.0, 1.0),
            tick_created: tick,
            required_capabilities: Vec::new(),
            conservation_budget: 0.0,
            status: IntentionStatus::Forming,
        }
    }

    pub fn require(&mut self, capability: &str) {
        if !self.required_capabilities.contains(&capability.to_string()) {
            self.required_capabilities.push(capability.to_string());
        }
    }

    pub fn allocate(&mut self, budget: f64) {
        self.conservation_budget = budget.max(0.0);
    }

    pub fn ready(&mut self) -> bool {
        if !matches!(self.status, IntentionStatus::Forming) {
            return false;
        }
        if self.required_capabilities.is_empty() {
            return false;
        }
        if self.conservation_budget <= 0.0 {
            return false;
        }
        self.status = IntentionStatus::Ready;
        true
    }

    pub fn execute(&mut self) {
        if matches!(self.status, IntentionStatus::Ready) {
            self.status = IntentionStatus::Executing;
        }
    }

    pub fn complete(&mut self) {
        if matches!(self.status, IntentionStatus::Executing) {
            self.status = IntentionStatus::Completed;
        }
    }

    pub fn fail(&mut self) {
        self.status = IntentionStatus::Failed;
    }

    pub fn transform(&mut self, _new_goal: &str) -> IntentionId {
        let seq = INTENTION_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let new_id = IntentionId::new(&format!(
            "int-{:016x}-{:010x}",
            self.tick_created + 1,
            seq
        ));
        self.status = IntentionStatus::Transformed(new_id.clone());
        new_id
    }
}

// ============================================================================
// 5. IntentionGraph — the execution engine
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentionGraph {
    pub intentions: HashMap<IntentionId, Intention>,
    pub dependencies: HashMap<IntentionId, Vec<IntentionId>>,
    pub conservation_pool: f64,
    pub tick: u64,
}

impl Default for IntentionGraph {
    fn default() -> Self {
        Self {
            intentions: HashMap::new(),
            dependencies: HashMap::new(),
            conservation_pool: 1000.0,
            tick: 0,
        }
    }
}

impl IntentionGraph {
    pub fn new(conservation_pool: f64) -> Self {
        Self {
            intentions: HashMap::new(),
            dependencies: HashMap::new(),
            conservation_pool: conservation_pool.max(0.0),
            tick: 0,
        }
    }

    pub fn register(&mut self, intention: Intention) -> IntentionId {
        let id = intention.id.clone();
        self.intentions.insert(id.clone(), intention);
        self.dependencies.entry(id.clone()).or_default();
        id
    }

    pub fn depends_on(&mut self, child: &IntentionId, parent: &IntentionId) {
        if self.intentions.contains_key(child) && self.intentions.contains_key(parent) {
            self.dependencies
                .entry(child.clone())
                .or_default()
                .push(parent.clone());
        }
    }

    pub fn allocate_energy(&mut self, id: &IntentionId, amount: f64) -> bool {
        let amount = amount.max(0.0);
        let total_allocated: f64 = self
            .intentions
            .values()
            .map(|i| i.conservation_budget)
            .sum();
        if total_allocated + amount <= self.conservation_pool {
            if let Some(intention) = self.intentions.get_mut(id) {
                intention.allocate(amount);
                return true;
            }
        }
        false
    }

    pub fn execute(&mut self, id: &IntentionId) -> Result<(), String> {
        let intention = self
            .intentions
            .get(id)
            .ok_or_else(|| format!("Intention {} not found", id))?;
        if let Some(deps) = self.dependencies.get(id) {
            for dep in deps {
                if let Some(dep_int) = self.intentions.get(dep) {
                    match dep_int.status {
                        IntentionStatus::Completed => {}
                        _ => {
                            return Err(format!(
                                "Dependency {} for {} is not completed (status: {:?})",
                                dep, id, dep_int.status
                            ));
                        }
                    }
                }
            }
        }
        if intention.conservation_budget <= 0.0 {
            return Err(format!("Intention {} has no conservation budget allocated", id));
        }
        let intention = self.intentions.get_mut(id).unwrap();
        intention.execute();
        Ok(())
    }

    pub fn execute_ready(&mut self) -> Vec<IntentionId> {
        let ready_ids: Vec<IntentionId> = self
            .intentions
            .iter()
            .filter(|(_, i)| matches!(i.status, IntentionStatus::Ready))
            .map(|(id, _)| id.clone())
            .collect();
        ready_ids
            .into_iter()
            .filter(|id| self.execute(id).is_ok())
            .collect()
    }

    pub fn propagate(&mut self) {
        let forming: Vec<IntentionId> = self
            .intentions
            .iter()
            .filter(|(_, i)| matches!(i.status, IntentionStatus::Forming))
            .map(|(id, _)| id.clone())
            .collect();
        for fid in &forming {
            let all_deps_done = self
                .dependencies
                .get(fid)
                .map(|deps| {
                    deps.iter().all(|d| {
                        self.intentions
                            .get(d)
                            .map(|i| matches!(i.status, IntentionStatus::Completed))
                            .unwrap_or(false)
                    })
                })
                .unwrap_or(true);
            if all_deps_done {
                if let Some(intention) = self.intentions.get_mut(fid) {
                    if !intention.required_capabilities.is_empty()
                        && intention.conservation_budget > 0.0
                    {
                        let _ = intention.ready();
                    }
                }
            }
        }
    }

    pub fn is_conserved(&self) -> bool {
        let total: f64 = self
            .intentions
            .values()
            .map(|i| i.conservation_budget)
            .sum();
        total <= self.conservation_pool + 1e-9
    }

    pub fn energy_flow(&self) -> HashMap<IntentionId, f64> {
        self.intentions
            .iter()
            .map(|(id, i)| (id.clone(), i.conservation_budget))
            .collect()
    }

    pub fn bottlenecks(&self) -> Vec<&Intention> {
        let mut blockers = Vec::new();
        for parent_id in self.intentions.keys() {
            let has_dependents = self.dependencies.values().any(|deps| deps.contains(parent_id));
            if has_dependents {
                if let Some(intention) = self.intentions.get(parent_id) {
                    if !matches!(intention.status, IntentionStatus::Completed) {
                        blockers.push(intention);
                    }
                }
            }
        }
        blockers
    }

    pub fn frontier(&self) -> Vec<&Intention> {
        self.intentions
            .values()
            .filter(|i| matches!(i.status, IntentionStatus::Ready))
            .collect()
    }

    pub fn top_intentions(&self, n: usize) -> Vec<&Intention> {
        let mut all: Vec<&Intention> = self.intentions.values().collect();
        all.sort_by(|a, b| b.priority.partial_cmp(&a.priority).unwrap_or(std::cmp::Ordering::Equal));
        all.into_iter().take(n).collect()
    }

    pub fn graph_summary(&self) -> String {
        let total = self.intentions.len();
        let ready = self.intentions.values().filter(|i| matches!(i.status, IntentionStatus::Ready)).count();
        let executing = self.intentions.values().filter(|i| matches!(i.status, IntentionStatus::Executing)).count();
        let completed = self.intentions.values().filter(|i| matches!(i.status, IntentionStatus::Completed)).count();
        let failed = self.intentions.values().filter(|i| matches!(i.status, IntentionStatus::Failed)).count();
        let total_budget: f64 = self.intentions.values().map(|i| i.conservation_budget).sum();
        let edge_count: usize = self.dependencies.values().map(|v| v.len()).sum();
        format!(
            "IntentionGraph(tick={}, intentions={}, edges={}, ready={}, executing={}, completed={}, failed={}, pool={:.1}, allocated={:.1}, conserved={})",
            self.tick, total, edge_count, ready, executing, completed, failed,
            self.conservation_pool, total_budget, self.is_conserved()
        )
    }
}

// ============================================================================
// 6. SoulSignature
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoulSignature {
    pub patience: f64,
    pub precision: f64,
    pub playfulness: f64,
    pub conservation_affinity: f64,
}

impl Default for SoulSignature {
    fn default() -> Self {
        Self {
            patience: 0.5,
            precision: 0.5,
            playfulness: 0.5,
            conservation_affinity: 0.5,
        }
    }
}

impl SoulSignature {
    pub fn new(patience: f64, precision: f64, playfulness: f64, conservation_affinity: f64) -> Self {
        Self {
            patience: patience.clamp(0.0, 1.0),
            precision: precision.clamp(0.0, 1.0),
            playfulness: playfulness.clamp(0.0, 1.0),
            conservation_affinity: conservation_affinity.clamp(0.0, 1.0),
        }
    }
}

// ============================================================================
// 7. ExecutionResult
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub success: bool,
    pub energy_consumed: f64,
    pub insight: Option<String>,
    pub artifacts: Vec<String>,
}

impl ExecutionResult {
    pub fn success(energy: f64) -> Self {
        Self {
            success: true,
            energy_consumed: energy,
            insight: None,
            artifacts: Vec::new(),
        }
    }

    pub fn failure(energy: f64, reason: &str) -> Self {
        Self {
            success: false,
            energy_consumed: energy,
            insight: Some(reason.to_string()),
            artifacts: Vec::new(),
        }
    }
}

// ============================================================================
// 8. AgentModule — like nn.Module but for agent capabilities
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentModule {
    pub agent_id: String,
    pub capabilities: Vec<String>,
    pub specializations: Vec<String>,
    pub energy_capacity: f64,
    pub energy_used: f64,
    pub intention_history: Vec<IntentionId>,
    pub soul_signature: SoulSignature,
}

impl AgentModule {
    pub fn new(agent_id: &str, capabilities: Vec<String>, capacity: f64) -> Self {
        Self {
            agent_id: agent_id.to_string(),
            capabilities,
            specializations: Vec::new(),
            energy_capacity: capacity.max(0.0),
            energy_used: 0.0,
            intention_history: Vec::new(),
            soul_signature: SoulSignature::default(),
        }
    }

    pub fn energy_available(&self) -> f64 {
        (self.energy_capacity - self.energy_used).max(0.0)
    }

    pub fn can_execute(&self, intention: &Intention) -> bool {
        let has_capabilities = intention
            .required_capabilities
            .iter()
            .all(|cap| self.capabilities.contains(cap));
        let has_energy = self.energy_available() >= intention.conservation_budget;
        has_capabilities && has_energy
    }

    pub fn execute(&mut self, intention: &mut Intention) -> ExecutionResult {
        let energy_needed = intention.conservation_budget;
        if !self.can_execute(intention) {
            return ExecutionResult::failure(0.0, "Insufficient capabilities or energy");
        }
        self.energy_used += energy_needed;
        self.intention_history.push(intention.id.clone());
        intention.execute();
        intention.complete();
        ExecutionResult::success(energy_needed)
    }

    pub fn rest(&mut self, amount: f64) {
        self.energy_used = (self.energy_used - amount).max(0.0);
    }

    pub fn learn_capability(&mut self, cap: &str) {
        if !self.capabilities.contains(&cap.to_string()) {
            self.capabilities.push(cap.to_string());
        }
    }

    pub fn is_exhausted(&self) -> bool {
        self.energy_available() <= 1e-9
    }

    pub fn utilization(&self) -> f64 {
        if self.energy_capacity <= 0.0 {
            return 0.0;
        }
        (self.energy_used / self.energy_capacity).clamp(0.0, 1.0)
    }
}

// ============================================================================
// 9. TickResult & RuntimeStatus
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TickResult {
    pub executed: Vec<IntentionId>,
    pub completed: Vec<IntentionId>,
    pub failed: Vec<IntentionId>,
    pub energy_consumed: f64,
    pub energy_remaining: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeStatus {
    pub total_intentions: usize,
    pub active: usize,
    pub completed: usize,
    pub failed: usize,
    pub agents_available: usize,
    pub agents_exhausted: usize,
    pub energy_utilization: f64,
    pub throughput: f64,
}

// ============================================================================
// 10. IntentionRuntime — THE SERVE LAYER
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentionRuntime {
    pub graph: IntentionGraph,
    pub agents: HashMap<String, AgentModule>,
    pub tick: u64,
    pub global_budget: f64,
}

impl IntentionRuntime {
    pub fn new(global_budget: f64) -> Self {
        Self {
            graph: IntentionGraph::new(global_budget),
            agents: HashMap::new(),
            tick: 0,
            global_budget: global_budget.max(0.0),
        }
    }

    pub fn register_agent(&mut self, agent: AgentModule) {
        self.agents.insert(agent.agent_id.clone(), agent);
    }

    pub fn submit(&mut self, mut intention: Intention) -> IntentionId {
        intention.tick_created = self.tick;
        let id = intention.id.clone();
        self.graph.register(intention);
        id
    }

    pub fn tick(&mut self) -> TickResult {
        self.tick += 1;
        self.graph.tick = self.tick;
        let assignments = self.auto_assign();
        let mut executed = Vec::new();
        let mut completed = Vec::new();
        let mut failed = Vec::new();
        let mut energy_consumed = 0.0;
        for (intention_id, agent_id) in &assignments {
            if let Some(agent) = self.agents.get_mut(agent_id) {
                if let Some(intention) = self.graph.intentions.get_mut(intention_id) {
                    if matches!(intention.status, IntentionStatus::Ready) {
                        let result = agent.execute(intention);
                        energy_consumed += result.energy_consumed;
                        executed.push(intention_id.clone());
                        if result.success {
                            completed.push(intention_id.clone());
                        } else {
                            failed.push(intention_id.clone());
                        }
                    }
                }
            }
        }
        self.graph.propagate();
        TickResult {
            executed,
            completed,
            failed,
            energy_consumed,
            energy_remaining: self.global_budget - energy_consumed,
        }
    }

    pub fn assign(&mut self, intention_id: &IntentionId, agent_id: &str) -> bool {
        self.agents.contains_key(agent_id) && self.graph.intentions.contains_key(intention_id)
    }

    pub fn auto_assign(&mut self) -> Vec<(IntentionId, String)> {
        let mut assignments = Vec::new();
        let ready: Vec<IntentionId> = self
            .graph
            .intentions
            .iter()
            .filter(|(_, i)| matches!(i.status, IntentionStatus::Ready))
            .map(|(id, _)| id.clone())
            .collect();
        for id in ready {
            let intention = self.graph.intentions.get(&id).unwrap();
            let mut best_agent: Option<String> = None;
            let mut best_score: f64 = -1.0;
            for (agent_id, agent) in &self.agents {
                if agent.can_execute(intention) {
                    let cap_match = intention
                        .required_capabilities
                        .iter()
                        .filter(|c| agent.capabilities.contains(c))
                        .count() as f64
                        / intention.required_capabilities.len().max(1) as f64;
                    let affinity_score = agent.soul_signature.conservation_affinity;
                    let score = cap_match * 0.7 + affinity_score * 0.3;
                    if score > best_score {
                        best_score = score;
                        best_agent = Some(agent_id.clone());
                    }
                }
            }
            if let Some(agent_id) = best_agent {
                assignments.push((id, agent_id));
            }
        }
        assignments
    }

    pub fn status(&self) -> RuntimeStatus {
        let total = self.graph.intentions.len();
        let active = self
            .graph
            .intentions
            .values()
            .filter(|i| matches!(i.status, IntentionStatus::Ready | IntentionStatus::Executing))
            .count();
        let completed = self
            .graph
            .intentions
            .values()
            .filter(|i| matches!(i.status, IntentionStatus::Completed))
            .count();
        let failed = self
            .graph
            .intentions
            .values()
            .filter(|i| matches!(i.status, IntentionStatus::Failed))
            .count();
        let agents_available = self.agents.values().filter(|a| !a.is_exhausted()).count();
        let agents_exhausted = self.agents.values().filter(|a| a.is_exhausted()).count();
        let total_energy: f64 = self.agents.values().map(|a| a.energy_used).sum();
        let total_capacity: f64 = self.agents.values().map(|a| a.energy_capacity).sum();
        let energy_utilization = if total_capacity > 0.0 { total_energy / total_capacity } else { 0.0 };
        RuntimeStatus {
            total_intentions: total,
            active,
            completed,
            failed,
            agents_available,
            agents_exhausted,
            energy_utilization,
            throughput: if self.tick > 0 { completed as f64 / self.tick as f64 } else { 0.0 },
        }
    }
}

// ============================================================================
// 11. IntentionCompiler
// ============================================================================

pub struct IntentionCompiler;

impl IntentionCompiler {
    pub fn compile(goal: &str, available_agents: &[AgentModule], budget: f64, tick: u64) -> IntentionGraph {
        let mut graph = IntentionGraph::new(budget);
        let goal_lower = goal.to_lowercase();
        if goal_lower.starts_with("build") {
            Self::compile_build(&mut graph, goal, tick);
        } else if goal_lower.starts_with("train") {
            Self::compile_train(&mut graph, goal, tick);
        } else if goal_lower.contains("conservation") || goal_lower.contains("balance") {
            Self::compile_conservation(&mut graph, goal, tick);
        } else if goal_lower.starts_with("explore") {
            Self::compile_explore(&mut graph, goal, tick);
        } else {
            let mut intention = Intention::new(goal, IntentionOrigin::System, 0.5, tick);
            if let Some(agent) = available_agents.first() {
                for cap in &agent.capabilities {
                    intention.require(cap);
                }
            }
            intention.allocate(budget * 0.8);
            let _ = intention.ready();
            graph.register(intention);
        }
        graph
    }

    fn compile_build(graph: &mut IntentionGraph, goal: &str, tick: u64) {
        let sub_budget = graph.conservation_pool / 4.0;
        let mut acquire = Intention::new(&format!("acquire_materials_for_{}", goal), IntentionOrigin::System, 0.7, tick);
        acquire.require("scout");
        acquire.allocate(sub_budget);
        let _ = acquire.ready();
        let acquire_id = graph.register(acquire);
        let mut design = Intention::new(&format!("design_{}", goal), IntentionOrigin::System, 0.8, tick);
        design.require("design");
        design.allocate(sub_budget);
        let design_id = graph.register(design);
        let mut construct = Intention::new(&format!("construct_{}", goal), IntentionOrigin::System, 0.9, tick);
        construct.require("place");
        construct.require("construct");
        construct.allocate(sub_budget);
        let construct_id = graph.register(construct);
        let mut verify = Intention::new(&format!("verify_{}", goal), IntentionOrigin::System, 0.6, tick);
        verify.require("verify");
        verify.allocate(sub_budget);
        let verify_id = graph.register(verify);
        graph.depends_on(&design_id, &acquire_id);
        graph.depends_on(&construct_id, &design_id);
        graph.depends_on(&verify_id, &construct_id);
    }

    fn compile_train(graph: &mut IntentionGraph, goal: &str, tick: u64) {
        let sub_budget = graph.conservation_pool / 4.0;
        let mut enroll = Intention::new(&format!("enroll_training_for_{}", goal), IntentionOrigin::System, 0.5, tick);
        enroll.require("coordinate");
        enroll.allocate(sub_budget);
        let _ = enroll.ready();
        let enroll_id = graph.register(enroll);
        let mut curriculum = Intention::new(&format!("complete_curriculum_for_{}", goal), IntentionOrigin::System, 0.7, tick);
        curriculum.require("analyze");
        curriculum.allocate(sub_budget);
        let curriculum_id = graph.register(curriculum);
        let mut graduate = Intention::new(&format!("graduate_from_{}", goal), IntentionOrigin::System, 0.8, tick);
        graduate.require("evaluate");
        graduate.allocate(sub_budget);
        let graduate_id = graph.register(graduate);
        let mut certify = Intention::new(&format!("certify_{}", goal), IntentionOrigin::System, 0.6, tick);
        certify.require("verify");
        certify.allocate(sub_budget);
        let certify_id = graph.register(certify);
        graph.depends_on(&curriculum_id, &enroll_id);
        graph.depends_on(&graduate_id, &curriculum_id);
        graph.depends_on(&certify_id, &graduate_id);
    }

    fn compile_conservation(graph: &mut IntentionGraph, goal: &str, tick: u64) {
        let sub_budget = graph.conservation_pool / 5.0;
        let mut measure_in = Intention::new(&format!("measure_inputs_for_{}", goal), IntentionOrigin::System, 0.7, tick);
        measure_in.require("scout");
        measure_in.allocate(sub_budget);
        let _ = measure_in.ready();
        let measure_in_id = graph.register(measure_in);
        let mut measure_out = Intention::new(&format!("measure_outputs_for_{}", goal), IntentionOrigin::System, 0.7, tick);
        measure_out.require("scout");
        measure_out.allocate(sub_budget);
        let measure_out_id = graph.register(measure_out);
        let mut compute_err = Intention::new(&format!("compute_error_for_{}", goal), IntentionOrigin::System, 0.8, tick);
        compute_err.require("compute");
        compute_err.allocate(sub_budget);
        let compute_err_id = graph.register(compute_err);
        let mut correct = Intention::new(&format!("correct_error_for_{}", goal), IntentionOrigin::System, 0.9, tick);
        correct.require("analyze");
        correct.allocate(sub_budget);
        let correct_id = graph.register(correct);
        let mut verify = Intention::new(&format!("verify_balance_for_{}", goal), IntentionOrigin::System, 0.6, tick);
        verify.require("verify");
        verify.allocate(sub_budget);
        let verify_id = graph.register(verify);
        graph.depends_on(&compute_err_id, &measure_in_id);
        graph.depends_on(&compute_err_id, &measure_out_id);
        graph.depends_on(&correct_id, &compute_err_id);
        graph.depends_on(&verify_id, &correct_id);
    }

    fn compile_explore(graph: &mut IntentionGraph, goal: &str, tick: u64) {
        let sub_budget = graph.conservation_pool / 4.0;
        let mut scout = Intention::new(&format!("scout_{}", goal), IntentionOrigin::System, 0.7, tick);
        scout.require("scout");
        scout.allocate(sub_budget);
        let _ = scout.ready();
        let scout_id = graph.register(scout);
        let mut map = Intention::new(&format!("map_{}", goal), IntentionOrigin::System, 0.8, tick);
        map.require("map");
        map.allocate(sub_budget);
        let map_id = graph.register(map);
        let mut identify = Intention::new(&format!("identify_resources_in_{}", goal), IntentionOrigin::System, 0.7, tick);
        identify.require("identify");
        identify.allocate(sub_budget);
        let identify_id = graph.register(identify);
        let mut report = Intention::new(&format!("report_on_{}", goal), IntentionOrigin::System, 0.6, tick);
        report.require("evaluate");
        report.allocate(sub_budget);
        let report_id = graph.register(report);
        graph.depends_on(&map_id, &scout_id);
        graph.depends_on(&identify_id, &map_id);
        graph.depends_on(&report_id, &identify_id);
    }
}

// ============================================================================
// 12. Pre-built Agent Modules
// ============================================================================

pub fn builder_agent() -> AgentModule {
    let mut agent = AgentModule::new("Builder", vec!["place".to_string(), "design".to_string(), "construct".to_string()], 100.0);
    agent.soul_signature = SoulSignature::new(0.5, 0.8, 0.3, 0.6);
    agent
}

pub fn scout_agent() -> AgentModule {
    let mut agent = AgentModule::new("Scout", vec!["scout".to_string(), "map".to_string(), "identify".to_string()], 60.0);
    agent.soul_signature = SoulSignature::new(0.4, 0.5, 0.7, 0.4);
    agent
}

pub fn scholar_agent() -> AgentModule {
    let mut agent = AgentModule::new("Scholar", vec!["analyze".to_string(), "compute".to_string(), "verify".to_string()], 80.0);
    agent.soul_signature = SoulSignature::new(0.6, 0.9, 0.2, 0.95);
    agent
}

pub fn captain_agent() -> AgentModule {
    let mut agent = AgentModule::new("Captain", vec!["coordinate".to_string(), "assign".to_string(), "evaluate".to_string()], 50.0);
    agent.soul_signature = SoulSignature::new(0.9, 0.7, 0.4, 0.5);
    agent
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[test]
    fn test_intention_new() {
        let intention = Intention::new("build a bridge", IntentionOrigin::Human("Alice".into()), 0.8, 1);
        assert_eq!(intention.goal, "build a bridge");
        assert_eq!(intention.priority, 0.8);
        assert!(matches!(intention.origin, IntentionOrigin::Human(_)));
        assert!(matches!(intention.status, IntentionStatus::Forming));
        assert!(intention.required_capabilities.is_empty());
        assert_eq!(intention.conservation_budget, 0.0);
    }

    #[test]
    #[test]
    fn test_intention_priority_clamped() {
        let too_high = Intention::new("test", IntentionOrigin::System, 1.5, 0);
        assert_eq!(too_high.priority, 1.0);
        let too_low = Intention::new("test", IntentionOrigin::System, -0.5, 0);
        assert_eq!(too_low.priority, 0.0);
    }

    #[test]
    #[test]
    fn test_intention_require_and_allocate() {
        let mut intention = Intention::new("cook dinner", IntentionOrigin::Human("Bob".into()), 0.6, 2);
        intention.require("analyze");
        intention.require("place");
        intention.allocate(50.0);
        assert_eq!(intention.required_capabilities.len(), 2);
        assert_eq!(intention.conservation_budget, 50.0);
    }

    #[test]
    #[test]
    fn test_intention_require_dedup() {
        let mut intention = Intention::new("test", IntentionOrigin::System, 0.5, 0);
        intention.require("analyze");
        intention.require("analyze");
        assert_eq!(intention.required_capabilities.len(), 1);
    }

    #[test]
    #[test]
    fn test_intention_ready() {
        let mut intention = Intention::new("test", IntentionOrigin::System, 0.5, 0);
        assert!(!intention.ready());
        intention.require("analyze");
        assert!(!intention.ready());
        intention.allocate(10.0);
        assert!(intention.ready());
        assert!(matches!(intention.status, IntentionStatus::Ready));
        assert!(!intention.ready());
    }

    #[test]
    #[test]
    fn test_intention_lifecycle() {
        let mut intention = Intention::new("test", IntentionOrigin::System, 0.5, 0);
        intention.require("verify");
        intention.allocate(10.0);
        assert!(intention.ready());
        intention.execute();
        assert!(matches!(intention.status, IntentionStatus::Executing));
        intention.complete();
        assert!(matches!(intention.status, IntentionStatus::Completed));
    }

    #[test]
    #[test]
    fn test_intention_fail() {
        let mut intention = Intention::new("test", IntentionOrigin::System, 0.5, 0);
        intention.fail();
        assert!(matches!(intention.status, IntentionStatus::Failed));
    }

    #[test]
    #[test]
    fn test_intention_transform() {
        let mut intention = Intention::new("old goal", IntentionOrigin::System, 0.5, 0);
        intention.require("analyze");
        intention.allocate(10.0);
        let new_id = intention.transform("new goal");
        assert!(matches!(intention.status, IntentionStatus::Transformed(_)));
        assert!(new_id.as_str().starts_with("int-"));
        if let IntentionStatus::Transformed(ref stored_id) = intention.status {
            assert_eq!(stored_id.as_str(), new_id.as_str());
        }
    }

    #[test]
    #[test]
    fn test_intention_origin_display() {
        assert_eq!(IntentionOrigin::Human("Alice".into()).to_string(), "Human(Alice)");
        assert_eq!(IntentionOrigin::Agent("Bot".into()).to_string(), "Agent(Bot)");
        assert_eq!(IntentionOrigin::System.to_string(), "System");
        assert_eq!(IntentionOrigin::Emergent("chaos".into()).to_string(), "Emergent(chaos)");
    }

    #[test]
    #[test]
    fn test_intention_id_newtype() {
        let id1 = IntentionId::new("abc");
        let id2 = IntentionId::new("abc");
        let id3 = IntentionId::new("xyz");
        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
        assert_eq!(id1.as_str(), "abc");
        assert_eq!(format!("{}", id1), "abc");
    }

    #[test]
    #[test]
    fn test_graph_register() {
        let mut graph = IntentionGraph::new(100.0);
        let intention = Intention::new("test", IntentionOrigin::System, 0.5, 0);
        let id = graph.register(intention);
        assert_eq!(graph.intentions.len(), 1);
        assert!(graph.intentions.contains_key(&id));
    }

    #[test]
    #[test]
    fn test_graph_default() {
        let graph = IntentionGraph::default();
        assert_eq!(graph.conservation_pool, 1000.0);
        assert!(graph.intentions.is_empty());
    }

    #[test]
    #[test]
    fn test_graph_depends_on() {
        let mut graph = IntentionGraph::new(100.0);
        let a_id = graph.register(Intention::new("A", IntentionOrigin::System, 0.5, 0));
        let b_id = graph.register(Intention::new("B", IntentionOrigin::System, 0.5, 0));
        graph.depends_on(&b_id, &a_id);
        assert_eq!(graph.dependencies[&b_id].len(), 1);
        assert_eq!(graph.dependencies[&b_id][0], a_id);
    }

    #[test]
    #[test]
    fn test_graph_allocate_energy_within_budget() {
        let mut graph = IntentionGraph::new(100.0);
        let id = graph.register(Intention::new("test", IntentionOrigin::System, 0.5, 0));
        assert!(graph.allocate_energy(&id, 50.0));
        assert_eq!(graph.intentions[&id].conservation_budget, 50.0);
    }

    #[test]
    #[test]
    fn test_graph_allocate_energy_exceeds_budget() {
        let mut graph = IntentionGraph::new(100.0);
        let id = graph.register(Intention::new("test", IntentionOrigin::System, 0.5, 0));
        assert!(!graph.allocate_energy(&id, 200.0));
    }

    #[test]
    #[test]
    fn test_graph_is_conserved() {
        let mut graph = IntentionGraph::new(100.0);
        let id1 = graph.register(Intention::new("a", IntentionOrigin::System, 0.5, 0));
        let id2 = graph.register(Intention::new("b", IntentionOrigin::System, 0.5, 0));
        graph.allocate_energy(&id1, 30.0);
        graph.allocate_energy(&id2, 50.0);
        assert!(graph.is_conserved());
    }

    #[test]
    #[test]
    fn test_graph_execute_requires_deps() {
        let mut graph = IntentionGraph::new(100.0);
        let a_id = graph.register(Intention::new("A", IntentionOrigin::System, 0.5, 0));
        let b_id = graph.register(Intention::new("B", IntentionOrigin::System, 0.5, 0));
        graph.depends_on(&b_id, &a_id);
        let a = graph.intentions.get_mut(&a_id).unwrap();
        a.require("verify");
        a.allocate(10.0);
        let _ = a.ready();
        a.execute();
        a.complete();
        let b = graph.intentions.get_mut(&b_id).unwrap();
        b.require("verify");
        b.allocate(10.0);
        let _ = b.ready();
        assert!(graph.execute(&b_id).is_ok());
        assert!(matches!(graph.intentions[&b_id].status, IntentionStatus::Executing));
    }

    #[test]
    #[test]
    fn test_graph_execute_fails_unsatisfied_dep() {
        let mut graph = IntentionGraph::new(100.0);
        let a_id = graph.register(Intention::new("A", IntentionOrigin::System, 0.5, 0));
        let b_id = graph.register(Intention::new("B", IntentionOrigin::System, 0.5, 0));
        graph.depends_on(&b_id, &a_id);
        let b = graph.intentions.get_mut(&b_id).unwrap();
        b.require("verify");
        b.allocate(10.0);
        let _ = b.ready();
        assert!(graph.execute(&b_id).is_err());
    }

    #[test]
    #[test]
    fn test_graph_execute_fails_no_budget() {
        let mut graph = IntentionGraph::new(100.0);
        let id = graph.register(Intention::new("test", IntentionOrigin::System, 0.5, 0));
        assert!(graph.execute(&id).is_err());
    }

    #[test]
    #[test]
    fn test_graph_energy_flow() {
        let mut graph = IntentionGraph::new(100.0);
        let id1 = graph.register(Intention::new("a", IntentionOrigin::System, 0.5, 0));
        let id2 = graph.register(Intention::new("b", IntentionOrigin::System, 0.5, 0));
        graph.allocate_energy(&id1, 25.0);
        graph.allocate_energy(&id2, 35.0);
        let flow = graph.energy_flow();
        assert_eq!(flow.len(), 2);
        assert!((flow[&id1] - 25.0).abs() < 1e-9);
        assert!((flow[&id2] - 35.0).abs() < 1e-9);
    }

    #[test]
    #[test]
    fn test_graph_frontier() {
        let mut graph = IntentionGraph::new(100.0);
        let id = graph.register(Intention::new("test", IntentionOrigin::System, 0.5, 0));
        let int = graph.intentions.get_mut(&id).unwrap();
        int.require("analyze");
        int.allocate(10.0);
        let _ = int.ready();
        assert_eq!(graph.frontier().len(), 1);
    }

    #[test]
    #[test]
    fn test_graph_top_intentions() {
        let mut graph = IntentionGraph::new(100.0);
        graph.register(Intention::new("low", IntentionOrigin::System, 0.2, 0));
        graph.register(Intention::new("high", IntentionOrigin::System, 0.9, 0));
        graph.register(Intention::new("mid", IntentionOrigin::System, 0.5, 0));
        let top2 = graph.top_intentions(2);
        assert_eq!(top2.len(), 2);
        assert_eq!(top2[0].goal, "high");
        assert_eq!(top2[1].goal, "mid");
    }

    #[test]
    #[test]
    fn test_graph_bottlenecks() {
        let mut graph = IntentionGraph::new(100.0);
        let a_id = graph.register(Intention::new("A", IntentionOrigin::System, 0.5, 0));
        let b_id = graph.register(Intention::new("B", IntentionOrigin::System, 0.5, 0));
        graph.depends_on(&b_id, &a_id);
        assert_eq!(graph.bottlenecks().len(), 1);
        assert_eq!(graph.bottlenecks()[0].goal, "A");
        let a = graph.intentions.get_mut(&a_id).unwrap();
        a.require("verify");
        a.allocate(10.0);
        let _ = a.ready();
        a.execute();
        a.complete();
        assert!(graph.bottlenecks().is_empty());
    }
}

#[cfg(test)]
mod tests2 {
    use super::*;

    #[test]
    #[test]
    fn test_graph_execute_ready() {
        let mut graph = IntentionGraph::new(100.0);
        let id = graph.register(Intention::new("test", IntentionOrigin::System, 0.5, 0));
        let int = graph.intentions.get_mut(&id).unwrap();
        int.require("analyze");
        int.allocate(10.0);
        let _ = int.ready();
        let executed = graph.execute_ready();
        assert_eq!(executed.len(), 1);
        assert!(matches!(graph.intentions[&id].status, IntentionStatus::Executing));
    }

    #[test]
    #[test]
    fn test_graph_propagate() {
        let mut graph = IntentionGraph::new(100.0);
        let a_id = graph.register(Intention::new("A", IntentionOrigin::System, 0.5, 0));
        let b_id = graph.register(Intention::new("B", IntentionOrigin::System, 0.5, 0));
        graph.depends_on(&b_id, &a_id);
        {
            let a = graph.intentions.get_mut(&a_id).unwrap();
            a.require("verify");
            a.allocate(10.0);
            let _ = a.ready();
        }
        {
            let b = graph.intentions.get_mut(&b_id).unwrap();
            b.require("verify");
            b.allocate(10.0);
        }
        graph.execute_ready();
        let _ = graph.intentions.get_mut(&a_id).unwrap().complete();
        graph.propagate();
        assert!(matches!(graph.intentions[&b_id].status, IntentionStatus::Ready));
    }

    #[test]
    #[test]
    fn test_graph_summary() {
        let mut graph = IntentionGraph::new(100.0);
        graph.register(Intention::new("a", IntentionOrigin::System, 0.5, 0));
        let summary = graph.graph_summary();
        assert!(summary.contains("IntentionGraph("));
        assert!(summary.contains("intentions=1"));
    }

    #[test]
    #[test]
    fn test_soul_signature_default() {
        let sig = SoulSignature::default();
        assert!((sig.patience - 0.5).abs() < 1e-9);
        assert!((sig.precision - 0.5).abs() < 1e-9);
        assert!((sig.playfulness - 0.5).abs() < 1e-9);
        assert!((sig.conservation_affinity - 0.5).abs() < 1e-9);
    }

    #[test]
    #[test]
    fn test_soul_signature_clamp() {
        let sig = SoulSignature::new(-1.0, 2.0, 0.5, 1.5);
        assert!((sig.patience - 0.0).abs() < 1e-9);
        assert!((sig.precision - 1.0).abs() < 1e-9);
        assert!((sig.playfulness - 0.5).abs() < 1e-9);
        assert!((sig.conservation_affinity - 1.0).abs() < 1e-9);
    }

    #[test]
    #[test]
    fn test_agent_module_new() {
        let agent = AgentModule::new("Builder", vec!["place".to_string(), "design".to_string()], 100.0);
        assert_eq!(agent.agent_id, "Builder");
        assert_eq!(agent.capabilities.len(), 2);
        assert_eq!(agent.energy_capacity, 100.0);
        assert!((agent.energy_available() - 100.0).abs() < 1e-9);
    }

    #[test]
    #[test]
    fn test_agent_can_execute() {
        let agent = AgentModule::new("Scholar", vec!["analyze".to_string(), "compute".to_string()], 80.0);
        let mut intention = Intention::new("analyze data", IntentionOrigin::System, 0.5, 0);
        intention.require("analyze");
        intention.allocate(30.0);
        assert!(agent.can_execute(&intention));
    }

    #[test]
    #[test]
    fn test_agent_cannot_execute_missing_capability() {
        let agent = AgentModule::new("Scholar", vec!["analyze".to_string()], 80.0);
        let mut intention = Intention::new("build", IntentionOrigin::System, 0.5, 0);
        intention.require("construct");
        intention.allocate(30.0);
        assert!(!agent.can_execute(&intention));
    }

    #[test]
    #[test]
    fn test_agent_cannot_execute_exhausted() {
        let mut agent = AgentModule::new("Builder", vec!["construct".to_string()], 10.0);
        agent.energy_used = 10.0;
        let mut intention = Intention::new("build", IntentionOrigin::System, 0.5, 0);
        intention.require("construct");
        intention.allocate(5.0);
        assert!(!agent.can_execute(&intention));
    }

    #[test]
    #[test]
    fn test_agent_execute() {
        let mut agent = AgentModule::new("Scholar", vec!["analyze".to_string()], 80.0);
        let mut intention = Intention::new("analyze", IntentionOrigin::System, 0.5, 0);
        intention.require("analyze");
        intention.allocate(30.0);
        let _ = intention.ready();
        let result = agent.execute(&mut intention);
        assert!(result.success);
        assert!((result.energy_consumed - 30.0).abs() < 1e-9);
        assert!((agent.energy_used - 30.0).abs() < 1e-9);
    }

    #[test]
    #[test]
    fn test_agent_execute_failure() {
        let mut agent = AgentModule::new("Builder", vec!["construct".to_string()], 10.0);
        let mut intention = Intention::new("analyze", IntentionOrigin::System, 0.5, 0);
        intention.require("analyze");
        intention.allocate(5.0);
        let _ = intention.ready();
        let result = agent.execute(&mut intention);
        assert!(!result.success);
    }

    #[test]
    #[test]
    fn test_agent_rest() {
        let mut agent = AgentModule::new("Test", vec!["analyze".to_string()], 100.0);
        agent.energy_used = 50.0;
        agent.rest(20.0);
        assert!((agent.energy_used - 30.0).abs() < 1e-9);
        agent.rest(100.0);
        assert!((agent.energy_used - 0.0).abs() < 1e-9);
    }

    #[test]
    #[test]
    fn test_agent_learn_capability() {
        let mut agent = AgentModule::new("Test", vec![], 100.0);
        agent.learn_capability("analyze");
        assert_eq!(agent.capabilities.len(), 1);
        agent.learn_capability("analyze");
        assert_eq!(agent.capabilities.len(), 1);
    }

    #[test]
    #[test]
    fn test_agent_exhausted() {
        let mut agent = AgentModule::new("Test", vec!["analyze".to_string()], 50.0);
        assert!(!agent.is_exhausted());
        agent.energy_used = 50.0;
        assert!(agent.is_exhausted());
    }

    #[test]
    #[test]
    fn test_agent_utilization() {
        let mut agent = AgentModule::new("Test", vec![], 100.0);
        assert!((agent.utilization() - 0.0).abs() < 1e-9);
        agent.energy_used = 75.0;
        assert!((agent.utilization() - 0.75).abs() < 1e-9);
    }
}
#[cfg(test)]
mod tests3 {
    use super::*;

    #[test]
    #[test]
    fn test_runtime_new() {
        let runtime = IntentionRuntime::new(500.0);
        assert!((runtime.global_budget - 500.0).abs() < 1e-9);
        assert!(runtime.agents.is_empty());
        assert_eq!(runtime.tick, 0);
    }

    #[test]
    #[test]
    fn test_runtime_register_agent() {
        let mut runtime = IntentionRuntime::new(500.0);
        runtime.register_agent(builder_agent());
        assert_eq!(runtime.agents.len(), 1);
    }

    #[test]
    #[test]
    fn test_runtime_submit_intention() {
        let mut runtime = IntentionRuntime::new(500.0);
        runtime.submit(Intention::new("test", IntentionOrigin::System, 0.5, 0));
        assert_eq!(runtime.graph.intentions.len(), 1);
    }

    #[test]
    #[test]
    fn test_runtime_tick_with_agents() {
        let mut runtime = IntentionRuntime::new(500.0);
        runtime.register_agent(builder_agent());
        let mut intention = Intention::new("build a tower", IntentionOrigin::Human("Alice".into()), 0.9, 0);
        intention.require("design");
        intention.require("place");
        intention.require("construct");
        intention.allocate(40.0);
        let _ = intention.ready();
        runtime.submit(intention);
        let result = runtime.tick();
        assert!(result.executed.len() <= 1);
        assert!(result.energy_consumed >= 0.0);
    }

    #[test]
    #[test]
    fn test_runtime_tick_no_agents() {
        let mut runtime = IntentionRuntime::new(500.0);
        let mut intention = Intention::new("test", IntentionOrigin::System, 0.5, 0);
        intention.require("analyze");
        intention.allocate(10.0);
        let _ = intention.ready();
        runtime.submit(intention);
        let result = runtime.tick();
        assert!(result.executed.is_empty());
        assert_eq!(result.energy_consumed, 0.0);
    }

    #[test]
    #[test]
    fn test_runtime_assign() {
        let mut runtime = IntentionRuntime::new(500.0);
        runtime.register_agent(builder_agent());
        let id = runtime.submit(Intention::new("test", IntentionOrigin::System, 0.5, 0));
        assert!(runtime.assign(&id, "Builder"));
        assert!(!runtime.assign(&id, "NonExistent"));
    }

    #[test]
    #[test]
    fn test_runtime_auto_assign() {
        let mut runtime = IntentionRuntime::new(500.0);
        runtime.register_agent(scholar_agent());
        runtime.register_agent(builder_agent());
        let mut intention = Intention::new("build", IntentionOrigin::System, 0.9, 0);
        intention.require("place");
        intention.allocate(30.0);
        let _ = intention.ready();
        runtime.submit(intention);
        let assignments = runtime.auto_assign();
        assert_eq!(assignments.len(), 1);
        assert_eq!(assignments[0].1, "Builder");
    }

    #[test]
    #[test]
    fn test_runtime_status() {
        let mut runtime = IntentionRuntime::new(100.0);
        runtime.register_agent(scholar_agent());
        let mut intention = Intention::new("analyze", IntentionOrigin::System, 0.5, 0);
        intention.require("analyze");
        intention.allocate(10.0);
        let _ = intention.ready();
        runtime.submit(intention);
        runtime.tick();
        let status = runtime.status();
        assert_eq!(status.total_intentions, 1);
        assert!(status.completed <= 1);
        assert_eq!(status.agents_available, 1);
    }

    #[test]
    #[test]
    fn test_compile_build() {
        let agents = vec![builder_agent(), scout_agent(), scholar_agent()];
        let graph = IntentionCompiler::compile("build a cabin", &agents, 200.0, 0);
        assert_eq!(graph.intentions.len(), 4);
        let goals: Vec<&str> = graph.intentions.values().map(|i| i.goal.as_str()).collect();
        assert!(goals.iter().any(|g| g.contains("acquire_materials")));
        assert!(goals.iter().any(|g| g.contains("design")));
        assert!(goals.iter().any(|g| g.contains("construct")));
        assert!(goals.iter().any(|g| g.contains("verify")));
        let edge_count: usize = graph.dependencies.values().map(|v| v.len()).sum();
        assert_eq!(edge_count, 3);
    }

    #[test]
    #[test]
    fn test_compile_train() {
        let agents = vec![captain_agent(), scholar_agent()];
        let graph = IntentionCompiler::compile("train agent in combat", &agents, 200.0, 0);
        assert_eq!(graph.intentions.len(), 4);
        let goals: Vec<&str> = graph.intentions.values().map(|i| i.goal.as_str()).collect();
        assert!(goals.iter().any(|g| g.contains("enroll")));
        assert!(goals.iter().any(|g| g.contains("curriculum")));
        assert!(goals.iter().any(|g| g.contains("graduate")));
        assert!(goals.iter().any(|g| g.contains("certify")));
    }

    #[test]
    #[test]
    fn test_compile_conservation() {
        let agents = vec![scholar_agent(), scout_agent()];
        let graph = IntentionCompiler::compile("solve conservation problem", &agents, 250.0, 0);
        assert_eq!(graph.intentions.len(), 5);
        let goals: Vec<&str> = graph.intentions.values().map(|i| i.goal.as_str()).collect();
        assert!(goals.iter().any(|g| g.contains("measure_inputs")));
        assert!(goals.iter().any(|g| g.contains("measure_outputs")));
        assert!(goals.iter().any(|g| g.contains("compute_error")));
        assert!(goals.iter().any(|g| g.contains("correct_error")));
        assert!(goals.iter().any(|g| g.contains("verify_balance")));
    }

    #[test]
    #[test]
    fn test_compile_explore() {
        let agents = vec![scout_agent()];
        let graph = IntentionCompiler::compile("explore the cave", &agents, 200.0, 0);
        assert_eq!(graph.intentions.len(), 4);
        let goals: Vec<&str> = graph.intentions.values().map(|i| i.goal.as_str()).collect();
        assert!(goals.iter().any(|g| g.contains("scout")));
        assert!(goals.iter().any(|g| g.contains("map")));
        assert!(goals.iter().any(|g| g.contains("identify_resources")));
        assert!(goals.iter().any(|g| g.contains("report")));
    }

    #[test]
    #[test]
    fn test_compile_fallback() {
        let agents = vec![scholar_agent()];
        let graph = IntentionCompiler::compile("do something random", &agents, 100.0, 0);
        assert_eq!(graph.intentions.len(), 1);
    }

    #[test]
    #[test]
    fn test_builder_agent() {
        let agent = builder_agent();
        assert_eq!(agent.agent_id, "Builder");
        assert_eq!(agent.capabilities.len(), 3);
        assert!((agent.energy_capacity - 100.0).abs() < 1e-9);
    }

    #[test]
    #[test]
    fn test_scout_agent() {
        let agent = scout_agent();
        assert_eq!(agent.agent_id, "Scout");
        assert_eq!(agent.capabilities.len(), 3);
        assert!((agent.energy_capacity - 60.0).abs() < 1e-9);
    }

    #[test]
    #[test]
    fn test_scholar_agent() {
        let agent = scholar_agent();
        assert_eq!(agent.agent_id, "Scholar");
        assert_eq!(agent.capabilities.len(), 3);
        assert!((agent.energy_capacity - 80.0).abs() < 1e-9);
    }

    #[test]
    #[test]
    fn test_captain_agent() {
        let agent = captain_agent();
        assert_eq!(agent.agent_id, "Captain");
        assert_eq!(agent.capabilities.len(), 3);
        assert!((agent.energy_capacity - 50.0).abs() < 1e-9);
    }

    #[test]
    #[test]
    fn test_compile_to_execution_full_flow() {
        let mut runtime = IntentionRuntime::new(500.0);
        runtime.register_agent(builder_agent());
        runtime.register_agent(scout_agent());
        runtime.register_agent(scholar_agent());
        let agents: Vec<AgentModule> = runtime.agents.values().cloned().collect();
        let graph = IntentionCompiler::compile("build a bridge", &agents, 100.0, 0);
        for (id, intention) in graph.intentions {
            runtime.graph.intentions.insert(id, intention);
        }
        for (id, deps) in graph.dependencies {
            runtime.graph.dependencies.insert(id, deps);
        }
        for _ in 0..20 {
            let result = runtime.tick();
            if result.executed.is_empty() {
                break;
            }
        }
        let status = runtime.status();
        assert!(status.completed > 0 || status.failed > 0);
    }

    #[test]
    #[test]
    fn test_serde_intention() {
        let intention = Intention::new("test", IntentionOrigin::System, 0.5, 0);
        let json = serde_json::to_string(&intention).unwrap();
        let deserialized: Intention = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.goal, intention.goal);
        assert_eq!(deserialized.priority, intention.priority);
    }

    #[test]
    #[test]
    fn test_serde_intention_graph() {
        let mut graph = IntentionGraph::new(100.0);
        graph.register(Intention::new("a", IntentionOrigin::System, 0.5, 0));
        graph.register(Intention::new("b", IntentionOrigin::System, 0.7, 0));
        let json = serde_json::to_string(&graph).unwrap();
        let deserialized: IntentionGraph = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.intentions.len(), 2);
        assert_eq!(deserialized.conservation_pool, 100.0);
    }

    #[test]
    #[test]
    fn test_serde_agent_module() {
        let agent = builder_agent();
        let json = serde_json::to_string(&agent).unwrap();
        let deserialized: AgentModule = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.agent_id, "Builder");
        assert_eq!(deserialized.capabilities.len(), 3);
    }

    #[test]
    #[test]
    fn test_serde_execution_result() {
        let result = ExecutionResult::success(42.0);
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: ExecutionResult = serde_json::from_str(&json).unwrap();
        assert!(deserialized.success);
        assert!((deserialized.energy_consumed - 42.0).abs() < 1e-9);
    }

    #[test]
    #[test]
    fn test_serde_runtime() {
        let mut runtime = IntentionRuntime::new(500.0);
        runtime.register_agent(builder_agent());
        let mut intention = Intention::new("build", IntentionOrigin::System, 0.5, 0);
        intention.require("construct");
        intention.allocate(10.0);
        let _ = intention.ready();
        runtime.submit(intention);
        let json = serde_json::to_string(&runtime).unwrap();
        let deserialized: IntentionRuntime = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.agents.len(), 1);
        assert_eq!(deserialized.graph.intentions.len(), 1);
    }

    #[test]
    #[test]
    fn test_serde_tick_result() {
        let result = TickResult {
            executed: vec![IntentionId::new("a")],
            completed: vec![],
            failed: vec![],
            energy_consumed: 42.0,
            energy_remaining: 100.0,
        };
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: TickResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.executed.len(), 1);
        assert!((deserialized.energy_consumed - 42.0).abs() < 1e-9);
    }

    #[test]
    #[test]
    fn test_serde_runtime_status() {
        let status = RuntimeStatus {
            total_intentions: 5,
            active: 2,
            completed: 3,
            failed: 0,
            agents_available: 2,
            agents_exhausted: 0,
            energy_utilization: 0.5,
            throughput: 1.5,
        };
        let json = serde_json::to_string(&status).unwrap();
        let deserialized: RuntimeStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.total_intentions, 5);
        assert!((deserialized.throughput - 1.5).abs() < 1e-9);
    }
}
