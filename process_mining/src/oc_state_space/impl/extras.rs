// use std::collections::HashSet;
// 
// fn calculate_query_static_costs(
//     &mut self,
//     query_case: &CaseGraph,
//     query_case_stats: &CaseStats,
// ) -> f64 {
//     let mut total_cost = 0.0;
// 
//     // for all events in the case but not in model transitions add cost of 1
// 
//     let mut unhittable_edges: HashSet<usize> = HashSet::new();
// 
//     for (event_type, &partial_count) in &query_case_stats.query_event_counts {
//         let evtypename = event_type.to_string();
//         if !self.model_transitions.contains(&evtypename) {
//             total_cost += partial_count as f64;
// 
//             // get all node ids with this event type in the partial case
//             query_case
//                 .nodes
//                 .values()
//                 .filter(|node| match node {
//                     Node::EventNode(event) => event.event_type.eq(event_type),
//                     _ => false,
//                 })
//                 .for_each(|node| {
//                     if let Some(adj) = query_case.adjacency.get(&node.id()) {
//                         unhittable_edges.extend(adj.iter());
//                     }
//                 });
//         }
//     }
// 
//     total_cost + unhittable_edges.len() as f64
// }