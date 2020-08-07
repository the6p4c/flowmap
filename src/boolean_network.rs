//! A boolean network

use std::iter;
use std::marker::PhantomData;

/// Wrapper around a node index for which an edge is "from", i.e., the edge
/// points away from the node.
#[repr(transparent)]
pub struct From<Ni: NodeIndex>(pub Ni);

/// Wrapper around a node index for which an edge is "to", i.e., the edge points
/// to the node.
#[repr(transparent)]
pub struct To<Ni: NodeIndex>(pub Ni);

/// Internal node representation.
pub struct Node<'a, N: Default, E: Default, Ni: NodeIndex, Ie: IncomingEdges<Ni, E>> {
    incoming_edges: Ie,
    value: N,
    phantom1: PhantomData<&'a Ni>,
    phantom2: PhantomData<&'a E>,
}

impl<N: Default, E: Default, Ni: NodeIndex, Ie: IncomingEdges<Ni, E>> Default
    for Node<'_, N, E, Ni, Ie>
{
    fn default() -> Self {
        Node {
            incoming_edges: Ie::default(),
            value: N::default(),
            phantom1: PhantomData,
            phantom2: PhantomData,
        }
    }
}

/// A boolean network.
pub struct BooleanNetwork<'a, N: Default, E: Default, Ni: NodeIndex, Ie: IncomingEdges<Ni, E>> {
    nodes: Vec<Node<'a, N, E, Ni, Ie>>,
    max_node_index: usize,
}

impl<N: Default, E: Default, Ni: 'static + NodeIndex, Ie: IncomingEdges<Ni, E>>
    BooleanNetwork<'_, N, E, Ni, Ie>
{
    /// Creates a new boolean network with the provided maximum index.
    pub fn new(max_index: Ni) -> BooleanNetwork<'static, N, E, Ni, Ie> {
        let max_node_index = max_index.node_index();

        let num_nodes = max_node_index + 1;
        let nodes = iter::repeat(())
            .map(|_| Node::default())
            .take(num_nodes)
            .collect();

        BooleanNetwork {
            nodes,
            max_node_index,
        }
    }

    /// Returns an iterator over the valid node index values for this network.
    pub fn nodes(&self) -> Box<dyn Iterator<Item = Ni>> {
        Box::new((0..=self.max_node_index).map(Ni::from_node_index))
    }

    /// Returns an iterator over the direct ancestors of the provided node.
    pub fn ancestors(&self, of: Ni) -> Box<dyn Iterator<Item = Ni>> {
        assert!(
            of.node_index() <= self.max_node_index,
            "node index out of bounds: the maximum node index is {} but the node index is {}",
            self.max_node_index,
            of.node_index()
        );

        self.nodes[of.node_index()].incoming_edges.ancestors()
    }

    /// Returns an iterator over the direct descendents of the provided node.
    pub fn descendents(&self, of: Ni) -> Box<dyn Iterator<Item = Ni> + '_> {
        assert!(
            of.node_index() <= self.max_node_index,
            "node index out of bounds: the maximum node index is {} but the node index is {}",
            self.max_node_index,
            of.node_index()
        );

        Box::new(
            self.nodes
                .iter()
                .enumerate()
                .map(|(from, node)| (from, &node.incoming_edges))
                .filter_map(move |(from, edges)| {
                    // Is the node we've been asked to find the descendents of
                    // an ancestor of the node we're currently looking at?
                    if edges.ancestors().any(|f| f == of) {
                        // If so, the node we're currently looking at is a
                        // descendent of the node we've been asked about
                        Some(Ni::from_node_index(from))
                    } else {
                        None
                    }
                }),
        )
    }

    /// Returns a reference to the provided node's value.
    pub fn node_value(&self, of: Ni) -> &N {
        &self.nodes[of.node_index()].value
    }

    /// Returns a mutable reference to the provided node's value.
    pub fn node_value_mut(&mut self, of: Ni) -> &mut N {
        &mut self.nodes[of.node_index()].value
    }

    /// Returns a reference to the provided edge's value.
    pub fn edge_value(&self, from: From<Ni>, to: To<Ni>) -> &E {
        self.nodes[to.0.node_index()]
            .incoming_edges
            .edge_value(from)
    }

    /// Returns a mutable reference to the provided edge's value.
    pub fn edge_value_mut(&mut self, from: From<Ni>, to: To<Ni>) -> &mut E {
        self.nodes[to.0.node_index()]
            .incoming_edges
            .edge_value_mut(from)
    }

    /// Adds an edge to the network graph.
    pub fn add_edge(&mut self, from: From<Ni>, to: To<Ni>) {
        assert!(
            from.0.node_index() <= self.max_node_index,
            "node index out of bounds: the maximum node index is {} but the node index is {}",
            self.max_node_index,
            from.0.node_index()
        );
        assert!(
            to.0.node_index() <= self.max_node_index,
            "node index out of bounds: the maximum node index is {} but the node index is {}",
            self.max_node_index,
            to.0.node_index()
        );

        self.nodes[to.0.node_index()].incoming_edges.add_edge(from);
    }
}

