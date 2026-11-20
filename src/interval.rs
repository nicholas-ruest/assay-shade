use serde::Serialize;

/// A zero-based, half-open genomic interval, matching the BED convention.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Interval {
    pub chrom: String,
    pub start: u64,
    pub end: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl Interval {
    /// Construct a non-empty interval.
    pub fn new(
        chrom: impl Into<String>,
        start: u64,
        end: u64,
        name: Option<String>,
    ) -> Result<Self, &'static str> {
        if start >= end {
            return Err("BED interval start must be smaller than end");
        }
        Ok(Self {
            chrom: chrom.into(),
            start,
            end,
            name,
        })
    }

    #[must_use]
    pub const fn len(&self) -> u64 {
        self.end - self.start
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.start == self.end
    }

    #[must_use]
    pub fn overlap_len(&self, other: &Self) -> u64 {
        if self.chrom != other.chrom {
            return 0;
        }
        self.end
            .min(other.end)
            .saturating_sub(self.start.max(other.start))
    }

    #[must_use]
    pub fn to_bed_line(&self) -> String {
        match &self.name {
            Some(name) => format!("{}\t{}\t{}\t{name}", self.chrom, self.start, self.end),
            None => format!("{}\t{}\t{}", self.chrom, self.start, self.end),
        }
    }
}
