//! A boolean network.

use std::hash::Hash;
use std::iter;

/// Wrapper around a node index for which an edge is "from", i.e., the edge
/// points away from the node.
#[derive(Eq, PartialEq, Copy, Clone, Hash)]
#[repr(transparent)]
pub struct From<Ni: NodeIndex>(pub Ni);

impl<Ni: NodeIndex> NodeIndex for From<Ni> {
    fn from_node_index(ni: usize) -> Self {
        From(Ni::from_node_index(ni))
    }

    fn node_index(&self) -> usize {
        self.0.node_index()
    }
}

/// Wrapper around a node index for which an edge is "to", i.e., the edge points
/// to the node.
#[derive(Eq, PartialEq, Copy, Clone, Hash)]
#[repr(transparent)]
pub struct To<Ni: NodeIndex>(pub Ni);

impl<Ni: NodeIndex> NodeIndex for To<Ni> {
    fn from_node_index(ni: usize) -> Self {
        To(Ni::from_node_index(ni))
    }

    fn node_index(&self) -> usize {
        self.0.node_index()
    }
}

/// Internal node representation.
#[derive(Default)]
pub struct Node<Ni> {
    ancestors: Vec<Ni>,
    descendents: Vec<Ni>,
}

/// A boolean network.
pub struct BooleanNetwork<N: Default, E: Default, Ni: NodeIndex> {
    nodes: Vec<Node<Ni>>,
    node_values: Vec<N>,
    edge_values: Vec<Vec<E>>,
    max_node_index: usize,
}

impl<N: Default, E: Default, Ni: NodeIndex> BooleanNetwork<N, E, Ni> {
    /// Creates a new boolean network with the provided maximum index.
    pub fn new(max_index: Ni) -> BooleanNetwork<N, E, Ni> {
        let max_node_index = max_index.node_index();
        let num_nodes = max_node_index + 1;

        let nodes = iter::repeat(())
            .map(|_| Node {
                ancestors: vec![],
                descendents: vec![],
            })
            .take(num_nodes)
            .collect();

        let node_values = iter::repeat(())
            .map(|_| N::default())
            .take(num_nodes)
            .collect();
        let edge_values = iter::repeat(()).map(|_| vec![]).take(num_nodes).collect();

        BooleanNetwork {
            nodes,
            node_values,
            edge_values,
            max_node_index,
        }
    }

    /// Returns the direct ancestors of the provided node.
    pub fn ancestors(&self, of: Ni) -> &[Ni] {
        assert!(
            of.node_index() <= self.max_node_index,
            "node index out of bounds: the maximum node index is {} but the node index is {}",
            self.max_node_index,
            of.node_index()
        );

        &self.nodes[of.node_index()].ancestors
    }

    /// Returns the direct descendents of the provided node.
    pub fn descendents(&self, of: Ni) -> &[Ni] {
        assert!(
            of.node_index() <= self.max_node_index,
            "node index out of bounds: the maximum node index is {} but the node index is {}",
            self.max_node_index,
            of.node_index()
        );

        &self.nodes[of.node_index()].descendents
    }

    /// Returns a reference to the provided node's value.
    pub fn node_value(&self, of: Ni) -> &N {
        assert!(
            of.node_index() <= self.max_node_index,
            "node index out of bounds: the maximum node index is {} but the node index is {}",
            self.max_node_index,
            of.node_index()
        );

        &self.node_values[of.node_index()]
    }

    /// Returns a mutable reference to the provided node's value.
    pub fn node_value_mut(&mut self, of: Ni) -> &mut N {
        assert!(
            of.node_index() <= self.max_node_index,
            "node index out of bounds: the maximum node index is {} but the node index is {}",
            self.max_node_index,
            of.node_index()
        );

        &mut self.node_values[of.node_index()]
    }

    /// Returns the two indices required to access the value for the specified
    /// edge.
    fn edge_value_index(&self, from: From<Ni>, to: To<Ni>) -> (usize, usize) {
        assert!(
            from.node_index() <= self.max_node_index,
            "node index out of bounds: the maximum node index is {} but the node index is {}",
            self.max_node_index,
            from.node_index()
        );
        assert!(
            to.node_index() <= self.max_node_index,
            "node index out of bounds: the maximum node index is {} but the node index is {}",
            self.max_node_index,
            to.node_index()
        );

        let i = to.node_index();
        let j = self.nodes[to.node_index()]
            .ancestors
            .iter()
            .position(|ni| *ni == from.0)
            .unwrap();

        (i, j)
    }

    /// Returns a reference to the provided edge's value.
    pub fn edge_value(&self, from: From<Ni>, to: To<Ni>) -> &E {
        let (i, j) = self.edge_value_index(from, to);
        &self.edge_values[i][j]
    }

    /// Returns a mutable reference to the provided edge's value.
    pub fn edge_value_mut(&mut self, from: From<Ni>, to: To<Ni>) -> &mut E {
        let (i, j) = self.edge_value_index(from, to);
        &mut self.edge_values[i][j]
    }

