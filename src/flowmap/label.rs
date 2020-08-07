use super::*;
use super::flow::*;

/// Provides a topological ordering on a boolean network.
struct TopologicalOrder<Ni: NodeIndex> {
    s: Vec<Ni>,
    visited: Vec<Ni>,
}

impl<Ni: 'static + NodeIndex> TopologicalOrder<Ni> {
    /// Creates a new topological ordering over the provided network.
    fn new<N: Default, E: Default, Ie: IncomingEdges<Ni, E>>(
        network: &BooleanNetwork<N, E, Ni, Ie>,
    ) -> TopologicalOrder<Ni> {
        let s = network
            .nodes()
            .filter(|ni| network.ancestors(*ni).count() == 0)
            .collect();

        TopologicalOrder {
            s,
            // We'll eventually completely fill our visited list with every node
            // on the graph, so make space now
            visited: Vec::with_capacity(network.nodes().count()),
        }
    }

    /// Returns the next node in the topological ordering, or `None` of no nodes
    /// remain.
    fn next<N: Default, E: Default, Ie: IncomingEdges<Ni, E>>(
        &mut self,
        network: &BooleanNetwork<N, E, Ni, Ie>,
    ) -> Option<Ni> {
        let n = self.s.pop();

        if let Some(n) = n {
            self.visited.push(n);

            for descendent in network.descendents(n) {
                let remaining_ancestors = network
                    .ancestors(descendent)
                    .filter(|ni| !self.visited.contains(ni));

                if remaining_ancestors.count() == 0 {
                    self.s.push(descendent);
                }
            }
        }

        n
    }
}


/// Returns the label for a single node of the network.
fn label_node<'a, 'b, Ni: 'static + NodeIndex + std::fmt::Debug, Ie: IncomingEdges<Ni, (u32, u32)>>(
    mut network: &'b mut FlowMapBooleanNetwork<'a, Ni, Ie>,
    node: Ni,
    k: u32,
) -> (u32, Vec<Ni>) {
    let p = network
        .ancestors(node)
        .map(|node| {
            network
                .node_value(node)
                .label
                .expect("ancestor to be labelled")
        })
        .max()
        .expect("node being labelled to have ancestors");

    dbg!(node);

    if p == 0 {
        // Our network of ancestors is entirely PIs, and thus after collapsing
        // all nodes with label >= p we would be left only with an edge with an
        // infinite capacity between the source and sink.
        // This would mean the maximum flow on the graph is infinite, and thus
        // the label of the node we're evaluating is p + 1.
        // This also gives us an \bar{X} which only contains the node we're
        // evaluating.
        // TODO: Also return \bar{X}.
        return (p + 1, vec![node]);
    }

    let mut source = vec![];
    let mut sink = vec![];

    let mut visited = vec![];
    let mut s = vec![node];
    while let Some(node) = s.pop() {
        let ancestors = network.ancestors(node);
        network.node_value_mut(node).flow = 0;

        for ancestor in ancestors {
            *network.edge_value_mut(From(ancestor), To(node)) = (0, 1);
            if !visited.contains(&ancestor) {
                let (label, is_pi) = {
                    let node_value = network.node_value(ancestor);

                    (node_value.label, node_value.is_pi)
                };

                if label == Some(p) {
                    // This node needs to be collapsed
                    for ancestor2 in network.ancestors(ancestor) {
                        println!("adding sink {:?}", ancestor2);
                        if !sink.contains(&ancestor2) {
                            sink.push(ancestor2);
                        }
                    }
                } else if is_pi {
                    // This node needs to be joined to the source
                    println!("adding source {:?}", ancestor);
                    source.push(ancestor);
                } else {
                    // TODO: Handle infinite capacity better
                    *network.edge_value_mut(From(ancestor), To(node)) = (0, 1000);
                }

                visited.push(ancestor);
                s.push(ancestor);
            }
        }
    }

    let mut flow = Flow::new(&mut network, node, &source, &sink);
    let mut max_flow = 0;
    while flow.step() {
        max_flow += 1;
    }

    dbg!(max_flow);

    if max_flow <= k {
        let mut visited2 = visited.clone();
        visited2.push(node);
        (p, flow.cut(&visited2))
    } else {
        (p + 1, vec![node])
    }
}

