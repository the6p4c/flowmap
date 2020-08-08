use super::flow::*;
use super::*;

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
) -> Vec<(Ni, Vec<Ni>)> {
    let mut luts = vec![];

    let mut s = (0..network.node_count())
        .map(Ni::from_node_index)
        .filter(|ni| network.node_value(*ni).is_po)
        .collect::<Vec<_>>();
    while let Some(n) = s.pop() {
        let node_value = network.node_value(n);
        if node_value.is_pi && !node_value.is_po {
            continue;
        }

        dbg!(n, &node_value.x_bar);
        let inputs = inputs(&network, &node_value.x_bar);
        luts.push((n, inputs.clone()));

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
}
