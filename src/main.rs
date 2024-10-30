#![allow(warnings)]
#![deny(missing_docs)]

//! Various CLI utilities for Figura.

mod bbmodel;
mod moon;

use std::collections::HashMap;
use std::fs::{File, create_dir_all, canonicalize, read_to_string, write};
use std::io::{self, stdout, IsTerminal};
use std::path::{Path, PathBuf};
use std::process::exit;
use std::str::FromStr;
use base64::{Engine as _, prelude::BASE64_STANDARD};
use bbmodel::BBModel;
use clap::{Args, ArgGroup, Parser, Subcommand};
use moon::Moon;
use quartz_nbt::serde::Array;
use resolve_path::PathResolveExt as _;
use serde::{Serialize, Deserialize};
use url::Url;

/// Set of modifications to perform to avatar data.
#[derive(Args, Clone, Debug, Default, PartialEq, Eq)]
#[command(next_help_heading = "Editing Options")]
pub struct MoonModifications {
    /// Add an avatar author (authors cannot be removed for obvious reasons).
    #[arg(short = 'p', long, value_name = "AUTHOR")]
    pub add_author: Vec<String>,
    /// Add or replace a script.
    #[arg(short = 'i', long, value_name = "\x08[NAME=]<PATH>\x1b[C\x1b")]
    pub add_script: Vec<String>,
    /// Add or replace a texture.
    #[arg(short = 'k', long, value_name = "\x08[NAME=]<PATH>\x1b[C\x1b")]
    pub add_texture: Vec<String>,
    /// Interactively edit a script (leaving a copy in the current working directory).
    #[arg(short = 'e', long, alias = "edit", value_name = "NAME")]
    pub edit_script: Vec<String>,
    /// Delete a script.
    #[arg(short = 'r', long, value_name = "NAME")]
    pub remove_script: Vec<String>,
    /// Delete a texture.
    #[arg(short = 's', long, value_name = "NAME")]
    pub remove_texture: Vec<String>,
}

impl MoonModifications {
    fn apply(self, moon: &mut Moon) {
        if self.add_author.len() > 0 {
            let authors: &mut moon::Authors = &mut moon.metadata.authors;
            // normalize
            let vec: &mut Vec<String> = match authors {
                moon::Authors::Authors(ref mut vec) => vec,
                moon::Authors::Author(_) => {
                    let mut new_authors = moon::Authors::Authors(vec![]);
                    // ah, the ol' authorship switcharoo
                    let moon::Authors::Author(a) = std::mem::replace(authors, moon::Authors::Authors(vec![])) else { unreachable!() };
                    let moon::Authors::Authors(ref mut vec) = authors else { unreachable!() };
                    vec.push(a);
                    vec
                }
            };
            vec.extend(self.add_author);
            drop(vec);
        }
    }
}