/// Perform the FlowMap labelling pass on the entire network.
fn label_network<'a, 'b, Ni: 'static + NodeIndex + std::fmt::Debug, Ie: IncomingEdges<Ni, (u32, u32)>>(
    mut network: &'b mut FlowMapBooleanNetwork<'a, Ni, Ie>,
    k: u32,
) {
    let mut topo = TopologicalOrder::new(&network);

    while let Some(ni) = topo.next(&network) {
        let node_value = network.node_value(ni);

        if node_value.is_pi {
            continue;
        }

        let (label, x_bar) = label_node(&mut network, ni, k);
        network.node_value_mut(ni).label = Some(label);
        network.node_value_mut(ni).x_bar = x_bar;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn topological_order() {
        // Graph has a unique topological order
        let mut network = BooleanNetwork::<(), (), usize, Bounded2<_, _>>::new(7);
        network.add_edge(From(0), To(1));
        network.add_edge(From(0), To(2));
        network.add_edge(From(1), To(2));
        network.add_edge(From(1), To(3));
        network.add_edge(From(2), To(3));
        network.add_edge(From(3), To(4));
        network.add_edge(From(3), To(5));
        network.add_edge(From(3), To(6));
        network.add_edge(From(4), To(5));
        network.add_edge(From(5), To(6));
        network.add_edge(From(6), To(7));

        let mut topo = TopologicalOrder::new(&network);
        assert_eq!(topo.next(&network), Some(0));
        assert_eq!(topo.next(&network), Some(1));
        assert_eq!(topo.next(&network), Some(2));
        assert_eq!(topo.next(&network), Some(3));
        assert_eq!(topo.next(&network), Some(4));
        assert_eq!(topo.next(&network), Some(5));
        assert_eq!(topo.next(&network), Some(6));
        assert_eq!(topo.next(&network), Some(7));
        assert_eq!(topo.next(&network), None);
    }

    #[test]
    fn label() {
        // Fig. 5(a) from FlowMap paper, numbered top-to-bottom, left-to-right.
        let mut network = BooleanNetwork::<NodeValue<usize>, (u32, u32), usize, Bounded2<_, _>>::new(12);

        network.add_edge(From(0), To(5));
        network.add_edge(From(1), To(5));
        network.add_edge(From(1), To(6));
        network.add_edge(From(2), To(6));
        network.add_edge(From(3), To(7));
        network.add_edge(From(4), To(7));
        network.add_edge(From(5), To(8));
        network.add_edge(From(5), To(12));
        network.add_edge(From(6), To(8));
        network.add_edge(From(6), To(10));
        network.add_edge(From(7), To(9));
        network.add_edge(From(7), To(11));
        network.add_edge(From(8), To(9));
        network.add_edge(From(9), To(10));
        network.add_edge(From(10), To(11));
        network.add_edge(From(11), To(12));

        // Mark PI nodes and give label of 0
        let node_value = network.node_value_mut(0);
        node_value.label = Some(0);
        node_value.is_pi = true;
        let node_value = network.node_value_mut(1);
        node_value.label = Some(0);
        node_value.is_pi = true;
        let node_value = network.node_value_mut(2);
        node_value.label = Some(0);
        node_value.is_pi = true;
        let node_value = network.node_value_mut(3);
        node_value.label = Some(0);
        node_value.is_pi = true;
        let node_value = network.node_value_mut(4);
        node_value.label = Some(0);
        node_value.is_pi = true;

        label_network(&mut network, 3);

        // The label of PI nodes should not have changed
        assert_eq!(network.node_value(0).label, Some(0));
        assert_eq!(network.node_value(1).label, Some(0));
        assert_eq!(network.node_value(2).label, Some(0));
        assert_eq!(network.node_value(3).label, Some(0));
        assert_eq!(network.node_value(4).label, Some(0));

        // Every other node should be labelled appropriately
        assert_eq!(network.node_value(5).label, Some(1));
        assert_eq!(network.node_value(6).label, Some(1));
        assert_eq!(network.node_value(7).label, Some(1));
        assert_eq!(network.node_value(8).label, Some(1));
        assert_eq!(network.node_value(9).label, Some(2));
        assert_eq!(network.node_value(10).label, Some(2));
        assert_eq!(network.node_value(11).label, Some(2));
        assert_eq!(network.node_value(12).label, Some(2));

        // Other nodes should have the correct \bar{X} sets
        assert_eq!({ let mut v = network.node_value(5).x_bar.clone(); v.sort(); v }, vec![5]);
        assert_eq!({ let mut v = network.node_value(6).x_bar.clone(); v.sort(); v }, vec![6]);
        assert_eq!({ let mut v = network.node_value(7).x_bar.clone(); v.sort(); v }, vec![7]);
        assert_eq!({ let mut v = network.node_value(8).x_bar.clone(); v.sort(); v }, vec![5, 6, 8]);
        assert_eq!({ let mut v = network.node_value(9).x_bar.clone(); v.sort(); v }, vec![9]);
        assert_eq!({ let mut v = network.node_value(10).x_bar.clone(); v.sort(); v }, vec![9, 10]);
        assert_eq!({ let mut v = network.node_value(11).x_bar.clone(); v.sort(); v }, vec![8, 9, 10, 11]);
        assert_eq!({ let mut v = network.node_value(12).x_bar.clone(); v.sort(); v }, vec![8, 9, 10, 11, 12]);
    }
}