/// Trait for types which represent a node in a boolean network, and thus can be
/// used to index into the network's node/edge storage.
///
/// Network storage allocation will begin at node index zero, so implementers of
/// NodeIndex should ideally provide node index values which also begin at zero
/// to avoid wasted storage space.
pub trait NodeIndex: PartialEq + Copy + Clone {
    /// Returns an instance of the type from a bare node index.
    fn from_node_index(ni: usize) -> Self;

    /// Returns a bare node index for the type.
    fn node_index(&self) -> usize;
}

/// Trait for types which can track the incoming edges of a node.
pub trait IncomingEdges<Ni: NodeIndex, E: Default>: Default {
    /// Returns an iterator over the direct ancestors of the node for which the
    /// incoming edges are being tracked.
    fn ancestors(&self) -> Box<dyn Iterator<Item = Ni>>;

    /// Adds an incoming edge.
    fn add_edge(&mut self, from: From<Ni>);

    /// Returns a reference to the provided edge's value.
    fn edge_value(&self, from: From<Ni>) -> &E;

    /// Returns a mutable reference to the provided edge's value.
    fn edge_value_mut(&mut self, from: From<Ni>) -> &mut E;
}

/// Incoming edges tracking for a 2-bounded boolean network.
pub struct Bounded2<Ni: NodeIndex, E: Default>([Option<(Ni, E)>; 2]);

impl<E: Default, Ni: 'static + NodeIndex> IncomingEdges<Ni, E> for Bounded2<Ni, E> {
    fn ancestors(&self) -> Box<dyn Iterator<Item = Ni>> {
        match self.0 {
            [None, None] => Box::new(iter::empty()),
            [Some((ni0, _)), None] => Box::new(iter::once(ni0)),
            [None, Some((ni0, _))] => Box::new(iter::once(ni0)),
            [Some((ni0, _)), Some((ni1, _))] => Box::new(iter::once(ni0).chain(iter::once(ni1))),
        }
    }

    fn add_edge(&mut self, from: From<Ni>) {
        match &mut self.0 {
            [ni @ None, _] => *ni = Some((from.0, E::default())),
            [_, ni @ None] => *ni = Some((from.0, E::default())),
            _ => panic!("could not add edge"),
        }
    }

    fn edge_value(&self, from: From<Ni>) -> &E {
        match &self.0 {
            [Some((ni, ev)), _] if *ni == from.0 => ev,
            [_, Some((ni, ev))] if *ni == from.0 => ev,
            _ => panic!("edge not found"),
        }
    }

    fn edge_value_mut(&mut self, from: From<Ni>) -> &mut E {
        match &mut self.0 {
            [Some((ni, ev)), _] if *ni == from.0 => ev,
            [_, Some((ni, ev))] if *ni == from.0 => ev,
            _ => panic!("edge not found"),
        }
    }
}

