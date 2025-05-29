use std::cmp::Ordering;
use crate::oc_case::case::{CaseGraph, CaseStats, Edge, EdgeType, Event, Node, Object};
use crate::oc_petri_net::marking::Binding;
use crate::oc_petri_net::oc_petri_net::ObjectCentricPetriNet;
use crate::type_storage::{EventType, ObjectType};
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;
use uuid::Uuid;

pub mod r#impl;

pub trait StateNode: Debug + Clone  {
    /// Returns the lower bound of the node.
    fn lb(&self) -> f64;
    /// Returns the depth of the node.
    fn depth(&self) -> usize;

    /// Returns whether the node represents an accepting state on the underlying model.
    fn is_accepting(&self) -> bool;

    /// Retrieves the partial case graph.
    fn partial_case(&self) -> &CaseGraph;
    /// Retrieves the action path as a string.
    fn action_path(&self) -> &Vec<Arc<SearchNodeAction>>;
    fn partial_case_stats(&self) -> &CaseStats;
}



/// Abstraction interface for building the OC state space. Implement this for all necessary formalisms
pub trait ModelStateInterface {
    type NodeType: StateNode;

    /// Returns the initial node for the OC state space.
    /// Different from the formalism, we allow to have an initial node that contains an object set.
    /// This improves DevX for building OC state spaces without OBJ-actions, operating on a fixed set of objects.
    fn get_initial_node(&mut self, log_case_stats: &CaseStats) -> Self::NodeType;

    fn generate_children(
        &mut self,
        node: &Self::NodeType,
        log_case_stats: &CaseStats
    ) -> Vec<Self::NodeType>;
}

#[derive(Debug, Clone)]
pub enum SearchNodeAction {
    /// Represents an event that is admissible on the OC state space.
    /// The first argument is the event type, and the second argument is a vector of object node ids from the case graph.
    EV(EventType, Vec<(ObjectType, usize)>),
    /// Represents adding an object to the case graph and internal state. Second argument is the node id of the object in the case graph.
    OBJ(ObjectType, usize),
    // used for the initial node
    VOID,
}

impl SearchNodeAction {
    pub fn ev(event_type: EventType, object_set: Vec<(ObjectType, usize)>) -> Self {
        SearchNodeAction::EV(event_type, object_set)
    }

    pub fn obj(ot: ObjectType, obj_id: usize) -> Self {
        SearchNodeAction::OBJ(ot, obj_id)
    }

    pub fn event_type(&self) -> Option<EventType> {
        match self {
            SearchNodeAction::EV(event_type, _) => Some(event_type.clone()),
            _ => {
                // print an error message if the action is not an event
                eprintln!("SearchNodeAction is not an event: {:?}", self);
                None
            }
        }
    }

    pub fn object_type(&self) -> Option<ObjectType> {
        match self {
            SearchNodeAction::OBJ(object_type, _) => Some(object_type.clone()),
            _ => None,
        }
    }

    /// Checks if the action is an event firing.
    /// This is helpful for OC state spaces that dont allow new OBJ-actions after an EV-action.
    pub fn is_pre_firing(&self) -> bool {
        match self {
            SearchNodeAction::EV(_, _) => false,
            _ => true,
        }
    }

    /// Modifies the case graph based on the action
    /// Optionally, it can also modify the case stats.
    pub fn apply_to_case_graph(
        &self,
        case_graph: &mut CaseGraph,
        mut case_stats: Option<&mut CaseStats>,
    ) {
        match self {
            SearchNodeAction::EV(event_type, object_set) => {
                // We need to fetch this here as it is modified by the event insertion later, before we need it
                let mut most_recent_event_id = case_graph.most_recent_event_id.clone();

                let event_id = case_graph.get_new_id();
                let new_event = Node::EventNode(Event {
                    id: event_id,
                    event_type: event_type.clone(),
                });
                case_graph.add_node(new_event);

                object_set.iter().for_each(|(object_type, object_id)| {
                    let e20_edge_id = case_graph.get_new_id();
                    case_graph.add_edge(Edge::new(
                        e20_edge_id,
                        event_id,
                        object_id.clone(),
                        EdgeType::E2O,
                    ));

                    if let Some(ref mut case_stats) = case_stats {
                        case_stats
                            .query_edge_counts
                            .entry(EdgeType::E2O)
                            .and_modify(|e| *e += 1)
                            .or_insert(1);

                        *case_stats
                            .edge_type_counts
                            .entry((
                                EdgeType::E2O,
                                event_type.clone().into(),
                                object_type.clone().into(),
                            ))
                            .or_insert(0) += 1;
                    }
                });

                let df_edge_id = case_graph.get_new_id();
                if let Some(prev_event_id) = most_recent_event_id {
                    case_graph.add_edge(Edge::new(
                        df_edge_id,
                        prev_event_id,
                        event_id,
                        EdgeType::DF,
                    ));

                    if let Some(case_stats) = case_stats {
                        case_stats
                            .query_edge_counts
                            .entry(EdgeType::DF)
                            .and_modify(|e| *e += 1)
                            .or_insert(1);

                        let a = case_graph.get_node(prev_event_id).unwrap().oc_type_id();
                        let b = event_type;

                        *case_stats
                            .edge_type_counts
                            .entry((EdgeType::DF, a, b.clone().into()))
                            .or_insert(0) += 1;

                        case_stats
                            .query_event_counts
                            .entry(*event_type)
                            .and_modify(|e| *e += 1)
                            .or_insert(1);
                    }
                }
            }
            SearchNodeAction::OBJ(object_type, obj_id) => {
                let new_object = Node::ObjectNode(Object {
                    id: *obj_id,
                    object_type: object_type.clone(),
                });
                case_graph.add_node(new_object);

                if let Some(case_stats) = case_stats {
                    case_stats
                        .query_object_counts
                        .entry(*object_type)
                        .and_modify(|e| *e += 1)
                        .or_insert(1);
                }
            }
            SearchNodeAction::VOID => {}
        }
    }
}
