use crate::boolean_network::*;
use crate::flowmap::map::LUT;
use crate::flowmap::*;
use std::collections::HashSet;
use std::io;

fn to_symbol_and_bit(s: &str) -> (&str, u32) {
    let mut symbol = s;
    let mut bit = 0;

    if let Some(open_square_index) = s.find('[') {
        let (symbol2, rest) = s.split_at(open_square_index);
        symbol = symbol2;

        assert_eq!(
            rest.chars().rev().next(),
            Some(']'),
            "symbol had open square bracket but did not end with a closing square bracket"
        );

        // The 'rest' slice includes the open square bracket, so skip one
        // character forward to ignore it. Subtract 1 from the length to skip
        // over the closing bracket, too.
        let bit_str = &rest[1..rest.len() - 1];
        bit = u32::from_str_radix(bit_str, 10).expect("symbol bit index was not an integer");
    }

    (symbol, bit)
}

pub fn write_rtlil<T: io::Write, Ni: 'static + NodeIndex>(
    mut writer: T,
    network: &FlowMapBooleanNetwork<Ni>,
    luts: &[LUT<Ni>],
    evaluate_lut: impl Fn(&LUT<Ni>) -> Vec<bool>,
) -> io::Result<()> {
    enum WireType {
        Input,
        Output,
    }

    let wires = (0..network.node_count())
        .map(|ni| {
            let ni = Ni::from_node_index(ni);
            (ni, network.node_value(ni))
        })
        .enumerate()
        .filter_map(|(i, (ni, nv))| {
            // HACK HACK HACK
            if ni.node_index() <= 1 {
                return None;
            }

            // TODO: This will require tweaking when latches are added
            let wire_type = if nv.is_pi {
                Some(WireType::Input)
            } else if nv.is_po {
                Some(WireType::Output)
            } else {
                None
            };

            if let Some(wire_type) = wire_type {
                let ident = if let Some(symbol) = &nv.symbol {
                    let (symbol, bit) = to_symbol_and_bit(&symbol);

                    (symbol.to_string(), bit)
                } else {
                    let wire_type_str = match wire_type {
                        WireType::Input => "input",
                        WireType::Output => "output",
                    };

                    (format!("{}${}", wire_type_str, i), 0)
                };

                Some((ni, ident, wire_type))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    writeln!(writer, "module \\top")?;

    for lut in luts {
        writeln!(writer, "  wire width 1 $ni${}", lut.output.node_index())?;
    }

    let mut wires_written = HashSet::new();
    for (i, (_, (symbol, _), wire_type)) in wires.iter().enumerate() {
        if !wires_written.insert(symbol) {
            continue;
        }

        let components = wires
            .iter()
            .filter(|(_, (symbol2, _), _)| symbol2 == symbol);

        let max_bit = components
            .clone()
            .map(|(_, (_, bit), _)| bit)
            .max()
            .unwrap();
        let width = max_bit + 1;

        let wire_type_str = match wire_type {
            WireType::Input => "input",
            WireType::Output => "output",
        };

        writeln!(
            writer,
            "  wire width {} {} {} \\{}",
            width, wire_type_str, i, symbol
        )?;

        for (ni, (_, bit), _) in components {
            let ni = ni.node_index();
            match wire_type {
                WireType::Input => {
                    writeln!(writer, "  wire width 1 $ni${}", ni)?;
                    writeln!(writer, "  connect $ni${} \\{} [{}]", ni, symbol, bit)?;
                }
                WireType::Output => {
                    writeln!(writer, "  connect \\{} [{}] $ni${}", symbol, bit, ni)?;
                }
            }
        }
    }

    for lut in luts {
        let output_ni = lut.output.node_index();
        let k = lut.inputs.len();
        let output_bitstring = evaluate_lut(lut)
            .iter()
            .rev()
            .map(|bit| if *bit { '1' } else { '0' })
            .collect::<String>();
        assert_eq!(output_bitstring.len(), 1 << k);

        writeln!(writer, "  cell $lut $lut${}", output_ni)?;
        writeln!(writer, "    parameter \\WIDTH {}", k)?;
        writeln!(
            writer,
            "    parameter \\LUT {}'{}",
            1 << k,
            output_bitstring
        )?;
        writeln!(writer, "    connect \\Y $ni${}", output_ni)?;
        write!(writer, "    connect \\A {{")?;
        for input in &lut.inputs {
            write!(writer, " $ni${}", input.node_index())?;
        }
        writeln!(writer, " }}")?;
        writeln!(writer, "  end")?;
    }

    writeln!(writer, "end")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_symbol_and_bit() {
        assert_eq!(to_symbol_and_bit("A"), ("A", 0));
        assert_eq!(to_symbol_and_bit("A[0]"), ("A", 0));
        assert_eq!(to_symbol_and_bit("A[1]"), ("A", 1));
        assert_eq!(to_symbol_and_bit("A[10]"), ("A", 10));
        assert_eq!(to_symbol_and_bit("A[15]"), ("A", 15));
        assert_eq!(to_symbol_and_bit("B"), ("B", 0));
        assert_eq!(
            to_symbol_and_bit("my_special_symbol"),
            ("my_special_symbol", 0)
        );
        assert_eq!(
            to_symbol_and_bit("my_special_symbol[5]"),
            ("my_special_symbol", 5)
        );
    }
}
