mod flow;
mod label;

use crate::boolean_network::*;

type FlowMapBooleanNetwork<'a, Ni, Ie> = BooleanNetwork<'a, NodeValue<Ni>, (u32, u32), Ni, Ie>;

#[derive(Clone)]
pub struct NodeValue<Ni> {
    pub label: Option<u32>,
    pub x_bar: Vec<Ni>,
    pub is_pi: bool,
    pub flow: u32,
}

impl<Ni: 'static + NodeIndex> Default for NodeValue<Ni> {
    fn default() -> Self {
        NodeValue {
            label: None,
            x_bar: vec![],
            is_pi: false,
            flow: 0,
        }
    }
}