/// Top-level parsing node
#[derive(Clone, Debug, Parser)]
pub enum Action {
    #[cfg_attr(feature = "unpack", doc = "Upload an avatar or compiled moon to the Figura backend.")]
    #[cfg_attr(not(feature = "unpack"), doc = "Upload an avatar directory to the Figura backend.")]
    Push {
        /// Path to the avatar to pack and upload.
        #[arg(required = true)]
        avatar: Option<PathBuf>,
        /// Treat the avatar path as a moon instead of packing it.
        #[cfg(feature = "unpack")]
        #[arg(short, long)]
        moon: bool,
        #[command(flatten)]
        #[allow(missing_docs)]
        modify: MoonModifications,
    },
    /// Download an avatar from the cloud by UUID or player name.
    #[cfg(feature = "pull")]
    Pull {
        /// String or UUID (or avatar ID with -A) to download.
        #[arg(required = true)]
        target: Option<String>,
        /// Treat the target as an avatar ID instead.
        #[arg(short = 'A', long, conflicts_with = "target")]
        avatar_id: Option<String>,
        #[cfg_attr(not(feature = "unpack"), doc = "Path to write the avatar data file to.")]
        #[cfg_attr(feature = "unpack", doc = "Path to write the avatar data file (or extracted contents with --unpack) to.")]
        #[arg(short, long)]
        out: Option<PathBuf>,
        /// Automatically determine the out path for CEM based on an entity ID.
        #[arg(short = 'C', long, conflicts_with = "out")]
        cem: Option<String>,
        /// Path to the root directory of the resource pack when using --cem.
        #[arg(short = 'r', long, requires = "cem")]
        pack_root: Option<PathBuf>,
        /// Extract the downloaded avatar's contents immediately.
        #[cfg(feature = "unpack")]
        #[arg(short, long, conflicts_with = "cem")]
        unpack: bool,
        #[command(flatten)]
        #[allow(missing_docs)]
        modify: MoonModifications,
    },
    /// Print information about an avatar file.
    Show {
        /// Path to the avatar file to show.
        #[arg()]
        file: PathBuf,
        /// Print the internal representation of the avatar file.
        #[arg(short, long)]
        parse: bool,
        /// Show more information, such as filenames.
        #[arg(short, long, conflicts_with = "parse")]
        verbose: bool,
        /// Output script content after each script.
        #[arg(short, long, requires = "verbose")]
        sources: bool,
        #[command(flatten)]
        #[allow(missing_docs)]
        modify: MoonModifications,
    },
    /// Create an avatar file from a directory.
    Pack {
        /// Path to avatar data to pack. Defaults to current directory.
        #[arg(default_value = ".")]
        dir: PathBuf,
        /// Where to write the resulting avatar data. Defaults to avatar.nbt.
        #[arg(default_value = "avatar.nbt")]
        out: PathBuf,
        #[command(flatten)]
        #[allow(missing_docs)]
        modify: MoonModifications,
    },
    #[cfg(feature = "unpack")]
    /// Unpack the contents of an avatar file.
    Unpack {
        /// Path to the avatar data to unpack.
        #[arg()]
        file: PathBuf,
        /// Where to unpack the data to. Defaults to current directory, which may be explosive!
        #[arg(short, long, default_value = ".")]
        out: PathBuf,
        #[command(flatten)]
        #[allow(missing_docs)]
        modify: MoonModifications,
    },
    /// Rewrite, recompress, and optionally modify an avatar file.
    Repack {
        /// File to read avatar data from.
        #[arg()]
        file: PathBuf,
        /// Output path for avatar data. Overwrites the input file by default.
        #[arg(short, long)]
        out: Option<PathBuf>,
        /// Set the compression level to the given value or maximum.
        #[arg(short = 'z', long)]
        compress: Option<Option<u32>>,
        /// Do not compress the avatar data.
        #[arg(short = 'l', long, conflicts_with = "compress")]
        no_compress: bool,
        /// Only [over]write the avatar data if it was made smaller.
        #[arg(short = 'w', long)]
        if_smaller: bool,
        #[command(flatten)]
        #[allow(missing_docs)]
        modify: MoonModifications,
    },
    #[cfg(feature = "backend")]
    /// Run a Figura-compatible backend.
    Backend {
    },
    /// 🦭
    #[command(hide = true, group = ArgGroup::new("image").multiple(false))]
    #[allow(missing_docs)]
    Fok {
        #[arg(short, long, group = "image")]
        stock: bool,
        #[arg(short = '1', long, group = "image")]
        first: bool,
        #[arg(short = '2', long, group = "image")]
        second: bool,
        #[arg(short = '3', long, group = "image")]
        third: bool,
    },
}

fn get_moon_with_name(path: &Path) -> io::Result<(Moon, String)> {
    let mut file = File::open(path)?;
    let data: (Moon, _) = quartz_nbt::serde::deserialize_from(&mut file, quartz_nbt::io::Flavor::GzCompressed).unwrap_or_else(|m| panic!("moon data corrputed due to {m:?}"));
    Ok(data)
}
fn get_moon(path: &Path) -> io::Result<Moon> {
    get_moon_with_name(path).map(|d| d.0)
}

