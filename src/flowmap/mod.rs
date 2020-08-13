mod flow;
pub mod label;
pub mod map;

use crate::boolean_network::*;

pub type FlowMapBooleanNetwork<Ni> = BooleanNetwork<NodeValue<Ni>, (u32, u32), Ni>;

#[derive(Clone)]
pub struct NodeValue<Ni> {
    pub symbol: Option<String>,
    pub label: Option<u32>,
    pub x_bar: Vec<Ni>,
    pub is_pi: bool,
    pub is_po: bool,
    pub flow: u32,
}

impl<Ni: 'static + NodeIndex> Default for NodeValue<Ni> {
    fn default() -> Self {
        NodeValue {
            symbol: None,
            label: None,
            x_bar: vec![],
            is_pi: false,
            is_po: false,
            flow: 0,
        }
    }
}
