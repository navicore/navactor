use petgraph::graph::Graph;

#[test]
fn test_graph() {
    let mut graph = Graph::<(), ()>::new(); // directed and unlabeled

    graph.extend_with_edges(&[(0, 1)]);

    assert_eq!(graph.node_count(), 2);
    assert_eq!(graph.edge_count(), 1);
}

#[test]
fn test_graph_labels() {
    let mut graph = Graph::new();
    let origin = graph.add_node("Denver");
    let destination_1 = graph.add_node("San Diego");
    let destination_2 = graph.add_node("New York");
    let cost_1 = graph.add_edge(origin, destination_1, 250);
    let cost_2 = graph.add_edge(origin, destination_2, 1099);

    assert_eq!(graph.node_weight(origin).unwrap(), &"Denver");
    assert_eq!(graph[destination_1], "San Diego");
    assert_eq!(graph.edge_weight(cost_1).unwrap(), &250);
    assert_eq!(graph.edge_weight(cost_2).unwrap(), &1099);
}

#[test]
fn test_unstable_indexing() {
    let mut graph = Graph::<(), ()>::new();
    let n0 = graph.add_node(());
    let n1 = graph.add_node(());

    assert_eq!(n0.index(), 0);
    assert_eq!(n1.index(), 1);

    graph.remove_node(n0);

    assert_eq!(n1.index(), 1); // watch out, not updated!

    let indexes = graph.node_indices().collect::<Vec<_>>();

    assert_ne!(indexes[0].index(), 1); // FAIL!
}
