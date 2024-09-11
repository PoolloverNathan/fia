#![allow(warnings)]

mod moon;

use std::collections::HashMap;
use std::fs::{File, create_dir_all, canonicalize, read_to_string, write};
use std::io::{self, stdout, IsTerminal};
use std::path::{Path, PathBuf};
use std::process::exit;
use std::str::FromStr;
use clap::{Parser, Subcommand};
use moon::Moon;
use resolve_path::PathResolveExt as _;
use serde::{Serialize, Deserialize};
use url::Url;

#[derive(Debug, Parser)]
enum Action {
    #[command(about = "Parses an avatar file and outputs data to stdout")]
    Parse {
        #[arg()]
        moon: PathBuf,
    },
    #[command(about = "Uploads an avatar (or compiled moon, in the full version) to the Figura backend.")]
    Push {
        #[arg(required = true)]
        avatar: Option<PathBuf>,
        #[arg(short, long)]
        moon: bool,
    },
    #[command(about = "Downloads an avatar from the cloud by UUID or player name.")]
    Pull {
        #[arg()]
        target: String,
        #[arg(short, long)]
        out: Option<PathBuf>,
        #[arg(short = 'C', long, conflicts_with = "out")]
        cem: Option<String>,
        #[arg(short = 'r', long, requires = "cem")]
        pack_root: Option<PathBuf>,
        #[cfg(feature = "unpack")]
        #[arg(short, long, conflicts_with = "cem")]
        unpack: bool,
    },
    #[command(about = "Creates an avatar file from a directory.")]
    Pack {
        #[arg()]
        dir: PathBuf,
        #[arg()]
        out: PathBuf,
    },
    #[cfg(feature = "unpack")]
    #[command(about = "Unpacks the contents of an avatar file.")]
    Unpack {
        #[arg()]
        file: PathBuf,
        #[arg(short, long, default_value = ".")]
        out: PathBuf,
    },
    #[cfg(feature = "backend")]
    #[command(about = "Runs a backend.")]
    Backend {
        
    },
}

fn getMoon(path: &Path) -> io::Result<Moon> {
    let mut file = File::open(path)?;
    let (data, _): (Moon, _) = quartz_nbt::serde::deserialize_from(&mut file, quartz_nbt::io::Flavor::GzCompressed).unwrap_or_else(|m| panic!("moon data corrputed due to {m:?}"));
    Ok(data)
}

fn main() -> io::Result<()> {
    match Action::parse() {
        Action::Parse { moon } => {
            let data = getMoon(&moon)?;
            println!("{data:?}");
        }
        Action::Push { .. } => todo!(),
        Action::Pull { .. } => todo!(),
        Action::Pack { .. } => todo!(),
        #[cfg(feature = "unpack")]
        Action::Unpack { file, out } => {
            let Moon { textures: moon::Textures { src, .. }, scripts, animations, models, metadata } = getMoon(&file)?;
            let mut files = HashMap::<PathBuf, &[u8]>::new();
            for (path, data) in &src {
                let mut path = out.join(Path::new(&(path.to_owned() + ".png")));
                files.insert(path, &data.as_ref());
            }
            for (path, data) in &scripts {
                let mut path = out.join(Path::new(&(path.to_owned() + ".lua")));
                files.insert(path, &data.as_ref());
            }
            if models.chld.len() > 0 {
                eprintln!("warning: extracting models not supported yet")
            }
            let mut dirs: Vec<_> = files.keys().filter_map(|p| p.parent().map(PathBuf::from)).collect();
            dirs.sort();
            dirs.dedup();
            let mut written = 0;
            let mut fails = std::num::Saturating(0i8);
            for dir in dirs {
                if let Err(e) = create_dir_all(&dir) {
                    fails += 1;
                    eprintln!("failed to mkdir {}: {e}", dir.display());
                    files.retain(|lost, _| {
                        if lost.starts_with(&dir) {
                            eprintln!("├╴lost file: {}", lost.display());
                            false
                        } else {
                            true
                        }
                    });
                    eprintln!("\x1b[A└"); // no need to check, as we can't create empty directories
                }
            }
            for (file, data) in files {
                if let Err(e) = write(&file, data) {
                    fails += 1;
                    eprintln!("failed to write {}: {e}", file.display());
                } else {
                    written += 1;
                }
            }
            eprintln!("wrote {written} files");
            std::process::exit(fails.0.into())
        }
        #[cfg(feature = "backend")]
        Action::Backend { .. } => todo!(),
    }
    Ok(())
}
