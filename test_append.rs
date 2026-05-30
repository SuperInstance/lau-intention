
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
    fn test_intention_priority_clamped() {
        let too_high = Intention::new("test", IntentionOrigin::System, 1.5, 0);
        assert_eq!(too_high.priority, 1.0);
        let too_low = Intention::new("test", IntentionOrigin::System, -0.5, 0);
        assert_eq!(too_low.priority, 0.0);
    }

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
    fn test_intention_require_dedup() {
        let mut intention = Intention::new("test", IntentionOrigin::System, 0.5, 0);
        intention.require("analyze");
        intention.require("analyze");
        intention.require("analyze");
        assert_eq!(intention.required_capabilities.len(), 1);
    }

    #[test]
    fn test_intention_ready() {
        let mut intention = Intention::new("test", IntentionOrigin::System, 0.5, 0);
        // Not ready without requirements
        assert!(!intention.ready());
        intention.require("analyze");
        // Not ready without budget
        assert!(!intention.ready());
        intention.allocate(10.0);
        // Now ready
        assert!(intention.ready());
        assert!(matches!(intention.status, IntentionStatus::Ready));
        // Already ready — ready() returns false
        assert!(!intention.ready());
    }

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
    fn test_intention_fail() {
        let mut intention = Intention::new("test", IntentionOrigin::System, 0.5, 0);
        intention.fail();
        assert!(matches!(intention.status, IntentionStatus::Failed));
    }

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
    fn test_intention_origin_display() {
        assert_eq!(IntentionOrigin::Human("Alice".into()).to_string(), "Human(Alice)");
        assert_eq!(IntentionOrigin::Agent("Bot".into()).to_string(), "Agent(Bot)");
        assert_eq!(IntentionOrigin::System.to_string(), "System");
        assert_eq!(IntentionOrigin::Emergent("chaos".into()).to_string(), "Emergent(chaos)");
    }

    #[test]
    fn test_intention_status_display() {
        assert_eq!(IntentionStatus::Forming.to_string(), "Forming");
        assert_eq!(IntentionStatus::Ready.to_string(), "Ready");
        assert_eq!(IntentionStatus::Executing.to_string(), "Executing");
        assert_eq!(IntentionStatus::Completed.to_string(), "Completed");
        assert_eq!(IntentionStatus::Failed.to_string(), "Failed");
    }

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

    // -----------------------------------------------------------------------
    // IntentionGraph tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_graph_register() {
        let mut graph = IntentionGraph::new(100.0);
        let intention = Intention::new("test", IntentionOrigin::System, 0.5, 0);
        let id = graph.register(intention);
        assert_eq!(graph.intentions.len(), 1);
        assert!(graph.intentions.contains_key(&id));
    }

    #[test]
    fn test_graph_default() {
        let graph = IntentionGraph::default();
        assert_eq!(graph.conservation_pool, 1000.0);
        assert!(graph.intentions.is_empty());
    }

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
    fn test_graph_allocate_energy_within_budget() {
        let mut graph = IntentionGraph::new(100.0);
        let id = graph.register(Intention::new("test", IntentionOrigin::System, 0.5, 0));
        assert!(graph.allocate_energy(&id, 50.0));
        assert_eq!(graph.intentions[&id].conservation_budget, 50.0);
    }

    #[test]
    fn test_graph_allocate_energy_exceeds_budget() {
        let mut graph = IntentionGraph::new(100.0);
        let id = graph.register(Intention::new("test", IntentionOrigin::System, 0.5, 0));
        assert!(!graph.allocate_energy(&id, 200.0));
    }

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
    fn test_graph_execute_requires_deps() {
        let mut graph = IntentionGraph::new(100.0);
        let a_id = graph.register(Intention::new("A", IntentionOrigin::System, 0.5, 0));
        let b_id = graph.register(Intention::new("B", IntentionOrigin::System, 0.5, 0));
        graph.depends_on(&b_id, &a_id);

        // Setup A
        let a = graph.intentions.get_mut(&a_id).unwrap();
        a.require("verify");
        a.allocate(10.0);
        let _ = a.ready();
        a.execute();
        a.complete();

        // Setup B
        let b = graph.intentions.get_mut(&b_id).unwrap();
        b.require("verify");
        b.allocate(10.0);
        let _ = b.ready();

        // Execute B — should work since A is completed
        assert!(graph.execute(&b_id).is_ok());
        assert!(matches!(graph.intentions[&b_id].status, IntentionStatus::Executing));
    }

    #[test]
    fn test_graph_execute_fails_unsatisfied_dep() {
        let mut graph = IntentionGraph::new(100.0);
        let a_id = graph.register(Intention::new("A", IntentionOrigin::System, 0.5, 0));
        let b_id = graph.register(Intention::new("B", IntentionOrigin::System, 0.5, 0));
        graph.depends_on(&b_id, &a_id);

        // Setup B but A is still Forming
        let b = graph.intentions.get_mut(&b_id).unwrap();
        b.require("verify");
        b.allocate(10.0);
        let _ = b.ready();

        // Execute B — should fail because A is not completed
        assert!(graph.execute(&b_id).is_err());
    }

    #[test]
    fn test_graph_execute_fails_no_budget() {
        let mut graph = IntentionGraph::new(100.0);
        let id = graph.register(Intention::new("test", IntentionOrigin::System, 0.5, 0));
        // No budget allocated
        assert!(graph.execute(&id).is_err());
    }

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
    fn test_graph_frontier() {
        let mut graph = IntentionGraph::new(100.0);
        let id = graph.register(Intention::new("test", IntentionOrigin::System, 0.5, 0));
        let int = graph.intentions.get_mut(&id).unwrap();
        int.require("analyze");
        int.allocate(10.0);
        let _ = int.ready();

        let frontier = graph.frontier();
        assert_eq!(frontier.len(), 1);
        assert_eq!(frontier[0].id, id);
    }

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
    fn test_graph_bottlenecks() {
        let mut graph = IntentionGraph::new(100.0);
        let a_id = graph.register(Intention::new("A", IntentionOrigin::System, 0.5, 0));
        let b_id = graph.register(Intention::new("B", IntentionOrigin::System, 0.5, 0));
        graph.depends_on(&b_id, &a_id);

        // A is not completed and B depends on it
        let bottlenecks = graph.bottlenecks();
        assert_eq!(bottlenecks.len(), 1);
        assert_eq!(bottlenecks[0].goal, "A");

        // Complete A, no more bottlenecks
        let a = graph.intentions.get_mut(&a_id).unwrap();
        a.require("verify");
        a.allocate(10.0);
        let _ = a.ready();
        a.execute();
        a.complete();

        let bottlenecks = graph.bottlenecks();
        assert!(bottlenecks.is_empty());
    }

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
    fn test_graph_propagate_forming_to_ready() {
        let mut graph = IntentionGraph::new(100.0);
        let a_id = graph.register(Intention::new("A", IntentionOrigin::System, 0.5, 0));
        let b_id = graph.register(Intention::new("B", IntentionOrigin::System, 0.5, 0));
        graph.depends_on(&b_id, &a_id);

        // Setup A properly
        {
            let a = graph.intentions.get_mut(&a_id).unwrap();
            a.require("verify");
            a.allocate(10.0);
            let _ = a.ready();
        }

        // Setup B — Give it budget and requirements but leave it Forming
        {
            let b = graph.intentions.get_mut(&b_id).unwrap();
            b.require("verify");
            b.allocate(10.0);
            // Don't call ready() — stays Forming
        }

        // Execute and complete A
        graph.execute_ready();
        let _ = graph.intentions.get_mut(&a_id).unwrap().complete();

        // Propagate
        graph.propagate();

        // B should now be Ready
        assert!(matches!(graph.intentions[&b_id].status, IntentionStatus::Ready));
    }

    #[test]
    fn test_graph_summary() {
        let mut graph = IntentionGraph::new(100.0);
        graph.register(Intention::new("a", IntentionOrigin::System, 0.5, 0));
        let summary = graph.graph_summary();
        assert!(summary.contains("IntentionGraph("));
        assert!(summary.contains("intentions=1"));
    }

    // -----------------------------------------------------------------------
    // SoulSignature tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_soul_signature_default() {
        let sig = SoulSignature::default();
        assert!((sig.patience - 0.5).abs() < 1e-9);
        assert!((sig.precision - 0.5).abs() < 1e-9);
        assert!((sig.playfulness - 0.5).abs() < 1e-9);
        assert!((sig.conservation_affinity - 0.5).abs() < 1e-9);
    }

    #[test]
    fn test_soul_signature_clamp() {
        let sig = SoulSignature::new(-1.0, 2.0, 0.5, 1.5);
        assert!((sig.patience - 0.0).abs() < 1e-9);
        assert!((sig.precision - 1.0).abs() < 1e-9);
        assert!((sig.playfulness - 0.5).abs() < 1e-9);
        assert!((sig.conservation_affinity - 1.0).abs() < 1e-9);
    }

    // -----------------------------------------------------------------------
    // AgentModule tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_agent_module_new() {
        let agent = AgentModule::new(
            "Builder",
            vec!["place".to_string(), "design".to_string()],
            100.0,
        );
        assert_eq!(agent.agent_id, "Builder");
        assert_eq!(agent.capabilities.len(), 2);
        assert_eq!(agent.energy_capacity, 100.0);
        assert_eq!(agent.energy_used, 0.0);
        assert!((agent.energy_available() - 100.0).abs() < 1e-9);
    }

    #[test]
    fn test_agent_can_execute() {
        let agent = AgentModule::new(
            "Scholar",
            vec!["analyze".to_string(), "compute".to_string()],
            80.0,
        );
        let mut intention = Intention::new("analyze data", IntentionOrigin::System, 0.5, 0);
        intention.require("analyze");
        intention.allocate(30.0);
        assert!(agent.can_execute(&intention));
    }

    #[test]
    fn test_agent_cannot_execute_missing_capability() {
        let agent = AgentModule::new(
            "Scholar",
            vec!["analyze".to_string()],
            80.0,
        );
        let mut intention = Intention::new("build", IntentionOrigin::System, 0.5, 0);
        intention.require("construct");
        intention.allocate(30.0);
        assert!(!agent.can_execute(&intention));
    }

    #[test]
    fn test_agent_cannot_execute_exhausted() {
        let mut agent = AgentModule::new(
            "Builder",
            vec!["construct".to_string()],
            10.0,
        );
        agent.energy_used = 10.0; // Exhausted
        let mut intention = Intention::new("build", IntentionOrigin::System, 0.5, 0);
        intention.require("construct");
        intention.allocate(5.0);
        assert!(!agent.can_execute(&intention));
    }

    #[test]
    fn test_agent_execute() {
        let mut agent = AgentModule::new(
            "Scholar",
            vec!["analyze".to_string()],
            80.0,
        );
        let mut intention = Intention::new("analyze", IntentionOrigin::System, 0.5, 0);
        intention.require("analyze");
        intention.allocate(30.0);
        let _ = intention.ready();

        let result = agent.execute(&mut intention);
        assert!(result.success);
        assert!((result.energy_consumed - 30.0).abs() < 1e-9);
        assert!((agent.energy_used - 30.0).abs() < 1e-9);
        assert_eq!(agent.intention_history.len(), 1);
    }

    #[test]
    fn test_agent_execute_failure() {
        let mut agent = AgentModule::new(
            "Builder",
            vec!["construct".to_string()],
            10.0,
        );
        let mut intention = Intention::new("analyze", IntentionOrigin::System, 0.5, 0);
        intention.require("analyze"); // Agent doesn't have this
        intention.allocate(5.0);
        let _ = intention.ready();

        let result = agent.execute(&mut intention);
        assert!(!result.success);
    }

    #[test]
    fn test_agent_rest() {
        let mut agent = AgentModule::new("Test", vec!["analyze".to_string()], 100.0);
        agent.energy_used = 50.0;
        agent.rest(20.0);
        assert!((agent.energy_used - 30.0).abs() < 1e-9);
        agent.rest(100.0); // Should not go below 0
        assert!((agent.energy_used - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_agent_learn_capability() {
        let mut agent = AgentModule::new("Test", vec![], 100.0);
        agent.learn_capability("analyze");
        assert_eq!(agent.capabilities.len(), 1);
        agent.learn_capability("analyze"); // Dedup
        assert_eq!(agent.capabilities.len(), 1);
    }

    #[test]
    fn test_agent_exhausted() {
        let mut agent = AgentModule::new("Test", vec!["analyze".to_string()], 50.0);
        assert!(!agent.is_exhausted());
        agent.energy_used = 50.0;
        assert!(agent.is_exhausted());
    }

    #[test]
    fn test_agent_utilization() {
        let mut agent = AgentModule::new("Test", vec![], 100.0);
        assert!((agent.utilization() - 0.0).abs() < 1e-9);
        agent.energy_used = 75.0;
        assert!((agent.utilization() - 0.75).abs() < 1e-9);
    }

    // -----------------------------------------------------------------------
    // IntentionRuntime tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_runtime_new() {
        let runtime = IntentionRuntime::new(500.0);
        assert!((runtime.global_budget - 500.0).abs() < 1e-9);
        assert!(runtime.agents.is_empty());
        assert_eq!(runtime.tick, 0);
    }

    #[test]
    fn test_runtime_register_agent() {
        let mut runtime = IntentionRuntime::new(500.0);
        let agent = builder_agent();
        runtime.register_agent(agent);
        assert_eq!(runtime.agents.len(), 1);
    }

    #[test]
    fn test_runtime_submit_intention() {
        let mut runtime = IntentionRuntime::new(500.0);
        let intention = Intention::new("test", IntentionOrigin::System, 0.5, 0);
        let id = runtime.submit(intention);
        assert_eq!(runtime.graph.intentions.len(), 1);
    }

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
        assert_eq!(runtime.tick, 1);
    }

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
    fn test_runtime_assign() {
        let mut runtime = IntentionRuntime::new(500.0);
        runtime.register_agent(builder_agent());
        let intention = Intention::new("test", IntentionOrigin::System, 0.5, 0);
        let id = runtime.submit(intention);
        assert!(runtime.assign(&id, "Builder"));
        assert!(!runtime.assign(&id, "NonExistent"));
    }

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
        assert!(status.throughput >= 0.0);
    }

    // -----------------------------------------------------------------------
    // IntentionCompiler tests
    // -----------------------------------------------------------------------

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
    fn test_compile_fallback() {
        let agents = vec![scholar_agent()];
        let graph = IntentionCompiler::compile("do something random", &agents, 100.0, 0);
        assert_eq!(graph.intentions.len(), 1);
    }

    // -----------------------------------------------------------------------
    // Pre-built Agent Module tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_builder_agent() {
        let agent = builder_agent();
        assert_eq!(agent.agent_id, "Builder");
        assert_eq!(agent.capabilities.len(), 3);
        assert!((agent.energy_capacity - 100.0).abs() < 1e-9);
        assert!((agent.soul_signature.precision - 0.8).abs() < 1e-9);
    }

    #[test]
    fn test_scout_agent() {
        let agent = scout_agent();
        assert_eq!(agent.agent_id, "Scout");
        assert_eq!(agent.capabilities.len(), 3);
        assert!((agent.energy_capacity - 60.0).abs() < 1e-9);
        assert!((agent.soul_signature.playfulness - 0.7).abs() < 1e-9);
    }

    #[test]
    fn test_scholar_agent() {
        let agent = scholar_agent();
        assert_eq!(agent.agent_id, "Scholar");
        assert_eq!(agent.capabilities.len(), 3);
        assert!((agent.energy_capacity - 80.0).abs() < 1e-9);
        assert!((agent.soul_signature.conservation_affinity - 0.95).abs() < 1e-9);
    }

    #[test]
    fn test_captain_agent() {
        let agent = captain_agent();
        assert_eq!(agent.agent_id, "Captain");
        assert_eq!(agent.capabilities.len(), 3);
        assert!((agent.energy_capacity - 50.0).abs() < 1e-9);
        assert!((agent.soul_signature.patience - 0.9).abs() < 1e-9);
    }

    // -----------------------------------------------------------------------
    // Full integration test
    // -----------------------------------------------------------------------

    #[test]
    fn test_compile_to_execution_full_flow() {
        let mut runtime = IntentionRuntime::new(500.0);
        runtime.register_agent(builder_agent());
        runtime.register_agent(scout_agent());
        runtime.register_agent(scholar_agent());

        let agents: Vec<AgentModule> = runtime.agents.values().cloned().collect();
        let graph = IntentionCompiler::compile("build a bridge", &agents, 400.0, 0);

        // Register all compiled intentions into the runtime
        for (id, intention) in graph.intentions {
            runtime.graph.intentions.insert(id, intention);
        }
        for (id, deps) in graph.dependencies {
            runtime.graph.dependencies.insert(id, deps);
        }

        // Tick until everything is done or stuck
        for _ in 0..20 {
            let result = runtime.tick();
            if result.executed.is_empty() {
                break;
            }
        }

        let status = runtime.status();
        assert!(status.completed > 0 || status.failed > 0);
    }

    // -----------------------------------------------------------------------
    // Serde tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_serde_intention() {
        let intention = Intention::new("test", IntentionOrigin::System, 0.5, 0);
        let json = serde_json::to_string(&intention).unwrap();
        let deserialized: Intention = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.goal, intention.goal);
        assert_eq!(deserialized.priority, intention.priority);
    }

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
    fn test_serde_agent_module() {
        let agent = builder_agent();
        let json = serde_json::to_string(&agent).unwrap();
        let deserialized: AgentModule = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.agent_id, "Builder");
        assert_eq!(deserialized.capabilities.len(), 3);
    }

    #[test]
    fn test_serde_execution_result() {
        let result = ExecutionResult::success(42.0);
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: ExecutionResult = serde_json::from_str(&json).unwrap();
        assert!(deserialized.success);
        assert!((deserialized.energy_consumed - 42.0).abs() < 1e-9);
    }

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
        assert!((deserialized.energy_consumed -