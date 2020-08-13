use std::env;

mod backends;
mod boolean_network;
mod flowmap;
mod frontends;
mod test_utils;

fn main() {
    let args = env::args().collect::<Vec<_>>();
    let aiger_path = args
        .get(1)
        .expect("path to aiger file as first command line argument");
    let rtlil_path = args
        .get(2)
        .expect("path to rtlil file as second command line argument");

    let aiger_file = std::fs::File::open(aiger_path).unwrap();
    let aiger_reader = aiger::Reader::from_reader(aiger_file).unwrap();
    let mut network = frontends::aiger::from_reader(aiger_reader);

    const K: u32 = 6;
    flowmap::label::label_network(&mut network, K);
    let luts = flowmap::map::map(&network, K);

    let rtlil_file = std::fs::File::create(rtlil_path).unwrap();
    backends::rtlil::write_rtlil(rtlil_file, &network, &luts, |lut| {
        let f = frontends::aiger::evaluate_lut(&network, lut);

        let num_bits = lut.inputs.len();
        let max_input = (1 << num_bits) - 1;
        (0..=max_input)
            .map(|i| {
                let bits = (0..num_bits)
                    .rev()
                    .map(|bit| i & (1 << bit) != 0)
                    .collect::<Vec<_>>();

                f(&bits)
            })
            .collect()
    })
    .unwrap();
}
