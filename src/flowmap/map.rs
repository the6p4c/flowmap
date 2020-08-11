use super::*;
use std::collections::HashSet;

#[derive(Debug, PartialEq, Clone)]
pub struct LUT<Ni: NodeIndex> {
    /// The node that the LUT generates.
    pub output: Ni,
    /// The nodes which serve as inputs to the LUT.
    pub inputs: Vec<Ni>,
    /// The nodes which the LUT replaces.
    pub contains: Vec<Ni>,
}

fn inputs<Ni: 'static + NodeIndex + std::fmt::Debug>(
    network: &FlowMapBooleanNetwork<Ni>,
    x_bar: &[Ni],
) -> Vec<Ni> {
    let mut inputs = vec![];

    for n in x_bar {
        for ancestor in network.ancestors(*n) {
            if !x_bar.contains(ancestor) && !inputs.contains(ancestor) {
                inputs.push(*ancestor);
            }
        }
    }

    inputs
}

pub fn map<Ni: 'static + NodeIndex + std::fmt::Debug>(
    network: &FlowMapBooleanNetwork<Ni>,
    k: u32,
) -> Vec<LUT<Ni>> {
    let mut done = HashSet::new();
    let mut luts = vec![];

    let mut s = (0..network.node_count())
        .map(Ni::from_node_index)
        .filter(|ni| network.node_value(*ni).is_po)
        .collect::<Vec<_>>();
    while let Some(n) = s.pop() {
        if !done.insert(n) {
            continue;
        }

        let node_value = network.node_value(n);
        if node_value.is_pi && !node_value.is_po {
            continue;
        }

        let inputs = inputs(&network, &node_value.x_bar);
        luts.push(LUT {
            output: n,
            inputs: inputs.clone(),
            contains: node_value.x_bar.clone(),
        });

        let num_inputs = inputs.len();
        assert!(
            num_inputs > 0 && num_inputs <= (k as usize),
            "number of inputs to LUT generating {:?} was {}, however K is {}",
            n,
            num_inputs,
            k
        );

        for i in inputs {
            s.push(i);
        }
    }

    luts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input() {
        // Fig. 5(a) from FlowMap paper, numbered top-to-bottom, left-to-right.
        let mut network = FlowMapBooleanNetwork::new(12);

        network.add_edge(From(0), To(5));
        network.add_edge(From(1), To(5));
        network.add_edge(From(1), To(6));
        network.add_edge(From(2), To(6));
        network.add_edge(From(3), To(7));
        network.add_edge(From(4), To(7));
        network.add_edge(From(5), To(8));
        network.add_edge(From(5), To(12));
        network.add_edge(From(6), To(8));
        network.add_edge(From(6), To(10));
        network.add_edge(From(7), To(9));
        network.add_edge(From(7), To(11));
        network.add_edge(From(8), To(9));
        network.add_edge(From(9), To(10));
        network.add_edge(From(10), To(11));
        network.add_edge(From(11), To(12));

        assert_eq!(
            {
                let mut v = inputs(&network, &[5]);
                v.sort();
                v
            },
            vec![0, 1]
        );
        assert_eq!(
            {
                let mut v = inputs(&network, &[6]);
                v.sort();
                v
            },
            vec![1, 2]
        );
        assert_eq!(
            {
                let mut v = inputs(&network, &[7]);
                v.sort();
                v
            },
            vec![3, 4]
        );
        assert_eq!(
            {
                let mut v = inputs(&network, &[5, 6]);
                v.sort();
                v
            },
            vec![0, 1, 2]
        );
        assert_eq!(
            {
                let mut v = inputs(&network, &[6, 7]);
                v.sort();
                v
            },
            vec![1, 2, 3, 4]
        );
        assert_eq!(
            {
                let mut v = inputs(&network, &[5, 6, 7]);
                v.sort();
                v
            },
            vec![0, 1, 2, 3, 4]
        );
        assert_eq!(
            {
                let mut v = inputs(&network, &[9, 10, 11]);
                v.sort();
                v
            },
            vec![6, 7, 8]
        );
        assert_eq!(
            {
                let mut v = inputs(&network, &[10, 11]);
                v.sort();
                v
            },
            vec![6, 7, 9]
        );
    }

    #[test]
    fn map_test() {
        // Fig. 5(a) from FlowMap paper, numbered top-to-bottom, left-to-right.
        let mut network = FlowMapBooleanNetwork::<usize>::new(12);

        network.add_edge(From(0), To(5));
        network.add_edge(From(1), To(5));
        network.add_edge(From(1), To(6));
        network.add_edge(From(2), To(6));
        network.add_edge(From(3), To(7));
        network.add_edge(From(4), To(7));
        network.add_edge(From(5), To(8));
        network.add_edge(From(5), To(12));
        network.add_edge(From(6), To(8));
        network.add_edge(From(6), To(10));
        network.add_edge(From(7), To(9));
        network.add_edge(From(7), To(11));
        network.add_edge(From(8), To(9));
        network.add_edge(From(9), To(10));
        network.add_edge(From(10), To(11));
        network.add_edge(From(11), To(12));

        // Mark PI nodes
        network.node_value_mut(0).is_pi = true;
        network.node_value_mut(1).is_pi = true;
        network.node_value_mut(2).is_pi = true;
        network.node_value_mut(3).is_pi = true;
        network.node_value_mut(4).is_pi = true;

        // Mark PO node
        network.node_value_mut(12).is_po = true;

        // Give nodes their correct \bar{X} set
        // We'll skip the nodes which shouldn't have LUTs generated for them, as
        // we should never need to access those sets.
        network.node_value_mut(5).x_bar = vec![5];
        network.node_value_mut(6).x_bar = vec![6];
        network.node_value_mut(7).x_bar = vec![7];
        network.node_value_mut(12).x_bar = vec![8, 9, 10, 11, 12];

        let luts = map(&network, 3);

        // As per Fig. 4 (c), we should end up with 4 LUTs
        assert_eq!(luts.len(), 4);
        assert!(luts.contains(&LUT {
            output: 5,
            inputs: vec![0, 1],
            contains: vec![5],
        }));
        assert!(luts.contains(&LUT {
            output: 6,
            inputs: vec![1, 2],
            contains: vec![6],
        }));
        assert!(luts.contains(&LUT {
            output: 7,
            inputs: vec![3, 4],
            contains: vec![7],
        }));
        assert!(luts.contains(&LUT {
            output: 12,
            inputs: vec![5, 6, 7],
            contains: vec![8, 9, 10, 11, 12],
        }));
    }
}
