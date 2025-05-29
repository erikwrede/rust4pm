use crate::oc_case::case::{CaseGraph, CaseStats, Node, Object};
use crate::oc_conformance_checking::model_case_conformance::SearchNode;
use crate::oc_petri_net::marking::{Binding, Marking};
use crate::oc_state_space::{SearchNodeAction, StateNode};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use uuid::Uuid;

use crate::oc_case::case::EdgeType;
use crate::oc_conformance_checking::util::reachability_cache::ReachabilityCache;
use crate::oc_conformance_checking::util::shortest_path_cache::ShortestPathCache;
use crate::oc_petri_net::marking::OCToken;
use crate::oc_petri_net::oc_petri_net::{ObjectCentricPetriNet, Transition};
use crate::oc_state_space::ModelStateInterface;
use crate::type_storage::{EventType, ObjectType, TYPE_STORAGE};
use std::cmp::Ordering;

#[derive(Debug, Clone)]
pub struct OCPNStateNode {
    pub marking: Marking,
    pub partial_case: CaseGraph,
    pub lb: f64,
    pub most_recent_event_id: Option<usize>,
    pub action_path: Vec<Arc<SearchNodeAction>>,
    pub partial_case_stats: CaseStats,
    pub forbidden_firings: Option<HashMap<Uuid, Vec<Arc<Binding>>>>,
    pub depth: usize,
}

impl OCPNStateNode {
    pub fn new(
        marking: Marking,
        partial_case: CaseGraph,
        lb: f64,
        most_recent_event_id: Option<usize>,
        action: Vec<Arc<SearchNodeAction>>,
    ) -> Self {
        OCPNStateNode {
            marking,
            partial_case_stats: partial_case.get_case_stats(),
            partial_case,
            lb,
            most_recent_event_id,
            action_path: action,
            depth: 0,
            forbidden_firings: None,
        }
    }

    /// Manually pass the case stats, if you already know them from the changes of the previous node
    pub fn new_with_stats(
        marking: Marking,
        partial_case: CaseGraph,
        lb: f64,
        most_recent_event_id: Option<usize>,
        action: Vec<Arc<SearchNodeAction>>,
        partial_case_stats: CaseStats,
        depth: usize,
        forbidden_firings: Option<HashMap<Uuid, Vec<Arc<Binding>>>>,
    ) -> Self {
        OCPNStateNode {
            marking,
            partial_case_stats,
            partial_case,
            lb,
            most_recent_event_id,
            action_path: action,
            depth,
            forbidden_firings,
        }
    }
}

impl StateNode for OCPNStateNode {
    fn lb(&self) -> f64 {
        self.lb
    }

    fn depth(&self) -> usize {
        self.depth
    }

    fn is_accepting(&self) -> bool {
        self.marking.is_final_has_tokens()
    }

    fn partial_case(&self) -> &CaseGraph {
        &self.partial_case
    }

    fn action_path(&self) -> &Vec<Arc<SearchNodeAction>> {
        &self.action_path
    }
    