    /// Adds an edge to the network graph.
    pub fn add_edge(&mut self, from: From<Ni>, to: To<Ni>) {
        assert!(
            from.node_index() <= self.max_node_index,
            "node index out of bounds: the maximum node index is {} but the node index is {}",
            self.max_node_index,
            from.node_index()
        );
        assert!(
            to.node_index() <= self.max_node_index,
            "node index out of bounds: the maximum node index is {} but the node index is {}",
            self.max_node_index,
            to.node_index()
        );

        self.nodes[to.node_index()].ancestors.push(from.0);
        self.nodes[from.node_index()].descendents.push(to.0);
        self.edge_values[to.node_index()].push(E::default());
    }

    /// Returns the number of nodes in the network.
    pub fn node_count(&self) -> usize {
        self.max_node_index + 1
    }
}

/// Trait for types which represent a node in a boolean network, and thus can be
/// used to index into the network's node/edge storage.
///
/// Network storage allocation will begin at node index zero, so implementers of
/// NodeIndex should ideally provide node index values which also begin at zero
/// to avoid wasted storage space.
pub trait NodeIndex: Eq + PartialEq + Copy + Clone + Hash {
    /// Returns an instance of the type from a bare node index.
    fn from_node_index(ni: usize) -> Self;

    /// Returns a bare node index for the type.
    fn node_index(&self) -> usize;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assert_equiv;

    impl NodeIndex for usize {
        fn from_node_index(ni: usize) -> usize {
            ni
        }

        fn node_index(&self) -> usize {
            *self
        }
    }

    fn get_network() -> BooleanNetwork<u32, u32, usize> {
        // Fig 2 from FlowMap paper, excluding source and sink with nodes
        // numbered top-to-bottom, left-to-right.
        let raw = [
            (0, vec![3, 5, 7], 0),
            (1, vec![3, 4], 0),
            (2, vec![4, 7], 0),
            (3, vec![6], 1),
            (4, vec![5, 6], 1),
            (5, vec![8, 11, 13], 2),
            (6, vec![9, 10, 11], 2),
            (7, vec![8, 9, 10, 14], 1),
            (8, vec![12, 14], 3),
            (9, vec![13], 3),
            (10, vec![15], 3),
            (11, vec![12], 3),
            (12, vec![], 4),
            (13, vec![], 4),
            (14, vec![15], 4),
            (15, vec![], 4),
        ];

        let mut network = BooleanNetwork::new(15);
        for (from, tos, node_value) in &raw {
            *network.node_value_mut(*from) = *node_value;

            for to in tos {
                network.add_edge(From(*from), To(*to));
            }
        }

        *network.edge_value_mut(From(2), To(7)) = 30;
        *network.edge_value_mut(From(10), To(15)) = 31;

        network
    }

    #[test]
    fn ancestors() {
        let network = get_network();

        assert_equiv!(network.ancestors(0), []);
        assert_equiv!(network.ancestors(1), []);
        assert_equiv!(network.ancestors(2), []);
        assert_equiv!(network.ancestors(3), [0, 1]);
        assert_equiv!(network.ancestors(4), [1, 2]);
        assert_equiv!(network.ancestors(5), [0, 4]);
        assert_equiv!(network.ancestors(6), [3, 4]);
        assert_equiv!(network.ancestors(7), [0, 2]);
        assert_equiv!(network.ancestors(8), [5, 7]);
        assert_equiv!(network.ancestors(9), [6, 7]);
        assert_equiv!(network.ancestors(10), [6, 7]);
        assert_equiv!(network.ancestors(11), [5, 6]);
        assert_equiv!(network.ancestors(12), [8, 11]);
        assert_equiv!(network.ancestors(13), [5, 9]);
        assert_equiv!(network.ancestors(14), [7, 8]);
        assert_equiv!(network.ancestors(15), [10, 14]);
    }

