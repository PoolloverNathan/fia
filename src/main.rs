#![allow(warnings)]

mod bbmodel;
mod moon;

use std::collections::HashMap;
use std::fs::{File, create_dir_all, canonicalize, read_to_string, write};
use std::io::{self, stdout, IsTerminal};
use std::path::{Path, PathBuf};
use std::process::exit;
use std::str::FromStr;
use bbmodel::BBModel;
use clap::{Parser, Subcommand};
use moon::Moon;
use quartz_nbt::serde::Array;
use resolve_path::PathResolveExt as _;
use serde::{Serialize, Deserialize};
use url::Url;

#[derive(Debug, Parser)]
enum Action {
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
    #[command(about = "Prints information about an avatar file.")]
    Show {
        #[arg()]
        file: PathBuf,
        #[arg(short, long)]
        parse: bool,
        #[arg(short, long, conflicts_with = "parse")]
        verbose: bool,
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
    #[command(about = "Rewrites, recompresses, and optionally modifies an avatar file.")]
    Repack {
        #[arg()]
        file: PathBuf,
        #[arg(short, long)]
        out: Option<PathBuf>,
        #[arg(short = 'z', long)]
        compress: Option<Option<u32>>,
        #[arg(short = 'Z', long, conflicts_with = "compress")]
        no_compress: bool,
        #[arg(short = 'w', long, conflicts_with = "if_smaller")]
        if_smaller: bool,
        #[arg(short = 'A', long, conflicts_with = "if_smaller")]
        add_author: Vec<String>,
        #[arg(short = 's', long, conflicts_with = "if_smaller")]
        add_script: Vec<String>,
        #[arg(short = 't', long, conflicts_with = "if_smaller")]
        add_texture: Vec<String>,
        #[arg(short = 'e', long, alias = "edit", conflicts_with = "if_smaller")]
        edit_script: Vec<String>,
        #[arg(short = 'r', long, conflicts_with = "if_smaller")]
        remove_script: Vec<String>,
        #[arg(short = 'R', long, conflicts_with = "if_smaller")]
        remove_texture: Vec<String>,
    },
    #[cfg(feature = "backend")]
    #[command(about = "Runs a Figura-compatible backend.")]
    Backend {
        
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
        Action::Push { .. } => todo!(),
        Action::Pull { .. } => todo!(),
        Action::Show { file, verbose, parse } => {
            let (moon, tag_name) = get_moon_with_name(&file)?;
            if parse {
                println!("{moon:?}");
            } else {
                println!("\x1b[1;4m{}\x1b[21;22;24m", moon.metadata.name);
                println!("{}", moon.metadata.description);
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
                        println!("• \x1b[1m{} texture{}", moon.textures.src.len(), if moon.textures.src.len() == 1 { "s" } else { "" });
                    }
                }
                if !moon.scripts.is_empty() {
                    if verbose {
                        println!("");
                        println!("\x1b[1;4mScripts\x1b[21;22;24m");
                        for (name, data) in moon.scripts {
                            let data = Array::into_inner(data);
                            println!("• \x1b[1m{name}\x1b[21;22;24m {}b", data.len());
                        }
                    } else {
                        println!("• \x1b[1m{} scripts{}", moon.scripts.len(), if moon.scripts.len() == 1 { "s" } else { "" });
                    }
                }
            }
        }
        Action::Pack { .. } => todo!(),
        #[cfg(feature = "unpack")]
        Action::Unpack { file, out } => {
            let Moon { textures: moon::Textures { src, .. }, scripts, animations, models, metadata } = get_moon(&file)?;
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
        Action::Repack { file, out, add_author, compress, no_compress, if_smaller, add_script, add_texture, edit_script, remove_script, remove_texture } => {
            let (mut moon, name) = get_moon_with_name(&file)?;
            if add_author.len() > 0 {
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
                vec.extend(add_author);
                drop(vec);
            }
            if add_script.len() > 0 {
                todo!("--add-script");
            }
            if add_texture.len() > 0 {
                todo!("--add-texture");
            }
            if edit_script.len() > 0 {
                todo!("--edit-script");
            }
            if remove_script.len() > 0 {
                todo!("--remove-script");
            }
            if remove_texture.len() > 0 {
                todo!("--remove-texture");
            }
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
    }
    Ok(())
}
