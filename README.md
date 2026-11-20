# assay-shade

`assay-shade` is a small Rust CLI for making assay blind spots explicit. Give it
BED targets and BED regions where an assay is unreliable or unobservable; it
reports the affected bases without double-counting overlapping blind regions,
and can emit only the remaining trustworthy fragments.

This is useful for probe exclusions, mapping ambiguity, fragmentation gaps,
labeling-induced blind regions, or any other interval-shaped limitation that
should travel with a genomic result.

## Install

Requires stable Rust 1.85 or newer.

```console
cargo install --git https://github.com/nicholas-ruest/assay-shade
```

Or build from a clone:

```console
cargo build --release
```

## Audit targets

Input follows the BED convention: zero-based, half-open coordinates. The first
three columns are required; the optional fourth column supplies a name. Blank
lines, comments, `track`, and `browser` metadata lines are ignored.

```console
assay-shade audit \
  --targets examples/targets.bed \
  --blind examples/blind-regions.bed
```

Example output:

```text
target          interval      blind_bases  blind_percent  safe_fragments
promoter-alpha  chr1:100-180  40           50.00          2
enhancer-beta   chr1:220-300  0            0.00           1
amplicon-gamma  chr2:40-90    10           20.00          1
TOTAL           3 targets     50           23.81          4
```

Use JSON for pipelines:

```console
assay-shade audit --targets targets.bed --blind blind.bed --format json
```

Add an automated quality gate. The command fails when the percentage of all
target bases covered by blind regions is greater than the limit, using exit
status `2` for a threshold violation and `1` for input or runtime errors:

```console
assay-shade audit --targets targets.bed --blind blind.bed --fail-above 5
```

## Emit trustworthy fragments

```console
assay-shade mask \
  --targets examples/targets.bed \
  --blind examples/blind-regions.bed \
  --output safe-targets.bed
```

Blind regions are sorted and unioned independently per chromosome before any
coverage calculation or subtraction. Adjacent regions are coalesced. A target
that is completely blind produces no safe fragment.

## Verify

```console
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

## Scope

`assay-shade` performs interval quality control. It does not infer biological
causality, repair coordinate systems across genome assemblies, or decide that
an uncovered interval is experimentally valid. Blind-region completeness is
the caller's responsibility.

## License

MIT
