use std::io::{self, Write};
use std::num::NonZero;
use std::path::Path;

use noodles::bam;
use noodles::sam;
use noodles::sam::alignment::io::Write as AlnWrite;
use rand::SeedableRng;
use rand::distributions::{Distribution, Uniform};
use rand::rngs::StdRng;
use rsomics_common::{Result, RsomicsError};

pub fn subsample_bam(
    input: &Path,
    output: &str,
    fraction: f64,
    seed: u64,
    workers: NonZero<usize>,
) -> Result<u64> {
    let mut reader = rsomics_bamio::open_with_workers(input, workers)?;
    let header = reader.read_header().map_err(RsomicsError::Io)?;

    // Real output files use the parallel BGZF writer (libdeflate); stdout falls
    // back to the single-threaded writer.
    if output == "-" {
        let mut writer = bam::io::Writer::new(io::stdout().lock());
        run(&mut reader, &mut writer, &header, fraction, seed)
    } else {
        let mut writer = rsomics_bamio::create_with_workers(Path::new(output), workers)?;
        run(&mut reader, &mut writer, &header, fraction, seed)
    }
}

fn run<W: Write>(
    reader: &mut rsomics_bamio::ParallelBamReader,
    writer: &mut bam::io::Writer<W>,
    header: &sam::Header,
    fraction: f64,
    seed: u64,
) -> Result<u64> {
    writer.write_header(header).map_err(RsomicsError::Io)?;
    let mut rng = StdRng::seed_from_u64(seed);
    let dist = Uniform::new(0.0f64, 1.0);
    let mut kept: u64 = 0;
    for result in reader.records() {
        let record = result.map_err(RsomicsError::Io)?;
        if dist.sample(&mut rng) < fraction {
            writer
                .write_alignment_record(header, &record)
                .map_err(RsomicsError::Io)?;
            kept += 1;
        }
    }
    Ok(kept)
}
