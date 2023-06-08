use anyhow::{anyhow, Result};
use byteorder::{WriteBytesExt, LE};
use clap::Parser;
use rayon::prelude::*;
use std::{
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, BufWriter, Write},
    path::PathBuf,
    str::FromStr,
};

/// Signal Server Data (SDF) conversion utility.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Opts {
    /// SDF files to convert to binary (BSDF)
    input: Vec<PathBuf>,
    /// Output directory
    #[arg(short, long)]
    out: Option<PathBuf>,
}

fn main() {
    match go() {
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1)
        }
        Ok(()) => (),
    }
}

fn go() -> anyhow::Result<()> {
    let opts = Opts::parse();
    let out_dir = opts
        .out
        .ok_or_else(|| std::env::current_dir())
        .expect("not a path");
    let mut work_items: Vec<(BufReader<File>, BufWriter<File>)> =
        Vec::with_capacity(opts.input.len());
    for sdf_src in opts.input {
        let sdf_src_name = sdf_src.file_stem().ok_or_else(|| anyhow!("not a file"))?;
        let mut dst_path = out_dir.clone();
        dst_path.push(sdf_src_name);
        dst_path.set_extension("bsdf");
        let src = BufReader::new(File::open(sdf_src)?);
        let dst = BufWriter::new(
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(dst_path)?,
        );
        work_items.push((src, dst));
    }
    work_items
        .into_par_iter()
        .try_for_each(|(src, dst)| sdf_to_bsdf(src, dst))?;
    Ok(())
}

/// Converts the contents of a source SDF file to binary and write to
/// DST.
fn sdf_to_bsdf<S, D>(src: S, mut dst: D) -> Result<()>
where
    S: BufRead,
    D: Write,
{
    let mut min = i16::MAX;
    let mut max = i16::MIN;
    let mut vec = vec![0; 1200 * 1200];
    let mut x = 0;
    let mut y = 0;
    for line in src.lines().skip(4) {
        let elev = i16::from_str(&line?)?;
        if elev > max {
            max = elev;
        }
        if elev < min {
            min = elev;
        }
        vec[(y * 1200) + x] = elev;
        y += 1;
        if y == 1200 {
            y = 0;
            x += 1;
        }
        //dst.write_i16::<LE>(elev)?;
    }
    for elev in vec.iter() {
        dst.write_i16::<LE>(*elev)?;
    }
    dst.write_i16::<LE>(min)?;
    dst.write_i16::<LE>(max)?;
    dst.flush()?;
    Ok(())
}
