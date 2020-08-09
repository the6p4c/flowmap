use std::env;

mod boolean_network;
mod evaluate;
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

    const K: u32 = 6;
    flowmap::label::label_network(&mut network, K);
    let luts = flowmap::map::map(&network, K);

    for lut in luts {
        println!("{:?} <= {:?}", lut.output, lut.inputs);
        let input_literals = lut
            .inputs
            .iter()
            .map(|l| format!("{}", l.0))
            .collect::<Vec<_>>();
        let max_len = input_literals.iter().map(|s| s.len()).max().unwrap();
        let input_literals = lut
            .inputs
            .iter()
            .map(|l| format!("{:>1$}", l.0, max_len))
            .collect::<Vec<_>>();
        let header = format!("    {} | {}", input_literals.join(" "), lut.output.0);
        println!("{}", header);
        println!("    {:=>1$}", "", header.len() - 4);

        let f = evaluate::evaluate(&network, &lut);
        for i in 0..(1 << lut.inputs.len()) {
            print!("    ");

            let stim = (0..lut.inputs.len())
                .map(|j| i & (1 << j) != 0)
                .collect::<Vec<_>>();
            for s in &stim {
                print!("{:>1$} ", *s as u8, max_len);
            }

            let o = f(&stim);
            println!("| {}", o as u8);
        }
    }
}