    #[test]
    #[should_panic(
        expected = "node index out of bounds: the maximum node index is 0 but the node index is 1"
    )]
    fn ancestors_invalid_index() {
        let network = BooleanNetwork::<(), (), usize>::new(0);

        let _ancestors = network.ancestors(1);
    }

    #[test]
    fn descendents() {
        let network = get_network();

        assert_equiv!(network.descendents(0), [3, 5, 7]);
        assert_equiv!(network.descendents(1), [3, 4]);
        assert_equiv!(network.descendents(2), [4, 7]);
        assert_equiv!(network.descendents(3), [6]);
        assert_equiv!(network.descendents(4), [5, 6]);
        assert_equiv!(network.descendents(5), [8, 11, 13]);
        assert_equiv!(network.descendents(6), [9, 10, 11]);
        assert_equiv!(network.descendents(7), [8, 9, 10, 14]);
        assert_equiv!(network.descendents(8), [12, 14]);
        assert_equiv!(network.descendents(9), [13]);
        assert_equiv!(network.descendents(10), [15]);
        assert_equiv!(network.descendents(11), [12]);
        assert_equiv!(network.descendents(12), []);
        assert_equiv!(network.descendents(13), []);
        assert_equiv!(network.descendents(14), [15]);
        assert_equiv!(network.descendents(15), []);
    }

    #[test]
    #[should_panic(
        expected = "node index out of bounds: the maximum node index is 0 but the node index is 1"
    )]
    fn descendents_invalid_index() {
        let network = BooleanNetwork::<(), (), usize>::new(0);

        let _descendents = network.descendents(1);
    }

    #[test]
    fn node_value() {
        let network = get_network();

        assert_eq!(*network.node_value(0), 0);
        assert_eq!(*network.node_value(1), 0);
        assert_eq!(*network.node_value(2), 0);
        assert_eq!(*network.node_value(3), 1);
        assert_eq!(*network.node_value(4), 1);
        assert_eq!(*network.node_value(5), 2);
        assert_eq!(*network.node_value(6), 2);
        assert_eq!(*network.node_value(7), 1);
        assert_eq!(*network.node_value(8), 3);
        assert_eq!(*network.node_value(9), 3);
        assert_eq!(*network.node_value(10), 3);
        assert_eq!(*network.node_value(11), 3);
        assert_eq!(*network.node_value(12), 4);
        assert_eq!(*network.node_value(13), 4);
        assert_eq!(*network.node_value(14), 4);
        assert_eq!(*network.node_value(15), 4);
    }

    #[test]
    #[should_panic(
        expected = "node index out of bounds: the maximum node index is 0 but the node index is 1"
    )]
    fn node_value_invalid_index() {
        let network = BooleanNetwork::<(), (), usize>::new(0);

        let _node_value = network.node_value(1);
    }

    #[test]
    fn node_value_mut() {
        let mut network = get_network();

        *network.node_value_mut(4) = 100;
        assert_eq!(*network.node_value(4), 100);
        *network.node_value_mut(4) = 200;
        assert_eq!(*network.node_value(4), 200);
    }

    #[test]
    #[should_panic(
        expected = "node index out of bounds: the maximum node index is 0 but the node index is 1"
    )]
    fn node_value_mut_invalid_index() {
        let mut network = BooleanNetwork::<(), (), usize>::new(0);

        let _node_value = network.node_value_mut(1);
    }

    #[test]
    fn edge_value() {
        let network = get_network();

        assert_eq!(*network.edge_value(From(2), To(7)), 30);
        assert_eq!(*network.edge_value(From(10), To(15)), 31);
    }

    #[test]
    #[should_panic(
        expected = "node index out of bounds: the maximum node index is 0 but the node index is 1"
    )]
    fn edge_value_invalid_index_from() {
        let network = BooleanNetwork::<(), (), usize>::new(0);

        let _edge_value = network.edge_value(From(1), To(0));
    }

    #[test]
    #[should_panic(
        expected = "node index out of bounds: the maximum node index is 0 but the node index is 1"
    )]
    fn edge_value_invalid_index_to() {
        let network = BooleanNetwork::<(), (), usize>::new(0);

        let _edge_value = network.edge_value(From(0), To(1));
    }

    #[test]
    fn edge_value_mut() {
        let mut network = get_network();

        *network.edge_value_mut(From(5), To(11)) = 50;
        assert_eq!(*network.edge_value(From(5), To(11)), 50);
        *network.edge_value_mut(From(5), To(11)) = 51;
        assert_eq!(*network.edge_value(From(5), To(11)), 51);
    }

    #[test]
    #[should_panic(
        expected = "node index out of bounds: the maximum node index is 0 but the node index is 1"
    )]
    fn edge_value_mut_invalid_index_from() {
        let mut network = BooleanNetwork::<(), (), usize>::new(0);

        let _edge_value = network.edge_value_mut(From(1), To(0));
    }

    #[test]
    #[should_panic(
        expected = "node index out of bounds: the maximum node index is 0 but the node index is 1"
    )]
    fn edge_value_mut_invalid_index_to() {
        let mut network = BooleanNetwork::<(), (), usize>::new(0);

        let _edge_value = network.edge_value_mut(From(0), To(1));
    }

    #[test]
    #[should_panic(
        expected = "node index out of bounds: the maximum node index is 0 but the node index is 1"
    )]
    fn add_edge_invalid_index_to() {
        let mut network = BooleanNetwork::<(), (), usize>::new(0);

        network.add_edge(From(0), To(1));
    }

    #[test]
    #[should_panic(
        expected = "node index out of bounds: the maximum node index is 0 but the node index is 1"
    )]
    fn add_edge_invalid_index_from() {
        let mut network = BooleanNetwork::<(), (), usize>::new(0);

        network.add_edge(From(1), To(0));
    }

    #[test]
    fn node_count() {
        assert_eq!(get_network().node_count(), 16);
    }
}
