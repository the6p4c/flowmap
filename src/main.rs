use std::env;

mod boolean_network;
mod flowmap;
mod frontends;
mod test_utils;

fn main() {
    let args = env::args().collect::<Vec<_>>();
    let aiger_path = args
        .get(1)
        .expect("path to aiger file as first command line argument");

    let aiger_file = std::fs::File::open(aiger_path).unwrap();
    let aiger_reader = aiger::Reader::from_reader(aiger_file).unwrap();
    let mut network = frontends::aiger::from_reader(aiger_reader);

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

        let f = frontends::aiger::evaluate_lut(&network, &lut);
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
