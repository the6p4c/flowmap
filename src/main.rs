use std::env;

mod aiger;
mod boolean_network;
mod flowmap;
mod test_utils;

use aiger::*;
use boolean_network::*;
use flowmap::*;

impl NodeIndex for Literal {
    fn from_node_index(ni: usize) -> Literal {
        Literal(ni)
    }

    fn node_index(&self) -> usize {
        self.0
    }
}

//#[derive(Default)]
//struct NodeValue {
//    is_pi: bool,
//    is_po: bool,
//    label: Option<u32>,
//}
//
//type AIG<'a> = BooleanNetwork<'a, NodeValue, (), Literal, Bounded2<Literal, ()>>;

fn main() {
    let args = env::args().collect::<Vec<_>>();
    let aiger_path = args
        .get(1)
        .expect("path to aiger file as first command line argument");

    let aiger_file = std::fs::File::open(aiger_path).unwrap();
    let aiger_reader = Reader::from_reader(aiger_file).unwrap();
    let aiger_header = aiger_reader.header();

    let max_variable = aiger_header.m;
    let max_literal = aiger_header.m * 2 + 1;
    let mut network = FlowMapBooleanNetwork::new(Literal(max_literal));

    // Add implied inverters to graph
    for variable in 0..=max_variable {
        let from = Literal::from_variable(variable, false);
        let to = Literal::from_variable(variable, true);
        network.add_edge(From(from), To(to));
    }

    network.node_value_mut(Literal(0)).label = Some(0);
    network.node_value_mut(Literal(0)).is_pi = true;

    for aiger_record in aiger_reader.records() {
        match aiger_record.unwrap() {
            Aiger::Input(l) => {
                network.node_value_mut(l).label = Some(0);
                network.node_value_mut(l).is_pi = true;
            }
            Aiger::Latch { output, input } => {
                network.node_value_mut(output).is_pi = true;
                network.node_value_mut(output).is_po = true;

                network.add_edge(From(input), To(output));
            }
            Aiger::Output(l) => {
                network.node_value_mut(l).is_po = true;
            }
            Aiger::AndGate {
                output,
                inputs: [input0, input1],
            } => {
                network.add_edge(From(input0), To(output));
                network.add_edge(From(input1), To(output));
            }
        }
    }

    flowmap::label::label_network(&mut network, 4);
    let luts = flowmap::map::map(&mut network);

    for (output, inputs) in luts {
        println!("{:?} <= {:?}", output, inputs);
    }

    //println!("digraph {{");
    //for l in 0..=max_literal {
    //    let l = Literal(l);
    //    let node_value = network.node_value(l);
    //
    //    println!(
    //        "l{} [label=\"literal {0} (variable {}{}){}{}\"];",
    //        l.0,
    //        if l.is_inverted() { "~" } else { "" },
    //        l.variable(),
    //        if node_value.is_pi { " PI" } else { "" },
    //        if node_value.is_po { " PO" } else { "" },
    //    );
    //
    //    for ancestor in network.ancestors(l) {
    //        println!("l{} -> l{};", ancestor.0, l.0);
    //    }
    //}
    //println!("}}");
}
