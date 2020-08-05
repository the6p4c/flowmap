//! Parsing of AIGER files

use std::io::{self, BufRead};
use std::iter;
use std::str::FromStr;

/// A literal value from an AIGER file, encoding both a variable index and "sign
/// bit" which determines if the variable is negated/inverted.
#[derive(Debug, PartialEq, Copy, Clone)]
#[repr(transparent)]
pub struct Literal(usize);

impl Literal {
    /// Returns the variable the literal refers to.
    pub fn variable(&self) -> usize {
        self.0 / 2
    }

    /// Returns true if the literal inverts the variable, or false if it does
    /// not.
    pub fn is_inverted(&self) -> bool {
        (self.0 & 1) == 1
    }
}

/// The data contained in the header of an AIGER file.
#[derive(Debug, PartialEq, Copy, Clone)]
pub struct Header {
    /// The maximum variable index.
    pub m: usize,
    /// The number of inputs.
    pub i: usize,
    /// The number of latches.
    pub l: usize,
    /// The number of outputs.
    pub o: usize,
    /// The number of AND gates.
    pub a: usize,
}

impl FromStr for Header {
    type Err = AigerError;

    // In the interest of matching both the header structure and the naming
    // convention the format itself uses, we'll use the M I L O A names.
    #[allow(clippy::many_single_char_names)]
    fn from_str(header_line: &str) -> Result<Self, Self::Err> {
        let mut components = header_line.split(' ');
        let magic = components.next().ok_or(AigerError::InvalidHeader)?;

        const HEADER_MAGIC: &str = "aag";
        if magic != HEADER_MAGIC {
            return Err(AigerError::InvalidHeader);
        }

        let mut components =
            components.map(|s| usize::from_str_radix(s, 10).map_err(|_| AigerError::InvalidHeader));

        // The remaining components of the header are all integers
        let mut get_component = || components.next().ok_or(AigerError::InvalidHeader)?;
        let m = get_component()?;
        let i = get_component()?;
        let l = get_component()?;
        let o = get_component()?;
        let a = get_component()?;

        if components.next() != None {
            // We have extra components after what should've been the last
            // component
            Err(AigerError::InvalidHeader)
        } else {
            Ok(Header { m, i, l, o, a })
        }
    }
}

/// A record from an AIGER file.
#[derive(Debug, PartialEq)]
pub enum Aiger {
    /// A literal marked as an input.
    Input(Literal),
    /// A latch.
    Latch {
        /// The literal which receives the latch's current state.
        output: Literal,
        /// The literal which determines the latch's next state.
        input: Literal,
    },
    /// A literal marked as an output.
    Output(Literal),
    /// An AND gate.
    AndGate {
        /// The literal which receives the result of the AND operation.
        output: Literal,
        /// The literals which are inputs to the AND gate.
        inputs: [Literal; 2],
    },
}

impl Aiger {
    fn parse_input(literals: &[Literal]) -> Result<Aiger, AigerError> {
        match literals {
            [input] => {
                if !input.is_inverted() {
                    Ok(Aiger::Input(*input))
                } else {
                    Err(AigerError::InvalidInverted)
                }
            }
            _ => Err(AigerError::InvalidLiteralCount),
        }
    }

    fn parse_latch(literals: &[Literal]) -> Result<Aiger, AigerError> {
        match literals {
            [output, input] => {
                if !output.is_inverted() {
                    Ok(Aiger::Latch {
                        output: *output,
                        input: *input,
                    })
                } else {
                    Err(AigerError::InvalidInverted)
                }
            }
            _ => Err(AigerError::InvalidLiteralCount),
        }
    }

    fn parse_output(literals: &[Literal]) -> Result<Aiger, AigerError> {
        match literals {
            [input] => Ok(Aiger::Output(*input)),
            _ => Err(AigerError::InvalidLiteralCount),
        }
    }

