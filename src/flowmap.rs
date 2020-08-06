use crate::boolean_network::*;

/// Provides a topological ordering on a boolean network.
struct TopologicalOrder<Ni: NodeIndex> {
    s: Vec<Ni>,
    visited: Vec<Ni>,
}

impl<Ni: 'static + NodeIndex> TopologicalOrder<Ni> {
    /// Creates a new topological ordering over the provided network.
    fn new<N: Default, E: IncomingEdges<Ni>>(
        network: &BooleanNetwork<N, Ni, E>,
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
    fn next<N: Default, E: IncomingEdges<Ni>>(
        &mut self,
        network: &BooleanNetwork<N, Ni, E>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn topological_order() {
        // Graph has a unique topological order
        let mut network = BooleanNetwork::<(), usize, Bounded2<_>>::new(7);
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
}
