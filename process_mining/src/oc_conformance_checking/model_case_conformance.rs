use crate::oc_case::case::{CaseGraph, CaseStats, Edge, EdgeType, Event, Node, Object};
use crate::oc_case::visualization::export_case_graph_image;
use crate::oc_conformance_checking::case_assignment::CaseAssignment;
use crate::oc_conformance_checking::util::reachability_cache::ReachabilityCache;
use crate::oc_conformance_checking::util::shortest_path_cache::ShortestPathCache;
use crate::oc_conformance_checking::visualization::case_visual::visualize_assignment;
use crate::oc_petri_net::marking::{Binding, Marking, OCToken};
use crate::oc_petri_net::oc_petri_net::{ObjectCentricPetriNet, Transition};
use crate::oc_state_space::{ModelStateInterface, SearchNodeAction, StateNode};
use crate::type_storage::{EventType, ObjectType, TYPE_STORAGE};
use graphviz_rust::cmd::Format;
use std::any::Any;
use std::cmp::{Ordering, PartialEq};
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::ops::{Add, Deref, Not};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct SearchNode {
    marking: Marking,
    pub partial_case: CaseGraph,
    pub min_cost: f64,
    most_recent_event_id: Option<usize>,
    action_path: Vec<Arc<SearchNodeAction>>,
    partial_case_stats: CaseStats,
    forbidden_firings: Option<HashMap<Uuid, Vec<Arc<Binding>>>>,
    depth: usize,
}

// Wrapper struct to establish min-heap ordering
struct OrderedSearchNode<T: StateNode> {
    node: T,
}

impl<T: StateNode> PartialEq for OrderedSearchNode<T> {
    fn eq(&self, other: &Self) -> bool {
        self.node.lb() == other.node.lb()
    }
}

impl<T: StateNode> Eq for OrderedSearchNode<T> {}

impl<T: StateNode> PartialOrd for OrderedSearchNode<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // Reverse ordering for min-heap
        other.node.lb().partial_cmp(&self.node.lb())
    }
}

impl<T: StateNode> Ord for OrderedSearchNode<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering for min-heap
        other.node.lb().partial_cmp(&self.node.lb()).unwrap()
    }
}

// Implement From<SearchNode> for OrderedSearchNode
impl<T: StateNode> From<T> for OrderedSearchNode<T> {
    fn from(node: T) -> OrderedSearchNode<T> {
        OrderedSearchNode { node: node }
    }
}

impl SearchNode {
    fn new(
        marking: Marking,
        partial_case: CaseGraph,
        min_cost: f64,
        most_recent_event_id: Option<usize>,
        action: Vec<Arc<SearchNodeAction>>,
    ) -> Self {
        SearchNode {
            marking,
            partial_case_stats: partial_case.get_case_stats(),
            partial_case,
            min_cost,
            most_recent_event_id,
            action_path: action,
            depth: 0,
            forbidden_firings: None,
        }
    }

    /// Manually pass the case stats, if you already know them from the changes of the previous node
    fn new_with_stats(
        marking: Marking,
        partial_case: CaseGraph,
        min_cost: f64,
        most_recent_event_id: Option<usize>,
        action: Vec<Arc<SearchNodeAction>>,
        partial_case_stats: CaseStats,
        depth: usize,
        forbidden_firings: Option<HashMap<Uuid, Vec<Arc<Binding>>>>,
    ) -> Self {
        SearchNode {
            marking,
            partial_case_stats,
            partial_case,
            min_cost,
            most_recent_event_id,
            action_path: action,
            depth,
            forbidden_firings,
        }
    }
}

/// Checker for conformance of a case to a model
pub struct ModelCaseChecker<N: 'static> {
    interface: Box<dyn ModelStateInterface<NodeType = N>>,
    shortest_case: Option<CaseGraph>,
}

