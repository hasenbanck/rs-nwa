extern crate byteorder;
#[macro_use]
extern crate failure;
extern crate nwa;
extern crate rayon;
extern crate structopt;

use byteorder::{LittleEndian, ReadBytesExt};
use nwa::NWAFile;
use rayon::prelude::*;
use std::fs::File;
use std::io::prelude::*;
use std::io::{copy, Read, SeekFrom};
use std::path::{Path, PathBuf};
use structopt::StructOpt;

enum FileType {
    Nwa,
    Nwk,
    Ovk,
}

struct IndexEntry {
    size: i32,
    offset: i32,
    count: i32,
}

#[derive(StructOpt, Debug)]
#[structopt(name = "nwatowav", about = "Converts NWA/NWK/OVK files to WAV/OGG.")]
struct Opt {
    /// Input file
    #[structopt(parse(from_os_str))]
    input: PathBuf,
}

fn main() -> Result<(), failure::Error> {
    let opt = Opt::from_args();

    let input = opt.input.as_path();
    let input_file_name = String::from(input.to_str().unwrap());
    println!("converting file: {}", input_file_name);

    let input_file_type = get_filetype(&input_file_name)?;
    let output_file_stem = String::from(input.file_stem().unwrap().to_str().unwrap());

    match input_file_type {
        FileType::Nwa => {
            handle_nwa(&input, output_file_stem)?;
        }
        FileType::Nwk => {
            handle_nwk(&input, output_file_stem)?;
        }
        FileType::Ovk => {
            handle_ovk(&input, output_file_stem)?;
        }
    }

    Ok(())
}

fn handle_nwa(path: &Path, file_stem: String) -> Result<(), failure::Error> {
    let mut file = File::open(path)?;
    let mut nwa_file = NWAFile::new(&mut file)?;
    nwa_file.save(format!("{}.{}", file_stem, "wav"))?;

    Ok(())
}

fn handle_nwk(path: &Path, file_stem: String) -> Result<(), failure::Error> {
    let index = read_index(path, 12)?;

    index.into_par_iter().for_each(|i| {
        decode_and_save_file(path, i, &file_stem).unwrap();
    });

    Ok(())
}

fn handle_ovk(path: &Path, file_stem: String) -> Result<(), failure::Error> {
    let index = read_index(path, 16)?;

    index.into_par_iter().for_each(|i| {
        save_file(path, i, &file_stem).unwrap();
    });

    Ok(())
}

#[rustfmt::skip]
fn decode_and_save_file(path: &Path, i: IndexEntry, file_stem: &String) -> Result<(), failure::Error> {
    let mut file = File::open(path)?;
    file.seek(SeekFrom::Start(i.offset as u64))?;
    let mut nwa = NWAFile::new(&mut file)?;
    nwa.save(format!("{}-{}.{}", file_stem, i.count, "nwk"))?;

    Ok(())
}

fn save_file(path: &Path, i: IndexEntry, file_stem: &String) -> Result<(), failure::Error> {
    let mut file = File::open(path)?;
    file.seek(SeekFrom::Start(i.offset as u64))?;
    let mut buf = vec![0; i.size as usize];
    file.read_exact(&mut buf)?;

    let mut out_file = File::create(format!("{}-{}.{}", file_stem, i.count, "nwk"))?;
    copy(&mut buf.as_slice(), &mut out_file)?;

    Ok(())
}

fn read_index(path: &Path, head_block_size: usize) -> Result<Vec<IndexEntry>, failure::Error> {
    let mut file = File::open(path)?;
    let indexcount = file.read_i32::<LittleEndian>()?;
    if indexcount <= 0 {
        bail!("invalid indexcount found: {}", indexcount);
    }
    let mut index: Vec<IndexEntry> = Vec::with_capacity(indexcount as usize);

    for _i in 0..indexcount {
        let mut buf = vec![0; head_block_size];
        file.read_exact(&mut buf)?;

        let entry = IndexEntry {
            size: buf.as_slice().read_i32::<LittleEndian>()?,
            offset: buf.as_slice().read_i32::<LittleEndian>()?,
            count: buf.as_slice().read_i32::<LittleEndian>()?,
        };
        if entry.offset <= 0 || entry.size <= 0 {
            bail!(
                "invalid table entry. offset: {}, size: {}",
                entry.offset,
                entry.size
            );
        }

        index.push(entry)
    }
    Ok(index)
}

fn get_filetype(filename: &String) -> Result<FileType, failure::Error> {
    Ok(if filename.to_lowercase().contains("nwa") {
        FileType::Nwa
    } else if filename.to_lowercase().contains("nwk") {
        FileType::Nwk
    } else if filename.to_lowercase().contains("ovk") {
        FileType::Ovk
    } else {
        bail!("unknown filetype");
    })
}
