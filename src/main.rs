#![allow(warnings)]

mod moon;

use std::collections::HashMap;
use std::fs::{File, canonicalize, read_to_string};
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
        #[arg()]
        avatar: PathBuf,
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
        #[arg(short, long)]
        out: PathBuf,
    },
    #[cfg(feature = "backend")]
    #[command(about = "Runs a backend.")]
    Backend {
        
    },
}

fn getMoon(path: &Path) -> io::Result<Moon> {
    let mut file = File::open(path)?;
    let (data, _): (Moon, _) = quartz_nbt::serde::deserialize_from(&mut file, quartz_nbt::io::Flavor::GzCompressed).unwrap_or_else(|m| panic!["moon data corrputed due to {m:?}"]);
    Ok(data)
}

fn main() -> io::Result<()> {
    match Action::parse() {
        Action::Parse { moon } => {
            let data = getMoon(&moon);
            println!("{data:?}");
        }
        Action::Push { .. } => todo!(),
        Action::Pull { .. } => todo!(),
        Action::Pack { .. } => todo!(),
        #[cfg(feature = "unpack")]
        Action::Unpack { .. } => todo!(),
        #[cfg(feature = "backend")]
        Action::Backend { .. } => todo!(),
    }
    Ok(())
}