impl<N: 'static + StateNode> ModelCaseChecker<N> {
    /// Initialize the checker with a model.
    pub fn new(interface: Box<dyn ModelStateInterface<NodeType = N>>) -> Self {
        ModelCaseChecker {
            interface: interface,
            shortest_case: None,
        }
    }

    /// Initializes the checker with an initial solution that can be used to calculate the initial upper bound.
    /// Use this only if the shortest case is known beforehand and if it is a valid solution for all possible other cases.
    pub fn new_with_shortest_case(
        interface: Box<dyn ModelStateInterface<NodeType = N>>,
        shortest_case: CaseGraph,
    ) -> Self {
        ModelCaseChecker {
            interface: interface,
            shortest_case: Some(shortest_case),
        }
    }

    pub fn branch_and_bound<'a>(
        &mut self,
        query_case: &'a CaseGraph,
        initial_marking: Marking,
    ) -> Option<N> {
        let mut global_upper_bound = f64::INFINITY;
        let mut best_node: Option<N> = None;

        let query_case_stats = query_case.get_case_stats();

        query_case_stats.pretty_print_stats();

        let mut any_solution_found = false;

        if let Some(shortest_case) = &self.shortest_case {
            println!("Calculating initial upper bound");
            let alignment = CaseAssignment::compute_assignment_mip(query_case, shortest_case);
            global_upper_bound = alignment.total_cost().unwrap_or(f64::INFINITY);
            println!("Initial upper bound: {}", global_upper_bound);
            
            // FIXME: This is a hack to get the initial node with the shortest case
            // best_node = Some(N::new(
            //     initial_marking.clone(),
            //     shortest_case.clone(),
            //     global_upper_bound,
            //     None,
            //     vec![Arc::new(SearchNodeAction::VOID)],
            // ));
        }

        let mut open_list: BinaryHeap<OrderedSearchNode<N>> = BinaryHeap::with_capacity(60000000);

        open_list.push(OrderedSearchNode {
            node: self.interface.get_initial_node(&query_case_stats),
        });
        
        
        let mut counter = 0;
        let mut mip_counter = 0;
        // save current time
        let mut most_recent_timestamp = std::time::Instant::now();
        let beginning_timestamp = most_recent_timestamp;
        while let Some(current_node) = open_list.pop() {
            let current_node: N = current_node.node;

            if current_node.lb() >= global_upper_bound {
                break;
            }

            counter += 1;
            // every 5 seconds print an update
            if most_recent_timestamp.elapsed().as_secs() >= 30 {
                most_recent_timestamp = std::time::Instant::now();
                println!("===================== Progress update =====================");
                println!("Nodes explored: {}", counter);
                println!(
                    "Exploration rate: {} nodes per second",
                    counter as f64 / beginning_timestamp.elapsed().as_secs_f64()
                );
                println!("Nodes aligned: {}", mip_counter);
                println!("Open list length: {}", open_list.len());
                println!("Current node min cost: {}", current_node.lb());
                println!("Global Upper Bound: {}", global_upper_bound);
                println!("---------------------");
                println!("depth: {}", current_node.depth());

                // save an intermediate result as an image in ./intermediates
                let intermediate_graph = current_node.partial_case().clone();
                let intermediate_alignment =
                    CaseAssignment::compute_assignment_mip(query_case, &intermediate_graph);
                println!(
                    "Intermediate alignment cost: {}",
                    intermediate_alignment.total_cost().unwrap_or(f64::INFINITY)
                );
                // export_case_graph_image(
                //     &intermediate_graph,
                //     format!("./intermediates_4/intermediate_{}_case.png", counter).as_str(),
                //     Format::Png,
                //     Some(0.75),
                // )
                // .unwrap();

                // export_c2_with_alignment_image(
                //     &intermediate_graph,
                //     &intermediate_alignment,
                //     format!("./intermediates/intermediate_{}_alignment.png", counter).as_str(),
                //     Format::Png,
                //     Some(2.0),
                // )
                // .unwrap();
            }

            let generate_children = true;
            if current_node.is_accepting() {
                mip_counter += 1;
                if (!any_solution_found) {
                    any_solution_found = true;
                    println!("First final marking reached")
                }
                
                let alignment = CaseAssignment::compute_assignment_mip(query_case, &current_node.partial_case());
                //println!("Alignment cost: {}", alignment.total_cost().unwrap_or(f64::INFINITY));
                /*
                if ((alignment.void_nodes.len() + alignment.void_edges.len()
                    - static_void_cost as usize)
                    <= 1)
                {
                    println!(
                        "Difference {}",
                        alignment.void_nodes.len() + alignment.void_edges.len()
                            - static_void_cost as usize
                    );
                    println!(
                        "Void nodes: {:?}",
                        alignment
                            .void_nodes
                            .values()
                            .map(|n| n.type_name())
                            .collect::<Vec<_>>()
                    );

                    // visualize alignment in file
                    export_c2_with_alignment_image(
                        &current_node.partial_case,
                        &alignment,
                        format!("./intermediates_2/alignment_{}_case.png", counter).as_str(),
                        Format::Png,
                        Some(2.0),
                    );
                }*/

                let alignment_cost = alignment.total_cost().unwrap_or(f64::INFINITY);

                if alignment_cost < global_upper_bound {
                    // log that new best bound has been found
                    println!("New best bound found: {}", alignment_cost);

                    global_upper_bound = alignment_cost;
                    best_node = Some(current_node.clone());

                    // remove all nodes that have a cost higher than the new upper bound
                    // This is currently disabled because it is very slow on high memory usage
                    //open_list.retain(|node| node.0.min_cost < global_upper_bound);
                }
            }

            if (generate_children) {
                let children = self.interface.generate_children(&current_node, &query_case_stats);
                for mut child in children {
                    if child.lb() < global_upper_bound {
                        //child.action.log(self.model.clone());
                        open_list.push(child.into());
                    } else {
                        ////println!("Pruned node before exploring with min cost: {}", child.min_cost);
                    }
                }
            }

            //open_list.sort_by(|a, b| b.min_cost.partial_cmp(&a.min_cost).unwrap());
            if (open_list.len() == 0 && best_node.is_none()) {
                println!("No solution found");
                current_node.partial_case_stats().pretty_print_stats();
            }
        }
        if (best_node.is_none()) {
            println!("No solution found after exploring {} nodes", counter);
        }
        println!("Total nodes explored: {}", counter);
        best_node
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oc_case::from_ocel::{
        json_to_case_graph, process_jsonocel_files, CaseGraphIterator,
    };
    use crate::oc_case::serialization::deserialize_case_graph;
    use crate::oc_case::visualization::export_case_graph_image;
    use crate::oc_conformance_checking::visualization::case_visual::visualize_assignment;
    use crate::oc_petri_net::initialize_ocpn_from_json;
    use graphviz_rust::cmd::Format;
    use std::path::Path;
    use std::{fs, panic};
    use crate::oc_state_space::r#impl::ocpn::{OCPNStateInterface, OCPNStateNode};
    use crate::oc_state_space::r#impl::ocpt::OCPTStateInterface;

    #[test]
    fn test_basic_alignment() {
        // Create an Object Centric Petri Net
        let mut petri_net = ObjectCentricPetriNet::new();

        // Define places
        let p1 = petri_net.add_place(
            Some("Start".to_string()),
            "ObjType".to_string(),
            true,
            false,
        );
        let p2 = petri_net.add_place(
            Some("Middle".to_string()),
            "ObjType".to_string(),
            false,
            false,
        );
        let p3 = petri_net.add_place(Some("End".to_string()), "ObjType".to_string(), false, true);

        // Define transitions
        let t1 = petri_net.add_transition("T1".to_string(), None, false);
        let t2 = petri_net.add_transition("T2".to_string(), None, false);

        // Connect places and transitions with arcs
        petri_net.add_input_arc(p1.id, t1.id, false, 1);
        petri_net.add_output_arc(t1.id, p2.id, false, 1);
        petri_net.add_input_arc(p2.id, t2.id, false, 1);
        petri_net.add_output_arc(t2.id, p3.id, false, 1);
        // Wrap the petri net in Arc to match branch_and_bound signature
        let petri_net_arc = Arc::new(petri_net);

        let initial_marking = Marking::new(petri_net_arc.clone());

        // Create a CaseGraph representing the query case
        let mut query_case = CaseGraph::new();

        // Add nodes corresponding to the events in the query case
        let event1 = Node::EventNode(Event {
            id: 1,
            event_type: "T1".into(),
        });
        let event2 = Node::EventNode(Event {
            id: 2,
            event_type: "T2".into(),
        });
        query_case.add_node(event1);
        query_case.add_node(event2);

        // Connect the events with a direct follows edge
        query_case.add_edge(Edge::new(1, 1, 2, EdgeType::DF));

        // Initialize ModelCaseChecker
        let interface = OCPTStateInterface::new(petri_net_arc.clone());

        // Initialize ModelCaseChecker
        let mut checker =
            ModelCaseChecker::new(Box::new(interface));

        // Use branch_and_bound to find alignment
        let result = checker.branch_and_bound(&query_case, initial_marking);

        // Validate if a result is found
        assert!(result.is_some(), "Failed to find a valid alignment");

        let best_node : OCPNStateNode = result.unwrap();
        let total_cost = best_node.lb();

        // Print the results for debugging
        println!("Best alignment cost: {}", total_cost);
        //best_node.partial_case.print_mappings();
    }
    #[test]
    fn process_jsonocel_files_test() {
        let source_dir = "/Users/erikwrede/dev/uni/ma-py/ocgc-py/ocgc/vars";
        process_jsonocel_files(source_dir).expect("TODO: panic message");
    }
    #[test]
    fn other() {
        let json_data = fs::read_to_string("./src/oc_petri_net/util/oc_petri_net.json").unwrap();
        let ocpn = initialize_ocpn_from_json(&json_data);

        // Wrap the petri net in Arc to match branch_and_bound signature
        let petri_net_arc = Arc::new(ocpn);

        // deserialize a query case from a json file
        // load file as string
        let case_file = fs::read_to_string(
            "./src/oc_case/test_data/variant_6eb2da5f3f3f7ea94ca51f1a72de8f47.jsonocel",
        )
        .expect("Unable to read file");

        let mut query_case = json_to_case_graph(case_file.as_str());

        // now visualize the case graph
        export_case_graph_image(&query_case, "test_case_graph.png", Format::Png, Some(2.0))
            .unwrap();
    }

    #[test]
    fn large_petri_net() {
        let result = panic::catch_unwind(|| {
            let json_data = fs::read_to_string(
                "./src/oc_conformance_checking/test_data/bpi17/oc_petri_net.json",
            )
            .unwrap();
            let ocpn = initialize_ocpn_from_json(&json_data);

            // Wrap the petri net in Arc to match branch_and_bound signature
            let petri_net_arc = Arc::new(ocpn);
            let initial_marking = Marking::new(petri_net_arc.clone());

            // deserialize a query case from a json file
            // load file as string

            let shortest_case_json = fs::read_to_string(
                "./src/oc_conformance_checking/test_data/bpi17/shortest_case_graph.json",
            )
            .expect("Unable to read file");
            let shortest_case = deserialize_case_graph(shortest_case_json.as_str());
            
            let interface = OCPNStateInterface::new(petri_net_arc.clone());
            
            // Initialize ModelCaseChecker
            let mut checker =
                ModelCaseChecker::new_with_shortest_case(Box::new(interface), shortest_case);

            let case_graph_iter = CaseGraphIterator::new(
                "/Users/erikwrede/dev/uni/ma-py/ocgc-py/ocgc/problemkinder/costcut",
            )
            .unwrap();
            let visualized_dir =
                Path::new("/Users/erikwrede/dev/uni/ma-py/ocgc-py/ocgc/varsbpi_visualized");
            for (case_graph, path) in case_graph_iter {
                let output_file_name = path.file_stem().unwrap().to_str().unwrap().to_owned();
                let output_path = visualized_dir.join(output_file_name);

                /*                export_case_graph_image(
                    &case_graph,
                    output_path.to_str().unwrap().to_owned() + "_query.png",
                    Format::Png,
                    Some(2.0),
                )
                    .unwrap();*/

                let result = checker.branch_and_bound(&case_graph, initial_marking.clone());
                if let Some(result_node) = result {
                    println!("Solution found for case {:?}", path);
                    // save the alignment result as an image in a directory next to /Users/erikwrede/dev/uni/ma-py/ocgc-py/ocgc/varsbpi
                    let alignment =
                        CaseAssignment::compute_assignment_mip(&case_graph, &result_node.partial_case);
                    let cost = alignment.total_cost().unwrap_or(f64::INFINITY);
                    println!("Cost: {}", cost);
                    visualize_assignment(
                        &result_node.partial_case,
                        &alignment,
                        output_path.to_str().unwrap().to_owned()
                            + format!("_aligned_cost_{}.png", cost).as_str(),
                        Format::Png,
                        Some(2.0),
                    )
                    .unwrap();

                    export_case_graph_image(
                        &result_node.partial_case,
                        output_path.to_str().unwrap().to_owned() + "_target.png",
                        Format::Png,
                        Some(2.0),
                    );
                } else {
                    println!("No solution found for case {:?}", path);
                }
            }
        });
        if result.is_err() {
            println!("Error: {:?}", result.err());
        }
    }

    #[test]
    fn save_case_stats() {
        let result = panic::catch_unwind(|| {
            let json_data = fs::read_to_string(
                "./src/oc_conformance_checking/test_data/bpi17/oc_petri_net.json",
            )
            .unwrap();
            let ocpn = initialize_ocpn_from_json(&json_data);

            // Wrap the petri net in Arc to match branch_and_bound signature
            let petri_net_arc = Arc::new(ocpn);
            let initial_marking = Marking::new(petri_net_arc.clone());

            // deserialize a query case from a json file
            // load file as string

            let shortest_case_json = fs::read_to_string(
                "./src/oc_conformance_checking/test_data/bpi17/shortest_case_graph.json",
            )
            .expect("Unable to read file");
            let shortest_case = deserialize_case_graph(shortest_case_json.as_str());

            // Initialize ModelCaseChecker
            let interface = OCPTStateInterface::new(petri_net_arc.clone());

            // Initialize ModelCaseChecker
            let mut checker =
                ModelCaseChecker::new_with_shortest_case(Box::new(interface), shortest_case);

            let case_graph_iter =
                CaseGraphIterator::new("/Users/erikwrede/dev/uni/ma-py/ocgc-py/test_data/varsbpi")
                    .unwrap();
            // ensure directory stats exists or create it
            let stats_dir = Path::new("/Users/erikwrede/dev/uni/ma-py/ocgc-py/ocgc/stats");
            if !stats_dir.exists() {
                fs::create_dir(stats_dir).unwrap();
            }
            for (case_graph, path) in case_graph_iter {
                let output_file_name = path.file_stem().unwrap().to_str().unwrap().to_owned();
                let output_path = stats_dir.join(output_file_name);

                let case_stats = case_graph.get_case_stats();

                let object_count = case_graph.nodes.values().filter(|n| n.is_object()).count();
                let event_count = case_graph.nodes.values().filter(|n| n.is_event()).count();

                let case_size = case_graph.nodes.len() + case_graph.edges.len();

                // save object_count, event_count, case_size, case_stats to a json file at output path

                let mut stats = HashMap::new();
                stats.insert("object_count", object_count);
                stats.insert("event_count", event_count);
                stats.insert("case_size", case_size);

                let case_stats_json = serde_json::to_string(&stats).unwrap();

                fs::write(
                    output_path.to_str().unwrap().to_owned() + "_stats.json",
                    case_stats_json,
                )
                .unwrap();
            }
        });
        if result.is_err() {
            println!("Error: {:?}", result.err());
        }
    }

    #[test]
    fn test_alignment_with_void_operations() {
        // Create a slightly more complex Petri net with optional paths (void scenarios)

        let mut petri_net = ObjectCentricPetriNet::new();

        // Define places
        let p1 = petri_net.add_place(
            Some("Start".to_string()),
            "ObjType".to_string(),
            true,
            false,
        );
        let p2 = petri_net.add_place(
            Some("Middle".to_string()),
            "ObjType".to_string(),
            false,
            false,
        );
        let p3 = petri_net.add_place(Some("End".to_string()), "ObjType".to_string(), false, true);
        let p4 = petri_net.add_place(
            Some("Optional".to_string()),
            "ObjType".to_string(),
            false,
            false,
        );

        // Define transitions
        let t1 = petri_net.add_transition("T1".to_string(), None, false);
        let t2 = petri_net.add_transition("T2".to_string(), None, false);
        let t3 = petri_net.add_transition("T3".to_string(), None, true); // Silent transition

        // Connect places and transitions with arcs
        petri_net.add_input_arc(p1.id, t1.id, false, 1);
        petri_net.add_output_arc(t1.id, p2.id, false, 1);
        petri_net.add_input_arc(p2.id, t2.id, false, 1);
        petri_net.add_output_arc(t2.id, p3.id, false, 1);
        petri_net.add_input_arc(p2.id, t3.id, true, 1);
        petri_net.add_output_arc(t3.id, p4.id, false, 1);

        // Wrap the petri net in Arc
        let petri_net_arc = Arc::new(petri_net);
        let initial_marking = Marking::new(petri_net_arc.clone());

        // Create a CaseGraph representing the query case (missing optional path)
        let mut query_case = CaseGraph::new();

        // Add nodes corresponding to the events in the query case
        let event1 = Node::EventNode(Event {
            id: 1,
            event_type: "T1".into(),
        });
        let event2 = Node::EventNode(Event {
            id: 2,
            event_type: "T2".into(),
        });
        query_case.add_node(event1);
        query_case.add_node(event2);

        // Connect the events with a direct follows edge
        query_case.add_edge(Edge::new(2, 1, 2, EdgeType::DF));

        // Initialize ModelCaseChecker
        let interface = OCPTStateInterface::new(petri_net_arc.clone());

        // Initialize ModelCaseChecker
        let mut checker =
            ModelCaseChecker::new(Box::new(interface));

        // Use branch_and_bound to find alignment
        let result = checker.branch_and_bound(&query_case, initial_marking);

        // Validate if a result is found
        assert!(result.is_some(), "Failed to find a valid alignment");

        let best_node = result.unwrap();
        let total_cost = best_node.lb();

        // Print the results for debugging
        println!(
            "Best alignment cost with optionally void edges: {}",
            total_cost
        );
        //best_node.partial_case.print_mappings();
    }
}
