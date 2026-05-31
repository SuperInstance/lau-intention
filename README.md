# lau-intention

The Intention Runtime — autograd for agent intentions.

You declare what you want (an intention), the runtime decomposes it into sub-intentions, assigns agents, and enforces energy conservation through the entire DAG. If you've ever wished your agent orchestration had the rigor of a computational graph — this is that.

## The concept in 60 seconds

An **Intention** is a typed goal with an origin, a priority, required capabilities, and a conservation budget. Intentions form a directed acyclic graph — one intention can depend on another. The runtime tracks energy flow through the graph, ensuring total spend never exceeds the pool.

Think of it like autograd: you build a graph of operations, then execute it topologically. Except here the operations are agent tasks, and the gradients are energy budgets.

## Quick start

```rust
use lau_intention::*;

// Build a runtime with an energy budget
let mut rt = IntentionRuntime::new(1000.0);

// Register intentions
let build = Intention::new("Build the feature", IntentionOrigin::Human("alice".into()), 0.8, rt.tick());
let test = Intention::new("Test the feature", IntentionOrigin::Agent("ci-bot".into()), 0.5, rt.tick());

let build_id = rt.register(build);
let test_id = rt.register(test);

// Test depends on build
rt.depends_on(&test_id, &build_id);

// Assign agents
let dev = builder_agent();
let qa = scout_agent();
rt.assign(&build_id, dev).unwrap();
rt.assign(&test_id, qa).unwrap();

// Allocate energy
rt.allocate_energy(&build_id, 400.0);
rt.allocate_energy(&test_id, 200.0);

// Execute the graph
let result = rt.tick();
assert!(result.completed.len() + result.executing.len() > 0);
assert!(rt.is_conserved()); // energy never exceeds the pool
```

## Key types

| Type | What it does |
|------|-------------|
| `Intention` | A typed goal with origin, priority, capabilities, budget |
| `IntentionRuntime` | The graph engine: register, assign, allocate, tick |
| `IntentionGraph` | DAG of intentions with dependency tracking |
| `AgentModule` | Agent descriptor: name, capabilities, conservation budget |
| `SoulSignature` | Per-agent behavioral signature |
| `IntentionCompiler` | Compiles a graph into an execution plan |

## Built-in agents

```rust
let builder = builder_agent();  // "Builder" — builds, creates, implements
let scout   = scout_agent();    // "Scout"   — searches, observes, explores
let scholar = scholar_agent();  // "Scholar" — researches, explains, documents
let captain = captain_agent();  // "Captain" — coordinates, decides, delegates
```

## Energy conservation

Every intention has a budget. The runtime tracks total allocated vs. total pool:

```rust
rt.allocate_energy(&id, 500.0); // returns false if it would exceed the pool
assert!(rt.is_conserved());      // spent <= pool + epsilon
```

This isn't bookkeeping — it's a constraint. The runtime will refuse allocations that violate conservation.

## Contributing

PRs welcome. This crate is part of the [SuperInstance](https://github.com/SuperInstance) ecosystem. Open an issue if you have questions about the intention model or want to propose new agent archetypes.
