use anyhow::{anyhow, bail, Context, Result};
use clap::{Parser, Subcommand};
use clap_cargo;
use clap_verbosity;
use exec;
use std::fs;
use std::fs::File;
use std::io;
use std::io::{Seek, SeekFrom, Write};
use std::mem;
use std::path::PathBuf;
use std::process::ExitCode;
use thiserror::Error;

mod encode;
use crate::encode::hinteger_encode;

#[derive(Clone, Debug)]
pub struct BlobSpec {
    /// Blob ID value
    id: usize,

    /// Blob alignment requirement, as a power of 2. default is byte-alignment
    align: usize,

    /// Blob contents filename
    blob: PathBuf,
}

// TODO: take app.elf on command line and a symbol like __utility_block_addr
// and search the file to get the associated address.

#[derive(Parser)]
#[command(version, about, long_about = None, styles = clap_cargo::style::CLAP_STYLING)]
pub struct Cli {
    #[command(subcommand)]
    pub thing_to_do: CmdUtilitySection,
}

#[derive(Subcommand)]
pub enum CmdUtilitySection {
    UtilitySection {

        /// Output file; extension determines format
        #[arg(short, long, default_value = "./utility.ihex")]
        outfile: PathBuf,

        /// Address at which to load the generated utility section block. I find
        /// this with ...-objdump -t ...app.elf, and search for the utility section
        /// symbol (__utility_block_addr in my use). This is a hex string in C-literal
        /// format, 0x12345678, passed straight through to objcopy.
        #[arg(long, short)]
        load_address: String,

        /// Upper limit on string item length. Safety says, make this match
        /// parameter N in utility_section::decode; but it's just a check on actual
        /// string lengths here.
        #[arg(long, default_value = "64" )]
        maximum_string_length: usize,

        /// String to include in the utility-section encoding
        #[arg(short, long)]
        string: Vec<String>,

        // Blobs to include in the encoding
        #[arg(short, long, value_parser = parse_blob_spec, value_name = "[ID,]FILENAME[,ALIGN]")]
        blob: Vec<BlobSpec>,

        // Cargo boilerplate
        #[command(flatten)]
        manifest: clap_cargo::Manifest,
        #[command(flatten)]
        workspace: clap_cargo::Workspace,
        #[command(flatten)]
        features: clap_cargo::Features,

        // Verbosity boilerplate
        #[command(flatten)]
        verbosity: clap_verbosity::Verbosity,
    },
}

#[derive(Error, Debug)]
pub enum BlobParseError {
    #[error("problem parsing integer")]
    ParseIntError(#[from] std::num::ParseIntError),

    #[error("file {0} not found")]
    FileNotFound(PathBuf),

    #[error("error finding file {0}: {1}")]
    FailedFindingFile(PathBuf, std::io::Error),
}

/// Parse arg "--blob 2,filename,3" -- ID, filename, alignment -- where
/// both numeric values are optional (default to zero)
fn parse_blob_spec(arg: &str) -> Result<BlobSpec> {
    let mut blob_id: usize = 0;
    let mut blob_align: usize = 0;
    let blob_filename: String;
    if let Some(comma) = arg.chars().position(|c| c == ',') {
        if let Ok(id) = arg[..comma].parse::<usize>() {
            blob_id = id;
            if let Some(c2) = arg[comma+1..].chars().position(|c| c == ',') {
                let comma2 = comma+1+c2;
                blob_filename = arg[comma+1..comma2].into();
                blob_align = arg[comma2+1..].parse::<usize>()?;
            } else {
                blob_filename = arg[comma+1..].into();
            }
        } else {
            blob_filename = arg[..comma].into();
            blob_align = arg[comma+1..].parse::<usize>()?;
        }
    } else {
        blob_filename = arg.into();
    }

    let path: PathBuf = blob_filename.into();
    match fs::exists(&path).with_context(|| format!("checking file {}", path.display())) {
        Ok(true) => {
            Ok(BlobSpec {
                id: blob_id,
                align: blob_align,
                blob: path,
            })
        },
        Ok(false) => {
            Err(anyhow!("file not found {}", path.display()))
        },
        Err(e) => {
            Err(anyhow!("fs error {}", e))
        },
    }
}

fn main() -> Result<ExitCode> {
    let args = Cli::parse();
    let CmdUtilitySection::UtilitySection {
        outfile,
        string,
        blob,
        load_address,
        maximum_string_length: _,
        manifest: _,
        workspace: _,
        features: _,
        verbosity: _,
    } = args.thing_to_do;

    let mut tmp_path = outfile.clone();
    tmp_path.add_extension("tmp");
    let mut tempfile = File::create(&tmp_path).with_context(|| { format!("creating file {}", tmp_path.display())})?;

    for s in string {
        tempfile.write(&hinteger_encode(s.len()))?;
        tempfile.write(s.as_bytes())?;
    }
    tempfile.write_all(&hinteger_encode(0))?;  // end of strings

    for b in blob {
        let mut blobfile = File::open(&b.blob).with_context(|| { format!("opening blobfile {}", b.blob.display())})?;
        tempfile.write(&hinteger_encode(blobfile.seek(SeekFrom::End(0))?.try_into().unwrap()))?;
        tempfile.write(&hinteger_encode(b.id))?;
        tempfile.write(&hinteger_encode(b.align))?;
        let here = tempfile.seek(SeekFrom::Current(0))?;
        let a = 1 << b.align;
        let mut there = here & !(a - 1);
        if there < here {
            there += a;
        }
        println!("here: {} there: {} a: {}", here, there, a);
        for _i in 0..(there - here) {
            tempfile.write(&[b'>'])?;
        }
        blobfile.seek(SeekFrom::Start(0))?;
        io::copy(&mut blobfile, &mut tempfile)?;
    }
    tempfile.write_all(&hinteger_encode(0))?;  // end of blobs
    mem::drop(tempfile);

    let _ = exec::Command::new("arm-none-eabi-objcopy")
        .arg("--input")
        .arg("binary")
        .arg("--output")
        .arg("ihex")
        .arg(tmp_path.clone())
        .arg(outfile.clone())
        .arg("--change-section-lma")
        .arg(format!("*={}", load_address))
        .exec();
    bail!("cargo objcopy failed");
}
