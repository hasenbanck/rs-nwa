mod nwa;

extern crate byteorder;

#[macro_use]
extern crate failure;
extern crate structopt;

use nwa::NWAFile;
use std::fs::File;
use std::path::PathBuf;
use structopt::StructOpt;

enum FileType {
    Nwa,
    Nwk,
    Ovk,
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
    println!("reading file: {}", input_file_name);

    let input_file_type = get_filetype(&input_file_name)?;
    let output_file_stem = input.file_stem().unwrap().to_str().unwrap();

    let mut file = File::open(input)?;
    let mut nwa_file = NWAFile::new(&mut file)?;
    nwa_file.save(String::from("test.wav"))?;

    Ok(())
}

fn get_filetype(filename: &String) -> Result<FileType, failure::Error> {
    Ok(
        if filename.to_lowercase().contains("nwa") {
            FileType::Nwa
        } else if filename.to_lowercase().contains("nwk") {
            FileType::Nwk
        } else if filename.to_lowercase().contains("ovk") {
            FileType::Ovk
        } else {
            bail!("unknown filetype");
        }
    )
}