    fn parse_and_gate(literals: &[Literal]) -> Result<Aiger, AigerError> {
        match literals {
            [output, input1, input2] => {
                if !output.is_inverted() {
                    Ok(Aiger::AndGate {
                        output: *output,
                        inputs: [*input1, *input2],
                    })
                } else {
                    Err(AigerError::InvalidInverted)
                }
            }
            _ => Err(AigerError::InvalidLiteralCount),
        }
    }
}

/// An error which occurs while parsing an AIGER file.
#[derive(Debug, PartialEq)]
pub enum AigerError {
    /// No AIGER header could be found, or the header which was found could not
    /// be parsed.
    InvalidHeader,
    /// A literal which was not a positive integer was encountered.
    InvalidLiteral,
    /// Too many or too few literals were encountered for the expected type of
    /// record.
    InvalidLiteralCount,
    /// An inverted literal was encountered where an inverted literal is not
    /// allowed.
    InvalidInverted,
    /// An IO error occurred while reading.
    IoError,
}

impl From<io::Error> for AigerError {
    fn from(_error: io::Error) -> Self {
        AigerError::IoError
    }
}

/// A wrapper around a type implementing `io::Read` which reads an AIGER header
/// and AIGER records.
pub struct Reader {
    /// The AIGER header which was parsed during reader construction.
    header: Header,
    /// An iterator yielding AIGER records or errors from the lines of the
    /// reader.
    records_iter: Box<dyn Iterator<Item = Result<Aiger, AigerError>>>,
}

impl std::fmt::Debug for Reader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Reader")
            .field("header", &self.header)
            .finish()
    }
}

impl Reader {
    /// Creates a new AIGER reader which reads from the provided reader.
    ///
    /// # Example
    /// ```
    /// let readable = "aag 2 1 0 1 0\n".as_bytes();
    /// let reader = Reader::from_reader(readable);
    ///
    /// println!("{:?}", reader.header());
    ///
    /// for record in reader.records() {
    ///     println!("{:?}", record);
    /// }
    /// ```
    pub fn from_reader<T: 'static + io::Read>(reader: T) -> Result<Reader, AigerError> {
        let reader = io::BufReader::new(reader);
        let mut lines = reader.lines();

        // Parse the header
        let header_line = lines.next().ok_or(AigerError::InvalidHeader)??;
        let header = header_line.parse::<Header>()?;

        // Set up an iterator which parses each record of the file
        type ParserFunc = fn(&[Literal]) -> Result<Aiger, AigerError>;
        let parsers_input = iter::repeat(Aiger::parse_input as ParserFunc).take(header.i);
        let parsers_latch = iter::repeat(Aiger::parse_latch as ParserFunc).take(header.l);
        let parsers_output = iter::repeat(Aiger::parse_output as ParserFunc).take(header.o);
        let parsers_and_gate = iter::repeat(Aiger::parse_and_gate as ParserFunc).take(header.a);

        let parsers = parsers_input
            .chain(parsers_latch)
            .chain(parsers_output)
            .chain(parsers_and_gate);

