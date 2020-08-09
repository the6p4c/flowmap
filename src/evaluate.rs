//! Evaluation of LUTs defined on a boolean network

use crate::boolean_network::*;
use crate::flowmap::map::LUT;
use crate::flowmap::*;
use aiger::Literal;
use std::collections::HashSet;

/// The internal logic of the LUT, encoded as a recursive structure.
#[derive(Debug, PartialEq, Clone)]
enum LogicNode {
    Literal(Literal),
    And(Box<LogicNode>, Box<LogicNode>),
    Inverter(Box<LogicNode>),
    Value(bool),
}

impl LogicNode {
    /// Recursively replaces the literal `n` with the specified replacement.
    fn replace(self, n: Literal, replacement: LogicNode) -> LogicNode {
        match self {
            LogicNode::Literal(l) if l == n => replacement.clone(),
            LogicNode::Literal(l) => LogicNode::Literal(l),
            LogicNode::And(input0, input1) => {
                let input0 = Box::new(input0.replace(n, replacement.clone()));
                let input1 = Box::new(input1.replace(n, replacement.clone()));

                LogicNode::And(input0, input1)
            }
            LogicNode::Inverter(ln) => LogicNode::Inverter(Box::new(ln.replace(n, replacement))),
            LogicNode::Value(v) => LogicNode::Value(v),
        }
    }

    /// Evaluates the logic function, panicing if any unspecified values (i.e.
    /// LogicNode::Literal instances) remain.
    fn evaluate(&self) -> bool {
        match self {
            LogicNode::Literal(_) => panic!("can't evaluate logic node with literal"),
            LogicNode::And(input0, input1) => input0.evaluate() && input1.evaluate(),
            LogicNode::Inverter(ln) => !ln.evaluate(),
            LogicNode::Value(v) => *v,
        }
    }
}

