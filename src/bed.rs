use std::fmt;
use std::io::{BufRead, BufReader, Read};
use std::num::ParseIntError;

use crate::Interval;

#[derive(Debug)]
pub struct BedError {
    pub line: usize,
    pub message: String,
}

impl fmt::Display for BedError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for BedError {}

/// Streaming reader for the first four BED columns.
pub struct BedReader<R> {
    lines: std::io::Lines<BufReader<R>>,
    line_number: usize,
}

impl<R: Read> BedReader<R> {
    pub fn new(reader: R) -> Self {
        Self {
            lines: BufReader::new(reader).lines(),
            line_number: 0,
        }
    }
}

impl<R: Read> Iterator for BedReader<R> {
    type Item = Result<Interval, BedError>;

    fn next(&mut self) -> Option<Self::Item> {
        for line in self.lines.by_ref() {
            self.line_number += 1;
            let raw = match line {
                Ok(raw) => raw,
                Err(error) => {
                    return Some(Err(BedError {
                        line: self.line_number,
                        message: error.to_string(),
                    }));
                }
            };
            let trimmed = raw.trim();
            if trimmed.is_empty()
                || trimmed.starts_with('#')
                || trimmed.starts_with("track ")
                || trimmed.starts_with("browser ")
            {
                continue;
            }
            return Some(parse_line(trimmed, self.line_number));
        }
        None
    }
}

fn parse_coordinate(raw: &str) -> Result<u64, ParseIntError> {
    raw.parse()
}

fn parse_line(raw: &str, line: usize) -> Result<Interval, BedError> {
    let fields: Vec<_> = raw.split('\t').collect();
    if fields.len() < 3 {
        return Err(BedError {
            line,
            message: "expected at least 3 tab-separated BED columns".to_owned(),
        });
    }
    if fields[0].is_empty() {
        return Err(BedError {
            line,
            message: "chromosome must not be empty".to_owned(),
        });
    }
    let start = parse_coordinate(fields[1]).map_err(|error| BedError {
        line,
        message: format!("invalid start '{}': {error}", fields[1]),
    })?;
    let end = parse_coordinate(fields[2]).map_err(|error| BedError {
        line,
        message: format!("invalid end '{}': {error}", fields[2]),
    })?;
    let name = fields
        .get(3)
        .filter(|value| !value.is_empty() && **value != ".")
        .map(|value| (*value).to_owned());
    Interval::new(fields[0], start, end, name).map_err(|message| BedError {
        line,
        message: message.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_comments_metadata_and_named_intervals() {
        let input = b"# comment\ntrack name=x\nchr1\t10\t20\ttarget\n\nchr2\t5\t8\n";
        let intervals = BedReader::new(&input[..])
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(intervals.len(), 2);
        assert_eq!(intervals[0].name.as_deref(), Some("target"));
        assert_eq!(intervals[1].chrom, "chr2");
    }

    #[test]
    fn reports_the_source_line_for_invalid_intervals() {
        let input = b"# comment\nchr1\t20\t10\n";
        let error = BedReader::new(&input[..]).next().unwrap().unwrap_err();
        assert_eq!(error.line, 2);
        assert!(error.to_string().contains("start must be smaller"));
    }
}
