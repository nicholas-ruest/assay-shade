//! Interval primitives for auditing BED targets against assay blind regions.

mod bed;
mod interval;
mod report;

pub use bed::{BedError, BedReader};
pub use interval::Interval;
pub use report::{AuditEntry, AuditReport, AuditSummary, audit, mask};
