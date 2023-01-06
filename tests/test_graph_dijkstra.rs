use petgraph::algo;
use petgraph::graph::Graph;

#[test]
fn test_dijkstra() {
    let mut graph = Graph::<(), ()>::new();

    graph.extend_with_edges(&[(0, 1), (0, 2), (0, 3), (3, 4)]);

    for start in graph.node_indices() {
        println!("--- {:?} ---", start.index());
        println!("{:?}", algo::dijkstra(&graph, start, None, |_| 1));
    }
}
