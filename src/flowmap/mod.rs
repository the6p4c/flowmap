mod flow;
mod label;

use crate::boolean_network::*;

type FlowMapBooleanNetwork<'a, Ni, Ie> = BooleanNetwork<'a, NodeValue, (u32, u32), Ni, Ie>;

#[derive(Clone, Default)]
pub struct NodeValue {
    pub label: Option<u32>,
    pub is_pi: bool,
    pub flow: u32,
}
