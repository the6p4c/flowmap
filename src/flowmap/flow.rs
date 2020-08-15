use super::*;
use crate::boolean_network::*;
use hashbrown::{HashMap, HashSet};
use std::iter;

#[derive(Debug, Eq, PartialEq, Copy, Clone, Hash)]
enum Position<Ni: NodeIndex> {
    Source,
    Sink,
    BeforeNode(Ni),
    AfterNode(Ni),
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
        let mut visited = HashSet::new();
        let mut path = HashMap::new();
        let mut s = vec![Position::Source];
        while let Some(p) = s.pop() {
            if !visited.insert(p) {
                continue;
            }

            if p == Position::Sink {
                break;
            }

            for descendent in self.descendents(p) {
                if visited.contains(&descendent) {
                    continue;
                }

                let (_, cap) = self.flow_cap(p, descendent);
                if cap > 0 {
                    path.insert(descendent, p);
                    s.push(descendent);
                }
            }

            for ancestor in self.ancestors(p) {
                if visited.contains(&ancestor) {
                    continue;
                }

                let (flow, _) = self.flow_cap(ancestor, p);
                if flow > 0 {
                    path.insert(ancestor, p);
                    s.push(ancestor);
                }
            }
        }

        // Did we fail to find an augmenting path?
        if !visited.contains(&Position::Sink) {
            return false;
        }

        let mut to = Position::Sink;
        loop {
            let from = *path
                .get(&to)
                .expect("node should have a parent in the path");
            self.augment(from, to, 1);

            to = from;

            if from == Position::Source {
                break;
            }
        }

        true
    }

    pub fn cut(&self, orig: &HashSet<Ni>) -> Vec<Ni> {
        let mut reachable = HashSet::new();
        let mut visited = HashSet::new();
        let mut s = vec![Position::Source];
        while let Some(n) = s.pop() {
            if visited.insert(n) {
                match n {
                    Position::Source => {}
                    Position::BeforeNode(n) => {
                        reachable.insert(n);
                    }
                    Position::AfterNode(n) => {
                        reachable.insert(n);
                    }
                    Position::Sink => continue,
                }

                for descendent in self.descendents(n) {
                    if self.is_undir_path(n, descendent, false) {
                        s.push(descendent);
                    }
                }

                for ancestor in self.ancestors(n) {
                    if self.is_undir_path(ancestor, n, true) {
                        s.push(ancestor);
                    }
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

    fn flow_cap(&self, from: Position<Ni>, to: Position<Ni>) -> (u32, u32) {
        match (from, to) {
            (Position::Source, Position::BeforeNode(ni)) => {
                for (ni2, flow) in &self.source {
                    if *ni2 == ni {
                        // TODO: Handle infinite capacity better
                        return (*flow, 1000);
                    }
                }
                (0, 0)
            }
            (Position::BeforeNode(ni1), Position::AfterNode(ni2)) if ni1 == ni2 => {
                let flow = self.network.node_value(ni1).flow;

                (flow, 1 - flow)
            }
            (Position::AfterNode(ni1), Position::BeforeNode(ni2)) => {
                *self.network.edge_value(From(ni1), To(ni2))
            }
            (Position::AfterNode(ni), Position::Sink) => {
                for (ni2, flow) in &self.sink {
                    if *ni2 == ni {
                        // TODO: Handle infinite capacity better
                        return (*flow, 1000);
                    }
                }
                (0, 0)
            }
            _ => (0, 0),
        }
    }

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

    fn is_undir_path(&self, from: Position<Ni>, to: Position<Ni>, swap: bool) -> bool {
        let (flow, cap) = self.flow_cap(from, to);
        let is_undir_path_fwd = cap != 0;
        let is_undir_path_bkw = flow != 0;

        let x = if swap { (to, from) } else { (from, to) };

        match x {
            (Position::Source, Position::BeforeNode(_)) => is_undir_path_fwd,
            (Position::BeforeNode(_), Position::Source) => is_undir_path_bkw,
            (Position::AfterNode(ni1), Position::BeforeNode(ni2)) if ni1 == ni2 => {
                is_undir_path_bkw
            }
            (Position::BeforeNode(ni1), Position::AfterNode(ni2)) if ni1 == ni2 => {
                is_undir_path_fwd
            }
            (Position::AfterNode(_), Position::BeforeNode(_)) => is_undir_path_fwd,
            (Position::BeforeNode(_), Position::AfterNode(_)) => is_undir_path_bkw,
            (Position::AfterNode(_), Position::Sink) => is_undir_path_fwd,
            (Position::Sink, Position::AfterNode(_)) => is_undir_path_bkw,
            _ => false,
        }
    }
}
