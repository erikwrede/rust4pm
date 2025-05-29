use crate::oc_conformance_checking::util::reachability_cache::ReachabilityCache;
use crate::oc_conformance_checking::util::shortest_path_cache::ShortestPathCache;
use crate::oc_petri_net::oc_petri_net::ObjectCentricPetriNet;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use crate::oc_case::case::CaseStats;
use crate::oc_state_space::ModelStateInterface;

/// Implementation of the OC state interface for OCPTs via the OCPT -> OCPN conversion.
/// This interface reuses portions of the OCPN state space implementation,
/// producing the same state space as the OCPN implementation but with additional
/// OBJ-actions to allow arbitrary object sets.
pub struct OCPTStateInterface {
    model: Arc<ObjectCentricPetriNet>,
    /// Maps OC token IDs to object node IDs in the case graph
    token_graph_id_mapping: HashMap<usize, usize>,
    /// Cache for the reachableFrom heuristic
    reachability_cache: ReachabilityCache,
    /// Cache for the remainingCost heuristic
    shortest_path_cache: ShortestPathCache,
    /// Cache containing all activities in the model
    model_transitions: HashSet<String>,
}

impl OCPTStateInterface {
    /// Initialize the Interface with a model.
    pub fn new(model: Arc<ObjectCentricPetriNet>) -> Self {
        OCPTStateInterface {
            token_graph_id_mapping: HashMap::new(),
            reachability_cache: ReachabilityCache::new(model.clone()),
            shortest_path_cache: ShortestPathCache::new(model.clone()),
            model_transitions: model.transitions.values().map(|t| t.name.clone()).collect(),
            model,
        }
    }
}

// impl ModelStateInterface for OCPTStateInterface {
//     type NodeType = OCPNStateNode;
// 
//     fn get_initial_node(&self, query_case_stats: &CaseStats) -> Self::NodeType {
//         todo!()
//     }
// 
//     fn generate_children(&mut self, node: &Self::NodeType, query_case_stats: &CaseStats, static_cost: f64) -> Vec<Self::NodeType> {
//         todo!()
//     }
// }
