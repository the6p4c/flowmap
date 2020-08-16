use super::*;
use crate::boolean_network::*;
use hashbrown::HashSet;
use std::iter;
use std::marker::PhantomData;

#[derive(Debug, Eq, PartialEq, Copy, Clone, Hash)]
enum Position<Ni: NodeIndex> {
    Source,
    Sink,
    BeforeNode(Ni),
    AfterNode(Ni),
}

#[derive(Debug)]
struct Visited<Ni: 'static + NodeIndex> {
    source: bool,
    sink: bool,
    before: Vec<bool>,
    after: Vec<bool>,
    phantom: PhantomData<Ni>,
}

impl<Ni: 'static + NodeIndex> Visited<Ni> {
    fn new(node_count: usize) -> Visited<Ni> {
        let after = iter::repeat(false).take(node_count).collect::<Vec<_>>();

        Visited {
            source: false,
            sink: false,
            before: after.clone(),
            after,
            phantom: PhantomData,
        }
    }

    /// Mark a node as visited. Returns `true` if the node has not been visited
    /// before, or `false` if it was already marked as visited.
    fn insert(&mut self, node: Position<Ni>) -> bool {
        let visited_ref = match node {
            Position::Source => &mut self.source,
            Position::Sink => &mut self.sink,
            Position::BeforeNode(ni) => &mut self.before[ni.node_index()],
            Position::AfterNode(ni) => &mut self.after[ni.node_index()],
        };

        let old = *visited_ref;
        *visited_ref = true;

        !old
    }

    /// Returns `true` if the node has been marked as visited.
    fn contains(&self, node: Position<Ni>) -> bool {
        match node {
            Position::Source => self.source,
            Position::Sink => self.sink,
            Position::BeforeNode(ni) => self.before[ni.node_index()],
            Position::AfterNode(ni) => self.after[ni.node_index()],
        }
    }
}

#[derive(Debug, PartialEq)]
struct PathStep<Ni: NodeIndex> {
    from: Position<Ni>,
    to: Position<Ni>,
}

#[derive(Debug)]
struct Path<Ni: NodeIndex> {
    source: Option<Position<Ni>>,
    sink: Option<Position<Ni>>,
    before: Vec<Option<Position<Ni>>>,
    after: Vec<Option<Position<Ni>>>,
}

impl<Ni: NodeIndex + std::fmt::Debug> Path<Ni> {
    fn new(node_count: usize) -> Path<Ni> {
        let after = iter::repeat(None).take(node_count).collect::<Vec<_>>();

        Path {
            source: None,
            sink: None,
            before: after.clone(),
            after,
        }
    }

    /// Returns the node used to access `to` in the current path.
    fn get_from(&self, to: Position<Ni>) -> Option<Position<Ni>> {
        match to {
            Position::Source => self.source,
            Position::Sink => self.sink,
            Position::BeforeNode(ni) => self.before[ni.node_index()],
            Position::AfterNode(ni) => self.after[ni.node_index()],
        }
    }

    /// Sets the "from" node for a "to" node, i.e. the node `from` which was
    /// used to access `to`.
    fn set_from(&mut self, from: Position<Ni>, to: Position<Ni>) {
        let from_ref = match to {
            Position::Source => &mut self.source,
            Position::Sink => &mut self.sink,
            Position::BeforeNode(ni) => &mut self.before[ni.node_index()],
            Position::AfterNode(ni) => &mut self.after[ni.node_index()],
        };

        *from_ref = Some(from);
    }

    /// Returns an iterator over the steps in the path, working backwards from
    /// the last node in the path `last`.
    fn path_rev(&self, last: Position<Ni>) -> impl Iterator<Item = PathStep<Ni>> + '_ {
        let mut prev_to = Some(last);

        iter::from_fn(move || {
            if let Some(to) = prev_to {
                let from = self.get_from(to);
                if let Some(from) = from {
                    let path_step = PathStep { from: from, to: to };

                    prev_to = Some(from);

                    Some(path_step)
                } else {
                    None
                }
            } else {
                None
            }
        })
    }
}

enum NetworkEdgeDirection {
    Descendent,
    Ancestor,
}

pub struct Flow<'a, Ni: 'static + NodeIndex + std::fmt::Debug> {
    network: &'a mut FlowMapBooleanNetwork<Ni>,
    node: Ni,
    source: Vec<(Ni, u32)>,
    sink: Vec<(Ni, u32)>,
}