        let records_iter = lines.zip(parsers).map(|(line, parser)| {
            let literals = line?
                .split(' ')
                .map(|s| usize::from_str_radix(s, 10).map(Literal))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| AigerError::InvalidLiteral)?;
            parser(&literals)
        });
        let records_iter = Box::new(records_iter);

        Ok(Reader {
            header,
            records_iter,
        })
    }

    /// Returns an iterator over the records in the AIGER file, consuming the
    /// reader.
    pub fn records(self) -> Box<dyn Iterator<Item = Result<Aiger, AigerError>>> {
        self.records_iter
    }

    /// Returns the AIGER header.
    pub fn header(&self) -> Header {
        self.header
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_reader(s: &'static str) -> Result<Reader, AigerError> {
        Reader::from_reader(s.as_bytes())
    }

    #[test]
    fn literal() {
        for (literal, variable, is_inverted) in &[
            (0, 0, false),
            (1, 0, true),
            (2, 1, false),
            (3, 1, true),
            (100, 50, false),
            (101, 50, true),
        ] {
            let literal = Literal(*literal);
            assert_eq!(literal.variable(), *variable);
            assert_eq!(literal.is_inverted(), *is_inverted);
        }
    }

    #[test]
    fn reader_no_header() {
        #[rustfmt::skip]
        let reader = make_reader(concat!(
            "",
        )).unwrap_err();

        assert_eq!(reader, AigerError::InvalidHeader);
    }

    #[test]
    fn reader_header_invalid_magic() {
        #[rustfmt::skip]
        let reader = make_reader(concat!(
            "axg 0 0 0 0 0\n",
        )).unwrap_err();

        assert_eq!(reader, AigerError::InvalidHeader);
    }

    #[test]
    fn reader_header_too_short() {
        #[rustfmt::skip]
        let reader = make_reader(concat!(
            "aag 0 0 0 0\n",
        )).unwrap_err();

        assert_eq!(reader, AigerError::InvalidHeader);
    }

    #[test]
    fn reader_header_too_long() {
        #[rustfmt::skip]
        let reader = make_reader(concat!(
            "aag 0 0 0 0 0 0\n",
        )).unwrap_err();

        assert_eq!(reader, AigerError::InvalidHeader);
    }

    #[test]
    fn reader_header_invalid_value() {
        #[rustfmt::skip]
        let reader = make_reader(concat!(
            "aag 0 q 0 0 0\n",
        )).unwrap_err();

        assert_eq!(reader, AigerError::InvalidHeader);
    }

    #[test]
    fn reader_invalid_literal() {
        #[rustfmt::skip]
        let reader = make_reader(concat!(
            "aag 1 0 0 1 0\n",
            "-5\n"
        )).unwrap();

        let header = reader.header();
        assert_eq!(
            header,
            Header {
                m: 1,
                i: 0,
                l: 0,
                o: 1,
                a: 0,
            }
        );

        let mut records = reader.records();
        assert_eq!(records.next(), Some(Err(AigerError::InvalidLiteral)));
    }

    #[test]
    fn reader_invalid_literal_count_too_many() {
        #[rustfmt::skip]
        let reader = make_reader(concat!(
            "aag 3 2 0 1 1\n",
            "2\n",
            "4\n",
            "6\n",
            "6 2\n", // should be 3 literals for an AND gate
        )).unwrap();

        let header = reader.header();
        assert_eq!(
            header,
            Header {
                m: 3,
                i: 2,
                l: 0,
                o: 1,
                a: 1,
            }
        );

        let mut records = reader.records();
        assert_eq!(records.next(), Some(Ok(Aiger::Input(Literal(2)))));
        assert_eq!(records.next(), Some(Ok(Aiger::Input(Literal(4)))));
        assert_eq!(records.next(), Some(Ok(Aiger::Output(Literal(6)))));
        assert_eq!(records.next(), Some(Err(AigerError::InvalidLiteralCount)));
    }

    #[test]
    fn reader_invalid_literal_count_too_few() {
        #[rustfmt::skip]
        let reader = make_reader(concat!(
            "aag 2 1 0 1 0\n",
            "2\n",
            "4 5\n",
        )).unwrap();

        let header = reader.header();
        assert_eq!(
            header,
            Header {
                m: 2,
                i: 1,
                l: 0,
                o: 1,
                a: 0,
            }
        );

        let mut records = reader.records();
        assert_eq!(records.next(), Some(Ok(Aiger::Input(Literal(2)))));
        assert_eq!(records.next(), Some(Err(AigerError::InvalidLiteralCount)));
    }
    #[test]
    fn reader_invalid_inverted_input() {
        #[rustfmt::skip]
        let reader = make_reader(concat!(
            "aag 1 1 0 0 0\n",
            "3\n",
        )).unwrap();

        let header = reader.header();
        assert_eq!(
            header,
            Header {
                m: 1,
                i: 1,
                l: 0,
                o: 0,
                a: 0,
            }
        );

        let mut records = reader.records();
        assert_eq!(records.next(), Some(Err(AigerError::InvalidInverted)));
    }

    #[test]
    fn reader_invalid_inverted_and() {
        #[rustfmt::skip]
        let reader = make_reader(concat!(
            "aag 1 0 0 0 1\n",
            "3 0 1\n",
        )).unwrap();

        let header = reader.header();
        assert_eq!(
            header,
            Header {
                m: 1,
                i: 0,
                l: 0,
                o: 0,
                a: 1,
            }
        );

        let mut records = reader.records();
        assert_eq!(records.next(), Some(Err(AigerError::InvalidInverted)));
    }

    #[test]
    fn reader_invalid_inverted_latch() {
        #[rustfmt::skip]
        let reader = make_reader(concat!(
            "aag 1 0 1 0 0\n",
            "3 0\n",
        )).unwrap();

        let header = reader.header();
        assert_eq!(
            header,
            Header {
                m: 1,
                i: 0,
                l: 1,
                o: 0,
                a: 0,
            }
        );

        let mut records = reader.records();
        assert_eq!(records.next(), Some(Err(AigerError::InvalidInverted)));
    }

    #[test]
    fn reader_empty_file() {
        #[rustfmt::skip]
        let reader = make_reader(concat!(
            "aag 0 0 0 0 0\n",
        )).unwrap();

        let header = reader.header();
        assert_eq!(
            header,
            Header {
                m: 0,
                i: 0,
                l: 0,
                o: 0,
                a: 0,
            }
        );

        let mut records = reader.records();
        assert_eq!(records.next(), None);
    }

    #[test]
    fn reader_single_output() {
        #[rustfmt::skip]
        let reader = make_reader(concat!(
            "aag 1 0 0 1 0\n",
            "2\n"
        )).unwrap();

        let header = reader.header();
        assert_eq!(
            header,
            Header {
                m: 1,
                i: 0,
                l: 0,
                o: 1,
                a: 0,
            }
        );

        let mut records = reader.records();
        assert_eq!(records.next(), Some(Ok(Aiger::Output(Literal(2)))));
        assert_eq!(records.next(), None);
    }

    #[test]
    fn reader_single_input() {
        #[rustfmt::skip]
        let reader = make_reader(concat!(
            "aag 1 1 0 0 0\n",
            "2\n",
        )).unwrap();

        let header = reader.header();
        assert_eq!(
            header,
            Header {
                m: 1,
                i: 1,
                l: 0,
                o: 0,
                a: 0,
            }
        );

        let mut records = reader.records();
        assert_eq!(records.next(), Some(Ok(Aiger::Input(Literal(2)))));
        assert_eq!(records.next(), None);
    }

    #[test]
    fn reader_and_gate() {
        #[rustfmt::skip]
        let reader = make_reader(concat!(
            "aag 3 2 0 1 1\n",
            "2\n",
            "4\n",
            "6\n",
            "6 2 4\n",
        )).unwrap();

        let header = reader.header();
        assert_eq!(
            header,
            Header {
                m: 3,
                i: 2,
                l: 0,
                o: 1,
                a: 1,
            }
        );

        let mut records = reader.records();
        assert_eq!(records.next(), Some(Ok(Aiger::Input(Literal(2)))));
        assert_eq!(records.next(), Some(Ok(Aiger::Input(Literal(4)))));
        assert_eq!(records.next(), Some(Ok(Aiger::Output(Literal(6)))));
        assert_eq!(
            records.next(),
            Some(Ok(Aiger::AndGate {
                inputs: [Literal(2), Literal(4)],
                output: Literal(6),
            }))
        );
        assert_eq!(records.next(), None);
    }

    #[test]
    fn reader_or_gate() {
        #[rustfmt::skip]
        let reader = make_reader(concat!(
            "aag 3 2 0 1 1\n",
            "2\n",
            "4\n",
            "7\n",
            "6 3 5\n",
        )).unwrap();

        let header = reader.header();
        assert_eq!(
            header,
            Header {
                m: 3,
                i: 2,
                l: 0,
                o: 1,
                a: 1,
            }
        );

        let mut records = reader.records();
        assert_eq!(records.next(), Some(Ok(Aiger::Input(Literal(2)))));
        assert_eq!(records.next(), Some(Ok(Aiger::Input(Literal(4)))));
        assert_eq!(records.next(), Some(Ok(Aiger::Output(Literal(7)))));
        assert_eq!(
            records.next(),
            Some(Ok(Aiger::AndGate {
                inputs: [Literal(3), Literal(5)],
                output: Literal(6),
            }))
        );
        assert_eq!(records.next(), None);
    }

    #[test]
    fn reader_half_adder() {
        #[rustfmt::skip]
        let reader = make_reader(concat!(
            "aag 7 2 0 2 3\n",
            "2\n",
            "4\n",
            "6\n",
            "12\n",
            "6 13 15\n",
            "12 2 4\n",
            "14 3 5\n",
        )).unwrap();

        let header = reader.header();
        assert_eq!(
            header,
            Header {
                m: 7,
                i: 2,
                l: 0,
                o: 2,
                a: 3,
            }
        );

        let mut records = reader.records();
        assert_eq!(records.next(), Some(Ok(Aiger::Input(Literal(2)))));
        assert_eq!(records.next(), Some(Ok(Aiger::Input(Literal(4)))));
        assert_eq!(records.next(), Some(Ok(Aiger::Output(Literal(6)))));
        assert_eq!(records.next(), Some(Ok(Aiger::Output(Literal(12)))));
        assert_eq!(
            records.next(),
            Some(Ok(Aiger::AndGate {
                inputs: [Literal(13), Literal(15)],
                output: Literal(6),
            }))
        );
        assert_eq!(
            records.next(),
            Some(Ok(Aiger::AndGate {
                inputs: [Literal(2), Literal(4)],
                output: Literal(12),
            }))
        );
        assert_eq!(
            records.next(),
            Some(Ok(Aiger::AndGate {
                inputs: [Literal(3), Literal(5)],
                output: Literal(14),
            }))
        );
        assert_eq!(records.next(), None);
    }

    #[test]
    fn reader_toggle_ff_en_rst() {
        let reader = make_reader(concat!(
            "aag 7 2 1 2 4\n",
            "2\n",
            "4\n",
            "6 8\n",
            "6\n",
            "7\n",
            "8 4 10\n",
            "10 13 15\n",
            "12 2 6\n",
            "14 3 7\n",
        ))
        .unwrap();

        let header = reader.header();
        assert_eq!(
            header,
            Header {
                m: 7,
                i: 2,
                l: 1,
                o: 2,
                a: 4,
            }
        );

        let mut records = reader.records();
        assert_eq!(records.next(), Some(Ok(Aiger::Input(Literal(2)))));
        assert_eq!(records.next(), Some(Ok(Aiger::Input(Literal(4)))));
        assert_eq!(
            records.next(),
            Some(Ok(Aiger::Latch {
                output: Literal(6),
                input: Literal(8),
            }))
        );
        assert_eq!(records.next(), Some(Ok(Aiger::Output(Literal(6)))));
        assert_eq!(records.next(), Some(Ok(Aiger::Output(Literal(7)))));
        assert_eq!(
            records.next(),
            Some(Ok(Aiger::AndGate {
                output: Literal(8),
                inputs: [Literal(4), Literal(10)],
            }))
        );
        assert_eq!(
            records.next(),
            Some(Ok(Aiger::AndGate {
                output: Literal(10),
                inputs: [Literal(13), Literal(15)],
            }))
        );
        assert_eq!(
            records.next(),
            Some(Ok(Aiger::AndGate {
                output: Literal(12),
                inputs: [Literal(2), Literal(6)],
            }))
        );
        assert_eq!(
            records.next(),
            Some(Ok(Aiger::AndGate {
                output: Literal(14),
                inputs: [Literal(3), Literal(7)],
            }))
        );
        assert_eq!(records.next(), None);
    }
}
