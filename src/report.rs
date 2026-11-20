use std::collections::BTreeMap;
use std::io::{self, Write};

use serde::Serialize;

use crate::Interval;

#[derive(Clone, Debug, Serialize)]
pub struct AuditEntry {
    pub target: Interval,
    pub blind_bases: u64,
    pub blind_percent: f64,
    pub safe_fragments: Vec<Interval>,
}

#[derive(Clone, Debug, Serialize)]
pub struct AuditSummary {
    pub targets: usize,
    pub affected_targets: usize,
    pub target_bases: u64,
    pub blind_bases: u64,
    pub blind_percent: f64,
    pub safe_fragments: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct AuditReport {
    pub coordinate_system: &'static str,
    pub summary: AuditSummary,
    pub entries: Vec<AuditEntry>,
}

impl AuditReport {
    pub fn write_table(&self, mut writer: impl Write) -> io::Result<()> {
        writeln!(
            writer,
            "target\tinterval\tblind_bases\tblind_percent\tsafe_fragments"
        )?;
        for (index, entry) in self.entries.iter().enumerate() {
            let name = entry
                .target
                .name
                .clone()
                .unwrap_or_else(|| format!("target_{}", index + 1));
            writeln!(
                writer,
                "{}\t{}:{}-{}\t{}\t{:.2}\t{}",
                name,
                entry.target.chrom,
                entry.target.start,
                entry.target.end,
                entry.blind_bases,
                entry.blind_percent,
                entry.safe_fragments.len()
            )?;
        }
        writeln!(
            writer,
            "TOTAL\t{} targets\t{}\t{:.2}\t{}",
            self.summary.targets,
            self.summary.blind_bases,
            self.summary.blind_percent,
            self.summary.safe_fragments
        )
    }
}

/// Merge overlapping or adjacent blind regions independently per chromosome.
#[must_use]
pub fn normalize(intervals: &[Interval]) -> BTreeMap<&str, Vec<(u64, u64)>> {
    let mut by_chrom: BTreeMap<&str, Vec<(u64, u64)>> = BTreeMap::new();
    for interval in intervals {
        by_chrom
            .entry(&interval.chrom)
            .or_default()
            .push((interval.start, interval.end));
    }
    for ranges in by_chrom.values_mut() {
        ranges.sort_unstable();
        let mut merged: Vec<(u64, u64)> = Vec::with_capacity(ranges.len());
        for &(start, end) in ranges.iter() {
            if let Some(last) = merged.last_mut() {
                if start <= last.1 {
                    last.1 = last.1.max(end);
                    continue;
                }
            }
            merged.push((start, end));
        }
        *ranges = merged;
    }
    by_chrom
}

fn subtract(target: &Interval, blind: &[(u64, u64)]) -> Vec<Interval> {
    let mut fragments = Vec::new();
    let mut cursor = target.start;
    for &(blind_start, blind_end) in blind {
        if blind_end <= cursor || blind_start >= target.end {
            continue;
        }
        let overlap_start = blind_start.max(target.start);
        let overlap_end = blind_end.min(target.end);
        if cursor < overlap_start {
            fragments.push((cursor, overlap_start));
        }
        cursor = cursor.max(overlap_end);
        if cursor >= target.end {
            break;
        }
    }
    if cursor < target.end {
        fragments.push((cursor, target.end));
    }
    fragments
        .into_iter()
        .enumerate()
        .map(|(index, (start, end))| {
            let name = target
                .name
                .as_ref()
                .map(|name| format!("{name}:safe:{}", index + 1));
            Interval::new(&target.chrom, start, end, name).expect("non-empty fragment")
        })
        .collect()
}

#[must_use]
pub fn audit(targets: &[Interval], blind: &[Interval]) -> AuditReport {
    let normalized = normalize(blind);
    let entries: Vec<_> = targets
        .iter()
        .map(|target| {
            let safe_fragments = subtract(
                target,
                normalized
                    .get(target.chrom.as_str())
                    .map(Vec::as_slice)
                    .unwrap_or_default(),
            );
            let safe_bases: u64 = safe_fragments.iter().map(Interval::len).sum();
            let blind_bases = target.len() - safe_bases;
            AuditEntry {
                target: target.clone(),
                blind_bases,
                blind_percent: percentage(blind_bases, target.len()),
                safe_fragments,
            }
        })
        .collect();
    let target_bases = entries.iter().map(|entry| entry.target.len()).sum();
    let blind_bases = entries.iter().map(|entry| entry.blind_bases).sum();
    AuditReport {
        coordinate_system: "BED 0-based half-open",
        summary: AuditSummary {
            targets: entries.len(),
            affected_targets: entries.iter().filter(|entry| entry.blind_bases > 0).count(),
            target_bases,
            blind_bases,
            blind_percent: percentage(blind_bases, target_bases),
            safe_fragments: entries.iter().map(|entry| entry.safe_fragments.len()).sum(),
        },
        entries,
    }
}

#[must_use]
pub fn mask(targets: &[Interval], blind: &[Interval]) -> Vec<Interval> {
    audit(targets, blind)
        .entries
        .into_iter()
        .flat_map(|entry| entry.safe_fragments)
        .collect()
}

fn percentage(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        (numerator as f64 / denominator as f64) * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn interval(chrom: &str, start: u64, end: u64) -> Interval {
        Interval::new(chrom, start, end, None).unwrap()
    }

    #[test]
    fn merges_blind_ranges_before_counting() {
        let targets = [interval("chr1", 10, 30)];
        let blind = [interval("chr1", 12, 20), interval("chr1", 18, 25)];
        let report = audit(&targets, &blind);
        assert_eq!(report.summary.blind_bases, 13);
        assert_eq!(report.entries[0].safe_fragments.len(), 2);
        assert_eq!(
            report.entries[0].safe_fragments[0],
            interval("chr1", 10, 12)
        );
        assert_eq!(
            report.entries[0].safe_fragments[1],
            interval("chr1", 25, 30)
        );
    }

    #[test]
    fn keeps_chromosomes_independent() {
        let targets = [interval("chr2", 10, 20)];
        let blind = [interval("chr1", 10, 20)];
        let report = audit(&targets, &blind);
        assert_eq!(report.summary.blind_bases, 0);
        assert_eq!(report.entries[0].safe_fragments, targets);
    }

    #[test]
    fn fully_blind_target_has_no_safe_fragment() {
        let targets = [interval("chr1", 10, 20)];
        let blind = [interval("chr1", 0, 30)];
        let report = audit(&targets, &blind);
        assert_eq!(report.summary.blind_percent, 100.0);
        assert!(report.entries[0].safe_fragments.is_empty());
    }

    #[test]
    fn adjacent_blind_regions_are_normalized() {
        let blind = [interval("chr1", 1, 5), interval("chr1", 5, 9)];
        assert_eq!(normalize(&blind)["chr1"], vec![(1, 9)]);
    }
}