impl<E: Default, Ni: 'static + NodeIndex> Default for Bounded2<Ni, E> {
    fn default() -> Self {
        Bounded2([None, None])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl NodeIndex for usize {
        fn from_node_index(ni: usize) -> usize {
            ni
        }

        fn node_index(&self) -> usize {
            *self
        }
    }

    fn get_network() -> BooleanNetwork<'static, u32, u32, usize, Bounded2<usize, u32>> {
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
            (7, vec![8, 9, 10, 14], 2),
            (8, vec![12, 14], 3),
            (9, vec![13], 3),
            (10, vec![15], 3),
            (11, vec![12], 3),
        ];

        let mut network = BooleanNetwork::new(15);
        for (from, tos, node_value) in &raw {
            *network.node_value_mut(*from) = *node_value;

            for to in tos {
                network.add_edge(From(*from), To(*to));
            }
        }

        network
    }

    #[test]
    #[should_panic(
        expected = "node index out of bounds: the maximum node index is 0 but the node index is 1"
    )]
    fn invalid_index_add_edge_to() {
        let mut network = BooleanNetwork::<(), (), usize, Bounded2<_, _>>::new(0);

        network.add_edge(From(0), To(1));
    }

    #[test]
    #[should_panic(
        expected = "node index out of bounds: the maximum node index is 0 but the node index is 1"
    )]
    fn invalid_index_add_edge_from() {
        let mut network = BooleanNetwork::<(), (), usize, Bounded2<_, _>>::new(0);

        network.add_edge(From(1), To(0));
    }

    #[test]
    #[should_panic(
        expected = "node index out of bounds: the maximum node index is 0 but the node index is 1"
    )]
    fn invalid_index_add_edge_ancestors() {
        let network = BooleanNetwork::<(), (), usize, Bounded2<_, _>>::new(0);

        let _ancestors = network.ancestors(1);
    }

    #[test]
    #[should_panic(
        expected = "node index out of bounds: the maximum node index is 0 but the node index is 1"
    )]
    fn invalid_index_add_edge_descendents() {
        let network = BooleanNetwork::<(), (), usize, Bounded2<_, _>>::new(0);

        let _descendents = network.descendents(1);
    }

    #[test]
    fn nodes() {
        let network = get_network();

        let mut nodes = network.nodes();
        for i in 0..=15 {
            assert_eq!(nodes.next(), Some(i));
        }
        assert_eq!(nodes.next(), None);
    }

    #[test]
    fn ancestors_and_descendents() {
        let network = get_network();

        let mut ancestors = network.ancestors(0);
        assert_eq!(ancestors.next(), None);
        let mut descendents = network.descendents(0);
        assert_eq!(descendents.next(), Some(3));
        assert_eq!(descendents.next(), Some(5));
        assert_eq!(descendents.next(), Some(7));
        assert_eq!(descendents.next(), None);

        let mut ancestors = network.ancestors(3);
        assert_eq!(ancestors.next(), Some(0));
        assert_eq!(ancestors.next(), Some(1));
        assert_eq!(ancestors.next(), None);
        let mut descendents = network.descendents(3);
        assert_eq!(descendents.next(), Some(6));
        assert_eq!(descendents.next(), None);

        let mut ancestors = network.ancestors(13);
        assert_eq!(ancestors.next(), Some(5));
        assert_eq!(ancestors.next(), Some(9));
        assert_eq!(ancestors.next(), None);
        let mut descendents = network.descendents(13);
        assert_eq!(descendents.next(), None);
    }

    #[test]
    fn node_value() {
        let mut network = get_network();

        assert_eq!(*network.node_value(0), 0);
        assert_eq!(*network.node_value(3), 1);
        assert_eq!(*network.node_value(5), 2);
        assert_eq!(*network.node_value(8), 3);

        *network.node_value_mut(8) = 100;

        assert_eq!(*network.node_value(8), 100);
    }

    #[test]
    fn edge_value() {
        let mut network = get_network();

        *network.edge_value_mut(From(0), To(3)) = 100;
        assert_eq!(*network.edge_value_mut(From(0), To(3)), 100);
        *network.edge_value_mut(From(0), To(3)) = 1;
        assert_eq!(*network.edge_value_mut(From(0), To(3)), 1);
    }
}