fn main() -> io::Result<()> {
    match Action::parse() {
        Action::Push { avatar, modify, #[cfg(feature = "unpack")] moon } => {
            todo!()
        }
        #[cfg(feature = "pull")]
        Action::Pull { target, avatar_id, out, cem, pack_root, modify, #[cfg(feature = "unpack")] unpack } => {
            todo!()
        }
        Action::Show { file, verbose, parse, sources, modify } => {
            let (mut moon, tag_name) = get_moon_with_name(&file)?;
            modify.apply(&mut moon);
            if parse {
                println!("{moon:?}");
            } else {
                println!("\x1b[1;4m{}\x1b[21;22;24m", moon.metadata.name);
                if moon.metadata.description != "" {
                    let mut desc: &str = (&*moon.metadata.description).into();
                    if !verbose {
                        if let Some(size) = desc.find('\n') {
                            desc = &desc[0..size];
                            // Safety:
                            // * Decreasing the length of a string is safe
                            // * `str::find` always returns a value less than length
                            // * `str::find` is codepoint-aligned, hopefully
                            // Rationale: Avoids an allocation
                            // unsafe {
                            //     let ptr2: &mut (*const (), usize) = std::mem::transmute(&mut desc);
                            //     debug_assert!(size <= ptr2.1);
                            //     ptr2.1 = size;
                            // }
                        }
                    }
                }
                // println!("\x1b[1mAuthors:\x1b[21;22m {}");
                if !moon.textures.src.is_empty() {
                    if verbose {
                        println!("");
                        println!("\x1b[1;4mTextures\x1b[21;22;24m");
                        for (name, data) in moon.textures.src {
                            let data = Array::into_inner(data);
                            println!("• \x1b[1m{name}\x1b[21;22;24m {}B", data.len());
                        }
                    } else {
                        println!("• \x1b[1m{} texture{}", moon.textures.src.len(), if moon.textures.src.len() == 1 { "" } else { "s" });
                    }
                }
                if !moon.scripts.is_empty() {
                    if verbose {
                        println!("");
                        println!("\x1b[1;4mScripts\x1b[21;22;24m");
                        for (name, data) in moon.scripts {
                            let data = Array::into_inner(data);
                            println!("• \x1b[1m{name}\x1b[21;22;24m {}b", data.len());
                            if sources {
                                println!("{}", String::from_utf8_lossy(&data));
                            }
                        }
                    } else {
                        println!("• \x1b[1m{} script{}", moon.scripts.len(), if moon.scripts.len() == 1 { "" } else { "s" });
                    }
                }
            }
        }
        Action::Pack { .. } => todo!(),
        #[cfg(feature = "unpack")]
        Action::Unpack { file, out, modify } => {
            let mut moon = get_moon(&file)?;
            modify.apply(&mut moon);
            let Moon { textures: moon::Textures { src, .. }, scripts, animations, models, metadata, resources } = moon;
            let mut files = HashMap::<PathBuf, &[u8]>::new();
            for (path, data) in &src {
                let mut path = out.join(Path::new(&(path.replace('.', "/") + ".png")));
                files.insert(path, &data.as_ref());
            }
            for (path, data) in &scripts {
                let mut path = out.join(Path::new(&(path.replace('.', "/") + ".lua")));
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
        Action::Repack { file, out, compress, no_compress, if_smaller, modify } => {
            let (mut moon, name) = get_moon_with_name(&file)?;
            if (modify != MoonModifications::default() && if_smaller) {
                panic!("cannot use modification flags with -w")
            }
            modify.apply(&mut moon);
            use quartz_nbt::serde as qs;
            use flate2::Compression;
            let compression = if no_compress {
                Compression::none()
            } else {
                match compress {
                    Some(Some(n)) => Compression::new(n),
                    Some(None)    => Compression::best(),
                    None          => Compression::default(),
                }
            };
            let flavor = quartz_nbt::io::Flavor::GzCompressedWith(compression);
            if if_smaller {
                let data = qs::serialize(&moon, Some(&name), flavor);
            } else {
                let mut file = File::create(out.as_deref().unwrap_or(&file))?;
                qs::serialize_into(&mut file, &moon, Some(&name), flavor);
            }
        }
        #[cfg(feature = "backend")]
        Action::Backend { .. } => todo!(),
        Action::Fok { stock, first, second, third } => {
            let mut path = Vec::<u8>::from(env!("FOKDIR"));
            path.extend_from_slice(b"/"); // needed to concatenate paths
            path.extend_from_slice(match (stock, first, second, third) {
                (false, false, false, false) => b"seal.png"  as &[u8],
                (true,  false, false, false) => b"fok.png"   as &[u8],
                (false, true,  false, false) => b"seal1.png" as &[u8],
                (false, false, true,  false) => b"seal2.png" as &[u8],
                (false, false, false, true)  => b"seal3.png" as &[u8],
                _ => unreachable!(),
            });
            println!("\x1b_Gf=100,t=f,a=T,r=10;{}\x1b\\", BASE64_STANDARD.encode(&path));
        },
    }
    Ok(())
}