impl<Ni: NodeIndex + std::fmt::Debug> Flow<'_, Ni> {
    pub fn new<'a>(
        network: &'a mut FlowMapBooleanNetwork<Ni>,
        node: Ni,
        source: &[Ni],
        sink: &[Ni],
    ) -> Flow<'a, Ni> {
        Flow {
            network,
            node,
            source: source.iter().map(|ni| (*ni, 0)).collect(),
            sink: sink.iter().map(|ni| (*ni, 0)).collect(),
        }
    }

    pub fn step(&mut self) -> bool {
        let mut visited = Visited::<Ni>::new(self.network.node_count());
        let mut path = Path::new(self.network.node_count());
        let mut s: Vec<Position<Ni>> = vec![Position::Source];

        while let Some(p) = s.pop() {
            if !visited.insert(p) {
                continue;
            }

            for descendent in self.descendents(p) {
                if visited.contains(descendent) {
                    continue;
                }

                // Descendents are "forward" edges which can only be travelled
                // on if the current capacity is non-zero
                let (_, cap) = self.flow_cap(p, descendent);
                if cap > 0 {
                    path.set_from(p, descendent);
                    s.push(descendent);
                }
            }

            for ancestor in self.ancestors(p) {
                if visited.contains(ancestor) {
                    continue;
                }

                // Ancestors are "backwards" edges which can only be travelled
                // on if the current flow is non-zero
                let (flow, _) = self.flow_cap(ancestor, p);
                if flow > 0 {
                    path.set_from(p, ancestor);
                    s.push(ancestor);
                }
            }
        }

        // Did we fail to find an augmenting path?
        if !visited.contains(Position::Sink) {
            return false;
        }

        for path_step in path.path_rev(Position::Sink) {
            self.augment(path_step.from, path_step.to, 1);
        }

        true
    }

    pub fn cut(&self, orig: &HashSet<Ni>) -> Vec<Ni> {
        let mut reachable = HashSet::new();
        let mut visited = HashSet::new();
        let mut s = vec![Position::Source];
        while let Some(n) = s.pop() {
            if !visited.insert(n) {
                continue;
            }

            match n {
                Position::BeforeNode(n) | Position::AfterNode(n) => {
                    reachable.insert(n);
                }
                _ => {}
            }

            for descendent in self.descendents(n) {
                if self.is_undirected_path(n, descendent, NetworkEdgeDirection::Descendent) {
                    s.push(descendent);
                }
            }

            for ancestor in self.ancestors(n) {
                if self.is_undirected_path(n, ancestor, NetworkEdgeDirection::Ancestor) {
                    s.push(ancestor);
                }
            }
        }

        // Our "reachable" set is X'', so generate \bar{X}''
        orig.difference(&reachable).copied().collect()
    }

    fn descendents(&self, position: Position<Ni>) -> Box<dyn Iterator<Item = Position<Ni>> + '_> {
        match position {
            Position::Source => {
                Box::new(self.source.iter().map(|(ni, _)| Position::BeforeNode(*ni)))
            }
            Position::Sink => Box::new(iter::empty()),
            Position::BeforeNode(ni) => Box::new(iter::once(Position::AfterNode(ni))),
            Position::AfterNode(ni) => {
                if self.sink.iter().any(|(ni2, _)| *ni2 == ni) {
                    Box::new(iter::once(Position::Sink))
                } else {
                    Box::new(self.network.descendents(ni).iter().map(move |ni| {
                        if *ni == self.node {
                            Position::Sink
                        } else {
                            Position::BeforeNode(*ni)
                        }
                    }))
                }
            }
        }
    }

    fn ancestors(&self, position: Position<Ni>) -> Box<dyn Iterator<Item = Position<Ni>> + '_> {
        match position {
            Position::Source => Box::new(iter::empty()),
            Position::Sink => Box::new(self.sink.iter().map(|(ni, _)| Position::AfterNode(*ni))),
            Position::BeforeNode(ni) => Box::new(
                self.network
                    .ancestors(ni)
                    .iter()
                    .map(|ni| Position::AfterNode(*ni)),
            ),
            Position::AfterNode(ni) => Box::new(iter::once(Position::BeforeNode(ni))),
        }
    }

    /// Returns the current flow and current capacity (i.e. the capacity of the
    /// edge, minus the current flow) of the provided edge.
    fn flow_cap(&self, from: Position<Ni>, to: Position<Ni>) -> (u32, u32) {
        match (from, to) {
            (Position::Source, Position::BeforeNode(ni)) => self
                .source
                .iter()
                .find_map(|(ni2, flow)| {
                    if *ni2 == ni {
                        // TODO: Handle infinite capacity better
                        Some((*flow, 1000))
                    } else {
                        None
                    }
                })
                .unwrap_or((0, 0)),
            (Position::BeforeNode(ni1), Position::AfterNode(ni2)) if ni1 == ni2 => {
                let flow = self.network.node_value(ni1).flow;

                (flow, 1 - flow)
            }
            (Position::AfterNode(ni1), Position::BeforeNode(ni2)) => {
                *self.network.edge_value(From(ni1), To(ni2))
            }
            (Position::AfterNode(ni), Position::Sink) => self
                .sink
                .iter()
                .find_map(|(ni2, flow)| {
                    if *ni2 == ni {
                        // TODO: Handle infinite capacity better
                        Some((*flow, 1000))
                    } else {
                        None
                    }
                })
                .unwrap_or((0, 0)),
            _ => (0, 0),
        }
    }

    /// "Augments" an edge, adding the provided value to the flow of forward
    /// edges and subtracting it from the flow of backward edges.
    fn augment(&mut self, from: Position<Ni>, to: Position<Ni>, f: u32) {
        match (from, to) {
            (Position::Source, Position::BeforeNode(ni)) => {
                for (ni2, flow) in &mut self.source {
                    if *ni2 == ni {
                        *flow += f;
                        return;
                    }
                }
            }
            (Position::BeforeNode(ni), Position::Source) => {
                for (ni2, flow) in &mut self.source {
                    if *ni2 == ni {
                        *flow -= f;
                        return;
                    }
                }
            }
            (Position::BeforeNode(ni1), Position::AfterNode(ni2)) if ni1 == ni2 => {
                self.network.node_value_mut(ni1).flow += f;
            }
            (Position::AfterNode(ni1), Position::BeforeNode(ni2)) if ni1 == ni2 => {
                self.network.node_value_mut(ni1).flow -= f;
            }
            (Position::AfterNode(ni1), Position::BeforeNode(ni2)) => {
                let (flow, cap) = self.network.edge_value_mut(From(ni1), To(ni2));
                *flow += f;
                *cap -= f;
            }
            (Position::BeforeNode(ni1), Position::AfterNode(ni2)) => {
                let (flow, cap) = self.network.edge_value_mut(From(ni2), To(ni1));
                *flow -= f;
                *cap += f;
            }
            (Position::AfterNode(ni), Position::Sink) => {
                for (ni2, flow) in &mut self.sink {
                    if *ni2 == ni {
                        // TODO: Handle infinite capacity better
                        *flow += f;
                    }
                }
            }
            (Position::Sink, Position::AfterNode(ni)) => {
                for (ni2, flow) in &mut self.sink {
                    if *ni2 == ni {
                        // TODO: Handle infinite capacity better
                        *flow -= f;
                    }
                }
            }
            _ => {}
        }
    }

    /// Returns `true` if there is an undirected edge between the provided nodes
    /// on the residual graph, or `false` if there is no undirected edge.
    ///
    /// The direction of the edge on the original network must be supplied
    /// (`network_edge_direction`) to allow the edge to be found in the network.
    fn is_undirected_path(
        &self,
        from: Position<Ni>,
        to: Position<Ni>,
        network_edge_direction: NetworkEdgeDirection,
    ) -> bool {
        let (flow, cap) = match network_edge_direction {
            NetworkEdgeDirection::Descendent => self.flow_cap(from, to),
            NetworkEdgeDirection::Ancestor => self.flow_cap(to, from),
        };

        let is_undir_path_forward = cap != 0;
        let is_undir_path_backward = flow != 0;

        match (from, to) {
            // In the original network, an edge between the source and a before
            // node of a node is always from the source to the before node
            (Position::Source, Position::BeforeNode(_)) => is_undir_path_forward,
            (Position::BeforeNode(_), Position::Source) => is_undir_path_backward,

            // In the original network, an edge between a before node and after
            // node *for the same original node* is always from the before node
            // to the after node
            (Position::BeforeNode(ni1), Position::AfterNode(ni2)) if ni1 == ni2 => {
                is_undir_path_forward
            }
            (Position::AfterNode(ni1), Position::BeforeNode(ni2)) if ni1 == ni2 => {
                is_undir_path_backward
            }

            // In the original network, an edge between an after node and before
            // node *for different original nodes* is always from the after node
            // to the before node (the original nodes must be different here,
            // otherwise the above cases would have caught it)
            (Position::AfterNode(_), Position::BeforeNode(_)) => is_undir_path_forward,
            (Position::BeforeNode(_), Position::AfterNode(_)) => is_undir_path_backward,

            // In the original network, an edge between an after node and the
            // sink is always from the after node to the sink
            (Position::AfterNode(_), Position::Sink) => is_undir_path_forward,
            (Position::Sink, Position::AfterNode(_)) => is_undir_path_backward,

            // An edge cannot exist between any other pair of nodes in the
            // original network
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visited() {
        let mut visited = Visited::<usize>::new(2);

        assert_eq!(visited.contains(Position::Source), false);
        assert_eq!(visited.contains(Position::BeforeNode(0)), false);
        assert_eq!(visited.contains(Position::AfterNode(0)), false);
        assert_eq!(visited.contains(Position::BeforeNode(1)), false);
        assert_eq!(visited.contains(Position::AfterNode(1)), false);
        assert_eq!(visited.contains(Position::Sink), false);

        assert_eq!(visited.insert(Position::Source), true);

        assert_eq!(visited.contains(Position::Source), true);
        assert_eq!(visited.contains(Position::BeforeNode(0)), false);
        assert_eq!(visited.contains(Position::AfterNode(0)), false);
        assert_eq!(visited.contains(Position::BeforeNode(1)), false);
        assert_eq!(visited.contains(Position::AfterNode(1)), false);
        assert_eq!(visited.contains(Position::Sink), false);

        assert_eq!(visited.insert(Position::Source), false);
        assert_eq!(visited.insert(Position::BeforeNode(0)), true);

        assert_eq!(visited.contains(Position::BeforeNode(0)), true);
        assert_eq!(visited.contains(Position::AfterNode(0)), false);
        assert_eq!(visited.contains(Position::BeforeNode(1)), false);
        assert_eq!(visited.contains(Position::AfterNode(1)), false);
        assert_eq!(visited.contains(Position::Sink), false);

        assert_eq!(visited.insert(Position::BeforeNode(0)), false);
        assert_eq!(visited.insert(Position::AfterNode(1)), true);

        assert_eq!(visited.contains(Position::BeforeNode(0)), true);
        assert_eq!(visited.contains(Position::AfterNode(0)), false);
        assert_eq!(visited.contains(Position::BeforeNode(1)), false);
        assert_eq!(visited.contains(Position::AfterNode(1)), true);
        assert_eq!(visited.contains(Position::Sink), false);

        assert_eq!(visited.insert(Position::AfterNode(1)), false);
        assert_eq!(visited.insert(Position::Sink), true);

        assert_eq!(visited.contains(Position::BeforeNode(0)), true);
        assert_eq!(visited.contains(Position::AfterNode(0)), false);
        assert_eq!(visited.contains(Position::BeforeNode(1)), false);
        assert_eq!(visited.contains(Position::AfterNode(1)), true);
        assert_eq!(visited.contains(Position::Sink), true);

        assert_eq!(visited.insert(Position::Sink), false);
    }

    #[test]
    fn path() {
        let mut path = Path::<usize>::new(9);

        // Emulate the steps a DFS would take through the following graph while
        // finding a path towards 8:
        //      0
        //     / \
        //    1   4
        //   / \   \
        //  2  3   5 -- 8
        //        / \
        //       6   7
        path.set_from(Position::BeforeNode(0), Position::BeforeNode(1));
        path.set_from(Position::BeforeNode(0), Position::BeforeNode(4));
        path.set_from(Position::BeforeNode(1), Position::BeforeNode(2));
        path.set_from(Position::BeforeNode(1), Position::BeforeNode(3));
        path.set_from(Position::BeforeNode(4), Position::BeforeNode(5));
        path.set_from(Position::BeforeNode(5), Position::BeforeNode(6));
        path.set_from(Position::BeforeNode(5), Position::BeforeNode(7));
        path.set_from(Position::BeforeNode(5), Position::BeforeNode(8));

        let path_rev = path.path_rev(Position::BeforeNode(8)).collect::<Vec<_>>();
        assert_eq!(
            path_rev,
            vec![
                PathStep {
                    from: Position::BeforeNode(5),
                    to: Position::BeforeNode(8),
                },
                PathStep {
                    from: Position::BeforeNode(4),
                    to: Position::BeforeNode(5),
                },
                PathStep {
                    from: Position::BeforeNode(0),
                    to: Position::BeforeNode(4),
                },
            ]
        );
    }
}