    fn partial_case_stats(&self) -> &CaseStats {
        &self.partial_case_stats
    }
}
// 
// /// Implementation of the OC state interface for OCPTs via the OCPT -> OCPN conversion.
// /// This interface reuses portions of the OCPN state space implementation,
// /// producing the same state space as the OCPN implementation but with additional
// /// OBJ-actions to allow arbitrary object sets.
// pub struct OCPNStateInterface {
//     model: Arc<ObjectCentricPetriNet>,
//     /// Maps OC token IDs to object node IDs in the case graph
//     token_graph_id_mapping: HashMap<usize, usize>,
//     /// Cache for the reachableFrom heuristic
//     reachability_cache: ReachabilityCache,
//     /// Cache for the remainingCost heuristic
//     shortest_path_cache: ShortestPathCache,
//     /// Cache containing all activities in the model
//     model_transitions: HashSet<String>,
// }
// 
// impl OCPNStateInterface {
//     /// Initialize the Interface with a model.
//     pub fn new(model: Arc<ObjectCentricPetriNet>) -> Self {
//         OCPNStateInterface {
//             token_graph_id_mapping: HashMap::new(),
//             reachability_cache: ReachabilityCache::new(model.clone()),
//             shortest_path_cache: ShortestPathCache::new(model.clone()),
//             model_transitions: model.transitions.values().map(|t| t.name.clone()).collect(),
//             model,
//         }
//     }
// }
// 
// impl OCPNStateInterface {
//     #[inline(never)]
//     fn calculate_lb(
//         &self,
//         log_case_stats: &CaseStats,
//         partial_case_stats: &CaseStats,
//         static_cost: f64,
//         marking: &Marking,
//     ) -> f64 {
//         let mut total_cost = 0.0;
// 
//         for (event_type, &partial_count) in &partial_case_stats.query_event_counts {
//             let query_count = log_case_stats
//                 .query_event_counts
//                 .get(event_type)
//                 .unwrap_or(&0);
//             total_cost += (partial_count as f64 - *query_count as f64).max(0.0);
//         }
// 
//         for (object_type, &partial_count) in &partial_case_stats.query_object_counts {
//             let query_count = log_case_stats
//                 .query_object_counts
//                 .get(object_type)
//                 .unwrap_or(&0);
//             total_cost += (partial_count as f64 - *query_count as f64).max(0.0);
//         }
// 
//         /*
//         for (edge_type, &partial_count) in &partial_case_stats.query_edge_counts {
//             let query_count = log_case_stats
//                 .query_edge_counts
//                 .get(edge_type)
//                 .unwrap_or(&0);
//             total_cost += (partial_count as f64 - *query_count as f64).max(0.0);
//         }*/
// 
//         let mut more_epsilon_than_void = true;
// 
//         for ((edge_type, a, b), &query_count) in &log_case_stats.edge_type_counts {
//             let partial_count = partial_case_stats
//                 .edge_type_counts
//                 .get(&(*edge_type, *a, *b))
//                 .unwrap_or(&0);
// 
//             if (edge_type.eq(&EdgeType::E2O) && *partial_count == 0) {
//                 let type_storage = TYPE_STORAGE.read().unwrap();
//                 let type_name = type_storage.get_type_name(*a).unwrap();
//                 if (self.model_transitions.contains(&type_name.to_string())) {
//                     more_epsilon_than_void = false;
//                     break;
//                 } else {
//                     println!("false!!!!!")
//                 }
//             } else if (*partial_count < query_count && edge_type.eq(&EdgeType::E2O)) {
//                 more_epsilon_than_void = false;
//                 break;
//             }
//         }
// 
//         for ((edge_type, a, b), &partial_count) in &partial_case_stats.edge_type_counts {
//             let query_count = log_case_stats
//                 .edge_type_counts
//                 .get(&(*edge_type, *a, *b))
//                 .unwrap_or(&0);
//             total_cost += (partial_count as f64 - *query_count as f64).max(0.0);
//         }
// 
//         let mut more_epsilon_cost = 0.0;
//         if (more_epsilon_than_void) {
//             marking
//                 .assignments
//                 .iter()
//                 .filter(|(place_id, _)| {
//                     let place = self.model.get_place(place_id).unwrap();
//                     !place.final_place
//                 })
//                 .for_each(|(place_id, count)| {
//                     let place = self.model.get_place(place_id).unwrap();
// 
//                     let final_place = self
//                         .model
//                         .get_final_place_for_type(&place.object_type)
//                         .unwrap();
// 
//                     let shortest_path = self
//                         .shortest_path_cache
//                         .shortest_path(&place.id, &final_place.id)
//                         .unwrap()
//                         .distance;
//                     more_epsilon_cost += (shortest_path * count.len()) as f64;
//                 });
//             //println!("More_epsilon_cost: {}", more_epsilon_cost)
//         }
// 
//         let mut added_void_cost = 0.0;
//         let mut unreachable_events: HashSet<EventType> = HashSet::new();
//         // print a list of edges in log_case_stats where query_count > partial_count
//         for ((edge_type, a, b), &query_count) in &log_case_stats.edge_type_counts {
//             if (*edge_type == EdgeType::DF) {
//                 let partial_count = partial_case_stats
//                     .edge_type_counts
//                     .get(&(*edge_type, *a, *b))
//                     .unwrap_or(&0);
//                 let difference: isize = query_count as isize - *partial_count as isize;
//                 if (difference > 0) {
//                     let b_type: EventType = b.clone().into();
// 
//                     let b_transition = self
//                         .model
//                         .transitions
//                         .values()
//                         .find(|t| t.event_type == b_type);
//                     if (b_transition.is_none()) {
//                         println!(
//                             "Couldnt find transition for event type: {}",
//                             b_type.to_string()
//                         );
//                         added_void_cost += difference as f64;
//                         unreachable_events.insert(b_type);
//                         continue;
//                     }
//                     // get all input places to b and their object types
//                     let b_input_places = b_transition
//                         .unwrap()
//                         .input_arcs
//                         .iter()
//                         .map(|arc| arc.source_place_id)
//                         .collect::<Vec<_>>();
//                     let mut b_input_object_types = b_input_places
//                         .iter()
//                         .map(|place_id| {
//                             (
//                                 (place_id.clone()/*,
//                                 self.model
//                                     .get_place(place_id)
//                                     .unwrap()
//                                     .oc_object_type
//                                     .clone(),*/),
//                                 false,
//                             )
//                         })
//                         .collect::<HashMap<_, _>>();
// 
//                     // check b reachable from any place with a token using reachability cache
// 
//                     marking
//                         .assignments
//                         .iter()
//                         .filter(|(place_id, tokens)| tokens.len() > 0)
//                         .for_each(|(place_id, tokens)| {
//                             for (place_id_b) in &b_input_places {
//                                 if self.reachability_cache.is_reachable(place_id, place_id_b) {
//                                     b_input_object_types.insert(place_id_b.clone(), true);
//                                 }
//                             }
//                         });
//                     let all_reachable = b_input_object_types.values().all(|v| *v);
// 
//                     if (!all_reachable) {
//                         added_void_cost += difference as f64;
//                         unreachable_events.insert(b_type);
//                     }
//                 }
//             }
//         }
// 
//         let mut added_void_event_cost = 0.0;
//         if (!unreachable_events.is_empty()) {
//             for ((edge_type, a, b), &query_count) in &log_case_stats.edge_type_counts {
//                 if (*edge_type == EdgeType::E2O && unreachable_events.contains(&a.clone().into())) {
//                     let partial_count = partial_case_stats
//                         .edge_type_counts
//                         .get(&(*edge_type, *a, *b))
//                         .unwrap_or(&0);
//                     let difference: isize = query_count as isize - *partial_count as isize;
//                     if (difference > 0) {
//                         added_void_event_cost += difference as f64;
//                     }
//                 }
//             }
//         }
// 
//         //if (added_void_event_cost > 0.0) {
//         //    println!("Added void cost: {}", added_void_event_cost);
//         //}
// 
//         total_cost + static_cost + more_epsilon_cost + added_void_cost + added_void_event_cost
//     }
// 
//     #[inline(never)]
//     fn filter_firing_combinations(
//         &self,
//         firing_combinations: &Vec<Arc<Binding>>,
//         transition: &Transition,
//         node: &OCPNStateNode,
//     ) -> Vec<Arc<Binding>> {
//         // If the transition has variable arcs, retain all combinations
//         if transition.input_arcs.iter().any(|arc| arc.variable) {
//             return firing_combinations.iter().cloned().collect();
//         }
// 
//         // Map of object_type to its sorted list of unused tokens
//         let mut unused_objects_per_type: HashMap<ObjectType, Vec<&OCToken>> = HashMap::new();
// 
//         // Determine all object types involved in this transition
//         // Assuming a method `get_object_types_for_transition` exists
// 
//         let object_types = firing_combinations
//             .get(0)
//             .unwrap()
//             .object_binding_info
//             .keys()
//             .collect::<Vec<_>>();
// 
//         for object_type in object_types {
//             // Retrieve all tokens of this object type from all bindings
//             let all_tokens: HashSet<&OCToken> = firing_combinations
//                 .iter()
//                 .flat_map(|binding| {
//                     binding
//                         .object_binding_info
//                         .get(object_type)
//                         .map_or(Vec::new(), |binding_info| {
//                             binding_info.tokens.iter().collect()
//                         })
//                 })
//                 .collect();
// 
//             // Determine which tokens are already used (have adjacent edges)
//             let used_tokens: HashSet<&OCToken> = all_tokens
//                 .iter()
//                 .filter_map(|token| {
//                     let obj_id = self.token_graph_id_mapping.get(&token.id).unwrap();
//                     let adj_edges = node.partial_case.adjacency.get(obj_id);
// 
//                     match adj_edges {
//                         Some(edges) => {
//                             if edges.is_empty() {
//                                 None
//                             } else {
//                                 Some(*token)
//                             }
//                         }
//                         None => None,
//                     }
//                 })
//                 .collect();
// 
//             // Identify unused tokens
//             let unused_tokens: Vec<&OCToken> = all_tokens
//                 .iter()
//                 .filter_map(|token| {
//                     if !used_tokens.contains(*token) {
//                         Some(*token)
//                     } else {
//                         None
//                     }
//                 })
//                 .collect();
// 
//             // Sort the unused tokens by ID to find the lowest
//             let mut sorted_unused_tokens = unused_tokens.clone();
//             sorted_unused_tokens.sort_by_key(|token| token.id);
// 
//             unused_objects_per_type.insert(object_type.clone(), sorted_unused_tokens);
//         }
// 
//         // Now, filter the firing combinations
//         firing_combinations
//             .iter()
//             .filter(|combination| {
//                 // For each object type in the combination, apply the following:
//                 combination
//                     .object_binding_info
//                     .iter()
//                     .all(|(object_type, binding_info)| {
//                         // Assuming each binding_info has exactly one token
//                         let token = binding_info.tokens.first().unwrap();
//                         let object_id = self.token_graph_id_mapping.get(&token.id).unwrap();
// 
//                         // Check if the object is already used
//                         let is_used = !node
//                             .partial_case
//                             .adjacency
//                             .get(object_id)
//                             .map_or(true, |edges| edges.is_empty());
// 
//                         if is_used {
//                             // If the object is already used, any is allowed
//                             return true;
//                         } else {
//                             // Find the lowest unused token for this object type
//                             let unused_tokens = unused_objects_per_type.get(object_type).unwrap();
//                             // The lowest unused token is the first in the sorted list
//                             let lowest_unused = unused_tokens.first().unwrap();
//                             // Ensure that this combination includes the lowest unused token^    ^
//                             return token.id == lowest_unused.id;
//                         }
//                     })
//             })
//             .cloned()
//             .collect()
//     }
// }
// 
// impl ModelStateInterface for OCPNStateInterface {
//     type NodeType = OCPNStateNode;
//     
// 
// 
//     fn get_initial_node(
//         &mut self,
//         log_case_stats: &CaseStats,
//     ) -> SearchNode {
//         let mut new_partial_case = CaseGraph::new();
//         let mut marking = Marking::new(self.model.clone());
//         // Get the initial places from the model
//         let initial_places: HashSet<String> = self
//             .model
//             .get_initial_places()
//             .iter()
//             .map(|p| p.object_type.clone())
//             .collect();
// 
//         // Initialize marking and case from log_case_stats, only for initial places
//         for (object_type, count) in &log_case_stats.query_object_counts {
//             let typename = object_type.to_string();
//             if initial_places.contains(&typename) {
//                 for _ in 0..*count {
//                     // Find the initial place with the same object type
//                     let place = self
//                         .model
//                         .get_initial_places()
//                         .iter()
//                         .find(|p| &p.object_type == &typename)
//                         .expect("Expected matching initial place for given object type")
//                         .clone();
// 
//                     // Add token to this initial place in the marking
//                     let token_ids = marking.add_initial_token_count(&place.id, 1);
// 
//                     // Create a new object node and add it to the case graph
//                     let object_id = new_partial_case.get_new_id();
//                     let new_object = Node::ObjectNode(Object {
//                         id: object_id.clone(),
//                         object_type: object_type.clone(),
//                     });
//                     new_partial_case.add_node(new_object);
// 
//                     // Map the token to object id
//                     self.token_graph_id_mapping.insert(token_ids[0], object_id);
//                 }
//             }
//         }
// 
//         let min_cost = self.calculate_lb(
//             &log_case_stats,
//             &new_partial_case.get_case_stats(),
//             static_cost,
//             &marking,
//         );
//         println!("Starting Min Cost: {}", min_cost);
//         // Calculate the minimum cost using the initialized stats
//         let min_cost = 0;
// 
//         // Create and return the initialized SearchNode
//         SearchNode::new(
//             marking,
//             new_partial_case,
//             0 as f64,
//             None,
//             vec![Arc::new(SearchNodeAction::VOID)],
//         )
//     }
//     fn generate_children(
//         &mut self,
//         node: &Self::NodeType,
//         log_case_stats: &CaseStats,
//         static_cost: f64,
//     ) -> Vec<Self::NodeType> {
//         let mut children = Vec::new();
// 
//         // only add tokens to initial places, if the node is the initial node or follows a add token action node
//         //println!("GEtting initial places");
//         // get last entry of node.action_path
// 
//         if (node.action_path.last().unwrap().is_pre_firing()) {
//             //println!("Pre firing");
//             // a search node that is pre firing is dead, when it misses tokens in an initial place of higher lexicographical order in order to fire anything
//             // this is because we can only add tokens to the initial places in lexicographical order
// 
//             // check this by checking [INSERT HERE]
// 
//             // Order the object types in the net lexicographically and remove duplicates
//             let mut initial_place_names: Vec<String> = self
//                 .model
//                 .get_initial_places()
//                 .iter()
//                 .map(|p| p.object_type.clone())
//                 .collect::<HashSet<_>>() // Remove duplicates
//                 .into_iter()
//                 .collect();
// 
//             initial_place_names.sort(); // Sort lexicographically
//                                         //initial_place_names.reverse();
// 
//             let mut initial_places = self.model.get_initial_places().clone();
//             //initial_places = next_permutation(&initial_places);
// 
//             let counts_per_type = node.marking.get_initial_counts_per_type();
// 
//             // sort the place by amount of tokens in the marking
//             initial_places.sort_by(|a, b| {
//                 let a_count = counts_per_type.get(&a.object_type).unwrap_or(&0);
//                 let b_count = counts_per_type.get(&b.object_type).unwrap_or(&0);
// 
//                 let a_query_count = log_case_stats
//                     .query_object_counts
//                     .get(&a.oc_object_type)
//                     .unwrap_or(&0);
//                 let b_query_count = log_case_stats
//                     .query_object_counts
//                     .get(&b.oc_object_type)
//                     .unwrap_or(&0);
// 
//                 let a_diff = (*a_query_count as i64 - *a_count as i64);
//                 let b_diff = (*b_query_count as i64 - *b_count as i64);
// 
//                 if a_diff > 0 {
//                     return b_diff.cmp(&a_diff);
//                 }
// 
//                 if a_count == b_count {
//                     if a_query_count == b_query_count {
//                         return b.object_type.cmp(&a.object_type);
//                     }
//                     return b_query_count.cmp(&a_query_count);
//                 }
//                 b_count.cmp(&a_count)
//             });
// 
//             for place in initial_places {
//                 // Find the index of the current object's type in the sorted list
//                 let type_index = initial_place_names
//                     .iter()
//                     .position(|t| t == &place.object_type)
//                     .expect("Object type should exist in initial_places");
// 
//                 // Check if any higher lexicographical types have been used (i.e., have a count > 0)
//                 let higher_types_used = initial_place_names[type_index + 1..]
//                     .iter()
//                     .any(|t| counts_per_type.get(t).map_or(false, |count| *count > 0));
// 
//                 // Only allow incrementing if no higher types have been used
//                 if !higher_types_used {
//                     // Proceed to add a token to this place
//                     let mut new_marking = node.marking.clone();
//                     let token_ids = new_marking.add_initial_token_count(&place.id, 1);
// 
//                     let mut new_partial_case = node.partial_case.clone();
//                     let mut new_partial_case_stats = node.partial_case_stats.clone();
//                     let object_id = new_partial_case.get_new_id();
// 
//                     let new_action = Arc::new(SearchNodeAction::obj(
//                         place.oc_object_type.clone(),
//                         object_id,
//                     ));
//                     new_action.apply_to_case_graph(
//                         &mut new_partial_case,
//                         Some(&mut new_partial_case_stats),
//                     );
// 
//                     let lb = self.calculate_lb(
//                         &log_case_stats,
//                         &new_partial_case_stats,
//                         static_cost,
//                         &new_marking,
//                     );
//                     self.token_graph_id_mapping.insert(token_ids[0], object_id);
//                     let mut new_action_path = node.action_path.clone();
//                     new_action_path.push(new_action);
//                     children.push(OCPNStateNode::new_with_stats(
//                         new_marking,
//                         new_partial_case,
//                         lb,
//                         None,
//                         new_action_path,
//                         new_partial_case_stats,
//                         node.depth + 1,
//                         None,
//                     ));
//                 }
//                 // If higher_types_used is true, do not add tokens to this and lower types
//                 // Continue to the next place
//             }
//         }
// 
//         //println!("Getting transitions");
// 
//         //
//         let mut transition_enabled: HashMap<Uuid, bool> = HashMap::new();
// 
//         let mut firing_combinations_per_transition: HashMap<Uuid, Vec<Arc<Binding>>> =
//             HashMap::new();
// 
//         let mut forbidden_firings_per_transition: HashMap<Uuid, Vec<Arc<Binding>>> = HashMap::new();
// 
//         let mut transition_children: Vec<OCPNStateNode> = Vec::new();
//         for transition in self.model.transitions.values() {
//             //println!("Transition: {}", transition.name);
//             let firing_combinations = node.marking.get_firing_combinations(transition);
//             if (firing_combinations.len() > 0) {
//                 //println!("{}",transition.name);
//                 //println!("Firing combinations: {:?}", firing_combinations.iter().map(|c| c.to_string()).collect::<Vec<String>>());
//             }
// 
//             transition_enabled.insert(transition.id, firing_combinations.len() > 0);
//             let firing_combinations: Vec<Arc<Binding>> =
//                 firing_combinations.into_iter().map(Arc::new).collect();
// 
//             if (!transition.silent) {
//                 forbidden_firings_per_transition
//                     .insert(transition.id, firing_combinations.iter().cloned().collect());
//             }
// 
//             firing_combinations_per_transition.insert(transition.id, firing_combinations);
//         }
// 
//         if let Some(forbidden_firings_previous) = &node.forbidden_firings {
//             // if this is the case, merge forbidden_firings_per_transition with forbidden_firings_previous
//             for (transition_id, forbidden_firings_to_extend) in forbidden_firings_previous {
//                 let forbidden_firings = forbidden_firings_per_transition
//                     .entry(*transition_id)
//                     .or_insert_with(|| vec![]);
//                 forbidden_firings.extend(forbidden_firings_to_extend.iter().cloned());
//             }
//         }
// 
//         for transition in self.model.transitions.values() {
//             let firing_combinations = firing_combinations_per_transition
//                 .get(&transition.id)
//                 .unwrap();
//             if firing_combinations.is_empty() {
//                 continue;
//             }
// 
//             // Filter combinations based on symmetry breaking
//             let filtered_combinations =
//                 self.filter_firing_combinations(&firing_combinations, transition, node);
// 
//             if (filtered_combinations.is_empty()) {
//                 panic!("No filtered combinations?!");
//             }
// 
//             if (filtered_combinations.len() < firing_combinations.len()) {
//                 //println!("Filtered out {} combinations", firing_combinations.len() - filtered_combinations.len());
//             }
// 
//             // filter out forbidden firings, meaning filter out all bindings from filteredCombinations that are in searchNode.forbiddenFirings
// 
//             let mut filtered_forbidden_combinations = match &node.forbidden_firings {
//                 Some(forbidden_firings_per_t) => {
//                     let forbidden_firings = forbidden_firings_per_t.get(&transition.id);
//                     if (forbidden_firings.is_none()) {
//                         filtered_combinations.clone()
//                     } else {
//                         let forbidden_firings = forbidden_firings.unwrap();
//                         filtered_combinations
//                             .iter()
//                             .filter_map(|combination| {
//                                 if (forbidden_firings.contains(combination)) {
//                                     return None;
//                                 }
//                                 return Some(combination);
//                             })
//                             .cloned()
//                             .collect()
//                     }
//                 }
//                 None => filtered_combinations.clone(),
//             };
//             let filtered_out_len =
//                 filtered_combinations.len() - filtered_forbidden_combinations.len();
//             if (filtered_out_len > 0) {
//                 //println!("Filtered out {} forbidden combinations", filtered_out_len);
//             }
// 
//             if (transition.silent) {
//                 filtered_forbidden_combinations.sort_by(|a, b| compare_bindings(a, b));
//             }
// 
//             filtered_forbidden_combinations
//                 .iter()
//                 .enumerate()
//                 .for_each(|(index, combination)| {
//                     let mut new_marking = node.marking.clone();
//                     let mut new_partial_case = node.partial_case.clone();
//                     let mut new_partial_case_stats = node.partial_case_stats.clone();
// 
//                     let mut most_recent_event_id = node.most_recent_event_id.clone();
// 
//                     new_marking.fire_transition(transition, combination);
// 
//                     let new_action = if (!transition.silent) {
//                         let new_action = Arc::new(SearchNodeAction::ev(
//                             transition.event_type,
//                             // flatten objectBinding_info into vec<object_type, object_id>
//                             combination
//                                 .object_binding_info
//                                 .iter()
//                                 .flat_map(|(object_type, binding_info)| {
//                                     binding_info.tokens.iter().map(|token| {
//                                         (
//                                             object_type.clone(),
//                                             self.token_graph_id_mapping
//                                                 .get(&token.id)
//                                                 .unwrap()
//                                                 .clone(),
//                                         )
//                                     })
//                                 })
//                                 .collect(),
//                         ));
// 
//                         new_action.apply_to_case_graph(
//                             &mut new_partial_case,
//                             Some(&mut new_partial_case_stats),
//                         );
//                         Some(new_action)
//                     } else {
//                         None
//                     };
// 
//                     let new_cost = self.calculate_lb(
//                         &log_case_stats,
//                         &new_partial_case_stats,
//                         static_cost,
//                         &new_marking,
//                     );
// 
//                     // This is a sanity check and should not happen during normal operation.
//                     if (new_cost < node.lb) {
//                         println!("!!!!! COST DECREASED by {}", node.lb - new_cost);
//                         for ((edge_type, a, b), &partial_count) in
//                             &node.partial_case_stats.edge_type_counts
//                         {
//                             if edge_type.ne(&EdgeType::E2O) {
//                                 continue;
//                             }
//                             let query_count = log_case_stats
//                                 .edge_type_counts
//                                 .get(&(*edge_type, *a, *b))
//                                 .unwrap_or(&0);
//                             let type_storage = TYPE_STORAGE.read().unwrap();
//                             println!(
//                                 "Edge: ({:?},{},{}) Difference: {}",
//                                 edge_type,
//                                 type_storage.get_type_name(*a).unwrap(),
//                                 type_storage.get_type_name(*b).unwrap(),
//                                 (partial_count as f64 - *query_count as f64)
//                             );
//                         }
//                         println!("-----------------");
//                         println!("After firing transition: {}", transition.name);
//                         for ((edge_type, a, b), &partial_count) in
//                             &new_partial_case_stats.edge_type_counts
//                         {
//                             if edge_type.ne(&EdgeType::E2O) {
//                                 continue;
//                             }
//                             let query_count = log_case_stats
//                                 .edge_type_counts
//                                 .get(&(*edge_type, *a, *b))
//                                 .unwrap_or(&0);
//                             let type_storage = TYPE_STORAGE.read().unwrap();
//                             println!(
//                                 "Edge: ({:?},{},{}) Difference: {}",
//                                 edge_type,
//                                 type_storage.get_type_name(*a).unwrap(),
//                                 type_storage.get_type_name(*b).unwrap(),
//                                 (partial_count as f64 - *query_count as f64)
//                             );
//                         }
//                         println!("-----------------");
//                         log_case_stats.pretty_print_stats()
//                     }
// 
//                     let forbidden_firings = if (transition.silent) {
//                         let mut base = forbidden_firings_per_transition.clone();
//                         // add all combinations from index 0 to index to forbidden firings
//                         // select them from filtered_forbidden_combinations
//                         // -> symmetry-breaking
//                         let forbidden_firings = filtered_forbidden_combinations[..index].to_vec();
//                         if (!forbidden_firings.is_empty()) {
//                             base.entry(transition.id)
//                                 .or_insert_with(|| vec![])
//                                 .extend(forbidden_firings);
//                         }
//                         // now, also forbid all combinations from all silent transitions with a higher lexicographical order
//                         // which are also firable in this node
//                         // first, sort all silent transitions lexicograhically by name
// 
//                         let mut sorted_silent_transitions: Vec<&Transition> = self
//                             .model
//                             .transitions
//                             .values()
//                             .filter(|t| t.silent)
//                             .collect();
//                         sorted_silent_transitions.sort_by(|a, b| a.id.cmp(&b.id));
// 
//                         // now, iterate over all silent transitions with a higher lexicographical order
//                         // and add all firable combinations to the forbidden firings
//                         for silent_transition in sorted_silent_transitions {
//                             if (silent_transition.id <= transition.id) {
//                                 continue;
//                             }
//                             let firable_combinations = firing_combinations_per_transition
//                                 .get(&silent_transition.id)
//                                 .unwrap();
//                             // todo consider filtering out forbidden firings
//                             base.entry(silent_transition.id)
//                                 .or_insert_with(|| vec![])
//                                 .extend(firable_combinations.iter().cloned());
//                         }
// 
//                         Some(base)
//                     } else {
//                         None
//                     };
// 
//                     let mut new_action_path = node.action_path.clone();
//                     new_action_path.push(new_action.unwrap_or_else(|| {
//                         Arc::new(SearchNodeAction::ev(std::usize::MAX.into(), vec![]))
//                     }));
//                     transition_children.push(OCPNStateNode::new_with_stats(
//                         new_marking,
//                         new_partial_case,
//                         new_cost,
//                         most_recent_event_id,
//                         new_action_path,
//                         new_partial_case_stats,
//                         node.depth + 1,
//                         forbidden_firings,
//                     ));
//                 });
//         }
//         //println!("done getting transitions");
//         // places are allowed to be dead as long as we keep adding tokens to alive them ;)
//         if (!node.action_path.last().unwrap().is_pre_firing()) {
//             if !node
//                 .marking
//                 .has_dead_places(&transition_enabled, &self.reachability_cache)
//                 .is_empty()
//             {
//                 // print a list of the names of all dead places
//                 let dead_places = node
//                     .marking
//                     .has_dead_places(&transition_enabled, &self.reachability_cache);
// 
//                 dead_places.iter().for_each(|place_id| {
//                     println!(
//                         "Dead place: {}",
//                         self.model
//                             .get_place(place_id)
//                             .unwrap()
//                             .name
//                             .clone()
//                             .unwrap_or("no_name".to_string())
//                     );
//                 });
// 
//                 return vec![];
//             }
//         }
//         //println!("done checking dead");
// 
//         // sort the transitions by the difference between case stats and query case stats
//         // transition_children.sort_by(|a, b| {
//         //     // prioritize transitions that have a higher difference between the case stats and the query case stats
//         //     // all actions in this list are transitions
//         //     let transition_type = &a.action_path.last().unwrap().event_type().unwrap();
//         //
//         //
//         //     let difference_a = *log_case_stats
//         //         .query_event_counts
//         //         .get(transition_type)
//         //         .unwrap_or(&0) as i64
//         //         - *a.partial_case_stats
//         //             .query_event_counts
//         //             .get(transition_type)
//         //             .unwrap_or(&0) as i64;
//         //
//         //     let transition_type = &b.action_path.last().unwrap().event_type().unwrap();
//         //
//         //     let difference_b = *log_case_stats
//         //         .query_event_counts
//         //         .get(transition_type)
//         //         .unwrap_or(&0) as i64
//         //         - *b.partial_case_stats
//         //             .query_event_counts
//         //             .get(transition_type)
//         //             .unwrap_or(&0) as i64;
//         //
//         //     difference_a.partial_cmp(&difference_b).unwrap()
//         // });
//         //return transition_children;
//         children.append(&mut transition_children);
//         children
//     }
// }
// 
// fn compare_bindings(a: &Arc<Binding>, b: &Arc<Binding>) -> Ordering {
//     // Collect and sort object types once
//     let mut object_types: Vec<&ObjectType> = a.object_binding_info.keys().collect();
//     object_types.sort_unstable();
// 
//     for object_type in object_types {
//         let a_tokens = &a.object_binding_info[object_type].tokens;
//         let b_tokens = &b.object_binding_info[object_type].tokens;
// 
//         // Check if the number of tokens differs
//         if a_tokens.len() != b_tokens.len() {
//             return a_tokens.len().cmp(&b_tokens.len());
//         }
// 
//         // Compare sorted token IDs
//         let mut a_sorted = a_tokens.iter().map(|t| t.id).collect::<Vec<_>>();
//         let mut b_sorted = b_tokens.iter().map(|t| t.id).collect::<Vec<_>>();
//         a_sorted.sort_unstable();
//         b_sorted.sort_unstable();
// 
//         for (&a_id, &b_id) in a_sorted.iter().zip(b_sorted.iter()) {
//             match a_id.cmp(&b_id) {
//                 Ordering::Less => return Ordering::Less,
//                 Ordering::Greater => return Ordering::Greater,
//                 Ordering::Equal => continue,
//             }
//         }
//     }
// 
//     // All object types and their tokens are equal
//     Ordering::Equal
// }
