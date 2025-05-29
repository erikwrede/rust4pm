use std::collections::HashMap;
use std::sync::Arc;

// #[derive(Debug, Clone)]
// pub struct OCPNStateNode {
//     marking: Marking,
//     pub partial_case: CaseGraph,
//     pub lb: f64,
//     most_recent_event_id: Option<usize>,
//     action_path: Vec<Arc<SearchNodeAction>>,
//     partial_case_stats: CaseStats,
//     forbidden_firings: Option<HashMap<Uuid, Vec<Arc<Binding>>>>,
//     depth: usize,
// }
// 
// impl StateNode for OCPNStateNode {
//     fn lb(&self) -> f64 {
//         self.lb
//     }
// 
//     fn depth(&self) -> usize {
//         self.depth
//     }
// 
//     fn is_accepting(&self) -> bool {
//         self.partial_case.is_accepting()
//     }
// 
//     fn partial_case(&self) -> &CaseGraph {
//         &self.partial_case
//     }
// 
//     fn action_path(&self) -> String {
//         self.action_path.iter().map(|a| a.to_string()).collect::<Vec<_>>().join(" -> ")
//     }
// }