/// Returns a function which can be used to determine the output value of a LUT
/// based on the value of its inputs.
///
/// The inputs to the LUT must be passed to the function returned in the same
/// order as the input literals in `lut.inputs`.
pub fn evaluate<'a>(
    network: &FlowMapBooleanNetwork<Literal>,
    lut: &'a LUT<Literal>,
) -> impl Fn(&[bool]) -> bool + 'a {
    let LUT {
        output,
        contains,
        inputs,
    } = lut;

    // TODO: This is just another topo search from the output, looking at
    // ancestors. Consider extracting this into the boolean network itself
    let mut logic = LogicNode::Literal(*output);

    let mut visited = HashSet::new();
    let mut s = vec![*output];
    while let Some(n) = s.pop() {
        if visited.contains(&n) {
            continue;
        }

        visited.insert(n);

        if !inputs.contains(&n) {
            let ancestors = network.ancestors(n);
            if n.is_inverted() {
                assert_eq!(
                    ancestors.len(),
                    1,
                    "inverter should only be driven by its non-inverted variable"
                );
                let parent = ancestors[0];

                logic = logic.replace(n, LogicNode::Inverter(Box::new(LogicNode::Literal(parent))));
            } else {
                // An AND gate should only be driven by two signals
                assert_eq!(
                    ancestors.len(),
                    2,
                    "and gate should only be driven by two literals"
                );
                let input0 = ancestors[0];
                let input1 = ancestors[1];

                logic = logic.replace(
                    n,
                    LogicNode::And(
                        Box::new(LogicNode::Literal(input0)),
                        Box::new(LogicNode::Literal(input1)),
                    ),
                );
            }

            for ancestor in ancestors {
                let remaining_descendents = network
                    .descendents(*ancestor)
                    .iter()
                    .filter(|ni| contains.contains(ni))
                    .filter(|ni| !visited.contains(ni));

                if remaining_descendents.count() == 0 {
                    s.push(*ancestor);
                }
            }
        }
    }

    move |literal_values| {
        let mut logic = logic.clone();

        for (literal, value) in inputs.iter().zip(literal_values.iter()) {
            logic = logic.replace(*literal, LogicNode::Value(*value));
        }

        logic.evaluate()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logic_node_replace() {
        let logic = LogicNode::Literal(Literal(2));

        let logic = logic.replace(Literal(2), LogicNode::Literal(Literal(4)));

        assert_eq!(logic, LogicNode::Literal(Literal(4)));
    }

    #[test]
    fn logic_node_replace_missing() {
        let logic = LogicNode::Literal(Literal(2));

        let logic = logic.replace(Literal(4), LogicNode::Literal(Literal(6)));

        assert_eq!(logic, LogicNode::Literal(Literal(2)));
    }

    #[test]
    fn logic_node_replace_value_unaffected() {
        let logic = LogicNode::Value(false);

        let logic = logic.replace(Literal(2), LogicNode::Literal(Literal(4)));

        assert_eq!(logic, LogicNode::Value(false));
    }

    #[test]
    fn logic_node_replace_and() {
        let logic = LogicNode::And(
            Box::new(LogicNode::Literal(Literal(2))),
            Box::new(LogicNode::Literal(Literal(4))),
        );

        let logic = logic.replace(Literal(2), LogicNode::Literal(Literal(6)));

        assert_eq!(
            logic,
            LogicNode::And(
                Box::new(LogicNode::Literal(Literal(6))),
                Box::new(LogicNode::Literal(Literal(4))),
            )
        );
    }

    #[test]
    fn logic_node_replace_or() {
        let logic = LogicNode::Inverter(Box::new(LogicNode::And(
            Box::new(LogicNode::Inverter(Box::new(LogicNode::Literal(Literal(
                2,
            ))))),
            Box::new(LogicNode::Inverter(Box::new(LogicNode::Literal(Literal(
                4,
            ))))),
        )));

        let logic = logic.replace(Literal(2), LogicNode::Literal(Literal(6)));

        assert_eq!(
            logic,
            LogicNode::Inverter(Box::new(LogicNode::And(
                Box::new(LogicNode::Inverter(Box::new(LogicNode::Literal(Literal(
                    6
                ))))),
                Box::new(LogicNode::Inverter(Box::new(LogicNode::Literal(Literal(
                    4
                ))))),
            )))
        );
    }

    #[test]
    fn logic_node_evaluate_value() {
        assert_eq!(LogicNode::Value(false).evaluate(), false);
        assert_eq!(LogicNode::Value(true).evaluate(), true);
    }

    #[test]
    fn logic_node_evaluate_inverter() {
        assert_eq!(
            LogicNode::Inverter(Box::new(LogicNode::Value(false))).evaluate(),
            true
        );
        assert_eq!(
            LogicNode::Inverter(Box::new(LogicNode::Value(true))).evaluate(),
            false
        );
    }

    #[test]
    fn logic_node_evaluate_and() {
        assert_eq!(
            LogicNode::And(
                Box::new(LogicNode::Value(false)),
                Box::new(LogicNode::Value(false))
            )
            .evaluate(),
            false
        );
        assert_eq!(
            LogicNode::And(
                Box::new(LogicNode::Value(false)),
                Box::new(LogicNode::Value(true))
            )
            .evaluate(),
            false
        );
        assert_eq!(
            LogicNode::And(
                Box::new(LogicNode::Value(true)),
                Box::new(LogicNode::Value(false))
            )
            .evaluate(),
            false
        );
        assert_eq!(
            LogicNode::And(
                Box::new(LogicNode::Value(true)),
                Box::new(LogicNode::Value(true))
            )
            .evaluate(),
            true
        );
    }

    #[test]
    fn evaluate_single_inverter() {
        // --2-->|~|>--3--
        let mut network = FlowMapBooleanNetwork::new(Literal(3));
        network.add_edge(From(Literal(2)), To(Literal(3)));

        let lut = LUT {
            output: Literal(3),
            contains: vec![Literal(3)],
            inputs: vec![Literal(2)],
        };
        let f = evaluate(&network, &lut);

        assert_eq!(f(&[false]), true);
        assert_eq!(f(&[true]), false);
    }

    #[test]
    fn evaluate_single_and_gate() {
        // --2-->|&|>--6--
        // --4-->| |
        let mut network = FlowMapBooleanNetwork::new(Literal(6));
        network.add_edge(From(Literal(2)), To(Literal(6)));
        network.add_edge(From(Literal(4)), To(Literal(6)));

        let lut = LUT {
            output: Literal(6),
            contains: vec![Literal(6)],
            inputs: vec![Literal(2), Literal(4)],
        };
        let f = evaluate(&network, &lut);

        assert_eq!(f(&[false, false]), false);
        assert_eq!(f(&[false, true]), false);
        assert_eq!(f(&[true, false]), false);
        assert_eq!(f(&[true, true]), true);
    }

    #[test]
    fn evaluate_single_and_gate_single_inverted_input() {
        // --2-->|~|>--3-->|&|>--6--
        // --4------------>| |
        let mut network = FlowMapBooleanNetwork::new(Literal(6));
        network.add_edge(From(Literal(2)), To(Literal(3)));
        network.add_edge(From(Literal(3)), To(Literal(6)));
        network.add_edge(From(Literal(4)), To(Literal(6)));

        let lut = LUT {
            output: Literal(6),
            contains: vec![Literal(3), Literal(6)],
            inputs: vec![Literal(2), Literal(4)],
        };
        let f = evaluate(&network, &lut);

        assert_eq!(f(&[false, false]), false);
        assert_eq!(f(&[false, true]), true);
        assert_eq!(f(&[true, false]), false);
        assert_eq!(f(&[true, true]), false);
    }

    #[test]
    fn evaluate_single_and_gate_single_inverted_input_unused_output() {
        //        8
        //        ^
        // --2-->|~|>--3-->|&|>--6--
        // --4------------>| |
        let mut network = FlowMapBooleanNetwork::new(Literal(8));
        network.add_edge(From(Literal(2)), To(Literal(3)));
        network.add_edge(From(Literal(3)), To(Literal(6)));
        network.add_edge(From(Literal(3)), To(Literal(8)));
        network.add_edge(From(Literal(4)), To(Literal(6)));

        let lut = LUT {
            output: Literal(6),
            contains: vec![Literal(3), Literal(6)],
            inputs: vec![Literal(2), Literal(4)],
        };
        let f = evaluate(&network, &lut);

        assert_eq!(f(&[false, false]), false);
        assert_eq!(f(&[false, true]), true);
        assert_eq!(f(&[true, false]), false);
        assert_eq!(f(&[true, true]), false);
    }

    #[test]
    fn evaluate_and_chain_single_inverted_input() {
        // --2-->|~|--3-->|&|>--10-->| |
        // --4----------->| |        | |
        //                           |&|>--14--
        // --6----------->|&|>--12-->| |
        // --8----------->| |        | |
        let mut network = FlowMapBooleanNetwork::new(Literal(14));
        network.add_edge(From(Literal(2)), To(Literal(3)));
        network.add_edge(From(Literal(3)), To(Literal(10)));
        network.add_edge(From(Literal(4)), To(Literal(10)));
        network.add_edge(From(Literal(6)), To(Literal(12)));
        network.add_edge(From(Literal(8)), To(Literal(12)));
        network.add_edge(From(Literal(10)), To(Literal(14)));
        network.add_edge(From(Literal(12)), To(Literal(14)));

        let lut = LUT {
            output: Literal(14),
            contains: vec![Literal(3), Literal(10), Literal(12)],
            inputs: vec![Literal(2), Literal(4), Literal(6), Literal(8)],
        };

        let f = evaluate(&network, &lut);

        assert_eq!(f(&[false, false, false, false]), false);
        assert_eq!(f(&[false, false, false, true]), false);
        assert_eq!(f(&[false, false, true, false]), false);
        assert_eq!(f(&[false, false, true, true]), false);
        assert_eq!(f(&[false, true, false, false]), false);
        assert_eq!(f(&[false, true, false, true]), false);
        assert_eq!(f(&[false, true, true, false]), false);
        assert_eq!(f(&[false, true, true, true]), true);
        assert_eq!(f(&[true, false, false, false]), false);
        assert_eq!(f(&[true, false, false, true]), false);
        assert_eq!(f(&[true, false, true, false]), false);
        assert_eq!(f(&[true, false, true, true]), false);
        assert_eq!(f(&[true, true, false, false]), false);
        assert_eq!(f(&[true, true, false, true]), false);
        assert_eq!(f(&[true, true, true, false]), false);
        assert_eq!(f(&[true, true, true, true]), false);
    }

    #[test]
    fn evaluate_single_or_gate() {
        // --2-->|~|>--3-->|&|>--6-->|~|>--7--
        // --4-->|~|>--5-->| |
        let mut network = FlowMapBooleanNetwork::new(Literal(7));
        network.add_edge(From(Literal(2)), To(Literal(3)));
        network.add_edge(From(Literal(3)), To(Literal(6)));
        network.add_edge(From(Literal(4)), To(Literal(5)));
        network.add_edge(From(Literal(5)), To(Literal(6)));
        network.add_edge(From(Literal(6)), To(Literal(7)));

        let lut = LUT {
            output: Literal(7),
            contains: vec![Literal(3), Literal(5), Literal(6)],
            inputs: vec![Literal(2), Literal(4)],
        };
        let f = evaluate(&network, &lut);

        assert_eq!(f(&[false, false]), false);
        assert_eq!(f(&[false, true]), true);
        assert_eq!(f(&[true, false]), true);
        assert_eq!(f(&[true, true]), true);
    }
}
