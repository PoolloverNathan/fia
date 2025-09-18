#![allow(warnings)]
#![deny(missing_docs)]
#![feature(never_type)]

//! Various CLI utilities for Figura.

mod bbmodel;
pub mod moon;

use base64::{prelude::BASE64_STANDARD, Engine as _};
use bbmodel::BBModel;
use clap::{ArgGroup, Args, Parser, Subcommand};
use moon::Moon;
use quartz_nbt::{io::NbtIoError, serde::Array};
use resolve_path::PathResolveExt as _;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Display;
use std::fs::{canonicalize, create_dir_all, read_to_string, write, File};
use std::io::{self, stdout, IsTerminal, Read, Write};
use std::path::{Path, PathBuf};
use std::process::exit;
use std::str::FromStr;
use thiserror::Error;
use url::Url;

#[derive(Debug, Error)]
enum EqualParseError<K: Display, V: Display> {
  #[error("string did not contain a '='")]
  EqualSignRequired,
  #[error("could not parse key: {0}")]
  InvalidKey(K),
  #[error("could not parse value: {0}")]
  InvalidValue(V),
}
#[derive(Debug, Error)]
enum OptEqualParseError<K: Display, V: Display> {
  #[error("could not parse lone value: {0}")]
  InvalidLoneValue(V),
  #[error("could not parse key in pair: {0}")]
  InvalidPairKey(K),
  #[error("could not parse value in pair: {0}")]
  InvalidPairValue(V),
}
fn equal<K: FromStr, V: FromStr>(pair: &str) -> Result<(K, V), EqualParseError<K::Err, V::Err>>
where
  K::Err: Display,
  V::Err: Display,
{
  if let Some(n) = pair.find('=') {
    match pair[0..n].parse() {
      Ok(k) => match pair[n + 1..].parse() {
        Ok(v) => Ok((k, v)),
        Err(e) => Err(EqualParseError::InvalidValue(e)),
      },
      Err(e) => Err(EqualParseError::InvalidKey(e)),
    }
  } else {
    Err(EqualParseError::EqualSignRequired)
  }
}
fn opt_equal<K: FromStr, V: FromStr>(
  pair: &str,
) -> Result<(Option<K>, V), OptEqualParseError<K::Err, V::Err>>
where
  K::Err: Display,
  V::Err: Display,
{
  if let Some(n) = pair.find('=') {
    match pair[0..n].parse() {
      Ok(k) => match pair[n + 1..].parse() {
        Ok(v) => Ok((Some(k), v)),
        Err(e) => Err(OptEqualParseError::InvalidPairValue(e)),
      },
      Err(e) => Err(OptEqualParseError::InvalidPairKey(e)),
    }
  } else {
    match pair.parse() {
      Ok(v) => Ok((None, v)),
      Err(e) => Err(OptEqualParseError::InvalidLoneValue(e)),
    }
  }
}

/// List of avatar fields to unpack or skip unpacking.
#[derive(Args, Clone, Debug, PartialEq, Eq)]
#[command(next_help_heading = "Unpack Filters")]
pub struct UnpackFilter {
  /// Whether to unpack textures.
  #[arg(short = 'T', long, default_value = "true")]
  pub textures: bool,
  /// Whether to unpack scripts.
  #[arg(short = 'S', long, default_value = "true")]
  pub scripts: bool,
  /// Whether to unpack models.
  #[arg(short = 'M', long, default_value = "true")]
  pub models: bool,
  /// Whether to unpack resources.
  #[arg(short = 'R', long, default_value = "true")]
  pub resources: bool,
  /// Whether to unpack avatar metadata (`avatar.json`).
  #[arg(short = 'F', long, default_value = "true")]
  pub manifest: bool,
}

/// Set of modifications to perform to avatar data.
#[derive(Args, Clone, Debug, PartialEq, Eq)]
#[command(next_help_heading = "Editing Options")]
pub struct MoonModifications {
  /// Add an avatar author (authors cannot be removed for obvious reasons).
  #[arg(short = 'p', long, value_name = "AUTHOR")]
  pub add_author: Vec<String>,
  /// Add or replace a script.
  #[arg(short = 'i', long, value_name = "\x08[NAME=]<PATH>\x1b[C\x1b", value_parser = equal::<String, PathBuf>)]
  pub add_script: Vec<(String, PathBuf)>,
  /// Add or replace a texture.
  #[arg(short = 'k', long, value_name = "\x08[NAME=]<PATH>\x1b[C\x1b", value_parser = equal::<String, PathBuf>)]
  pub add_texture: Vec<(String, PathBuf)>,
  /// Interactively edit a script (leaving a copy in the current working directory).
  #[arg(short = 'e', long, alias = "edit", value_name = "NAME")]
  pub edit_script: Vec<String>,
  /// Delete a script.
  #[arg(short = 'r', long, value_name = "NAME")]
  pub remove_script: Vec<String>,
  /// Delete a texture.
  #[arg(short = 's', long, value_name = "NAME")]
  pub remove_texture: Vec<String>,
  /// Reformat avatar scripts for readability.
  #[arg(short = 'u', long)]
  #[cfg_attr(not(feature = "stylua"), arg(hide = true))]
  pub format_scripts: bool,
}

impl MoonModifications {
  fn apply(self, moon: &mut Moon) -> io::Result<()> {
    let Self {
      add_author,
      add_script,
      add_texture,
      edit_script,
      remove_script,
      remove_texture,
      format_scripts,
    } = self;
    if add_author.len() > 0 {
      if moon.metadata.authors == "" || moon.metadata.authors == "?" {
        moon.metadata.authors = add_author.join("\n");
      } else {
        moon.metadata.authors += "\n";
        moon.metadata.authors += &add_author.join("\n");
      }
    }
    for name in remove_script {
      if let None = moon.scripts.remove(&name) {
        eprintln!("warning: removing nonexistent script {name}");
      }
    }
    for name in remove_texture {
      if let None = moon.textures.src.remove(&name) {
        eprintln!("warning: removing nonexistent texture {name}");
      }
    }
    for (name, path) in add_script {
      let mut buf = vec![];
      File::open(path)?.read_to_end(&mut buf);
      moon.scripts.insert(name, buf.into());
    }
    for (name, path) in add_texture {
      let mut buf = vec![];
      File::open(path)?.read_to_end(&mut buf);
      moon.textures.src.insert(name, buf.into());
    }
    if format_scripts {
      #[cfg(feature = "stylua")]
      for (name, script) in &mut moon.scripts {
        use stylua_lib::*;
        match std::str::from_utf8(script.as_mut()) {
          Ok(text) => {
            match format_code(
              text,
              Config {
                syntax: LuaVersion::Lua52,
                sort_requires: SortRequiresConfig { enabled: false },
                indent_type: IndentType::Spaces,
                indent_width: 2,
                ..Config::default()
              },
              None,
              OutputVerification::Full,
            ) {
              Ok(code) => *script = Array::from(code.into_bytes()),
              Err(e) => eprintln!("failed to format script {name}: {e}"),
            }
          }
          Err(e) => eprintln!("cannot decode script {name}: {e}"),
        }
      }
      #[cfg(not(feature = "stylua"))]
      {
        eprintln!(
          "warning: stylua is disabled; rebuild with `--features stylua` to format scripts"
        );
      }
    }
    Ok(())
  }
}

/// Top-level parsing node
#[derive(Clone, Debug, Parser)]
pub enum Action {
  #[cfg_attr(
    feature = "unpack",
    doc = "Upload an avatar or compiled moon to the Figura backend."
  )]
  #[cfg_attr(
    not(feature = "unpack"),
    doc = "Upload an avatar directory to the Figura backend."
  )]
  #[cfg(feature = "backend")]
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
  #[cfg(feature = "unpack")]
  #[cfg(feature = "backend")]
  Pull {
    /// String or UUID (or avatar ID with -A) to download.
    #[arg(required = true)]
    target: Option<String>,
    /// Treat the target as an avatar ID instead.
    #[arg(short = 'A', long, conflicts_with = "target")]
    avatar_id: Option<String>,
    #[cfg_attr(
      not(feature = "unpack"),
      doc = "Path to write the avatar data file to."
    )]
    #[cfg_attr(
      feature = "unpack",
      doc = "Path to write the avatar data file (or extracted contents with --unpack) to."
    )]
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
    #[arg(short = 'd', long)]
    parse: bool,
    /// Show more information, such as filenames.
    #[arg(short, long, conflicts_with = "parse")]
    verbose: bool,
    /// Output script content after each script.
    #[arg(short = 'w', long, requires = "verbose")]
    sources: bool,
    #[command(flatten)]
    #[allow(missing_docs)]
    modify: MoonModifications,
  },
  /// Parses a .bbmodel file. Mainly useful for internal testing.
  #[command(hide = true)]
  ParseBbmodel {
    /// Path to the Blockbench model to show.
    #[arg()]
    file: PathBuf,
  },
  /// Generates element JSON for a model.
  #[command(hide = true)]
  Element {
    /// Path to the avatar file to show.
    #[arg()]
    path: PathBuf,
    /// Instead of generating JSON, generate a modelpart hierarchy.
    #[arg(short = 'y', long)]
    hierarchy: bool,
    #[command(flatten)]
    #[allow(missing_docs)]
    modify: MoonModifications,
    /// The slice indexes leading to the element to convert.
    #[arg()]
    index: Vec<usize>,
  },
  /// Create an avatar file from a directory.
  #[command(hide = true)]
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
  ///
  /// Usage of this command is frowned upon. Many people's avatars are special to them, and
  /// unpacking them without permission is somewhat rude. Generally, people will be open to
  /// sharing code when asked, which is much less risky.
  ///
  /// When using this command, always follow the rules:
  /// * Don't unpack avatars if you're denied permission.
  /// * Never upload the generated files as an avatar.
  /// * Never use code or models from unpacked avatars without permission.
  ///
  /// Breaking the rules can lead to backend bans, or if you piss off the wrong person, copyright
  /// claims and legal costs.
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
    /// Which modelparts represent folders in the model hiearchy (as opposed to folders).
    #[arg(short = 't', long, value_name = "PATH")]
    folder: Vec<String>,
    /// Which files to unpack, if not all.
    #[arg()]
    paths: Option<Vec<String>>,
    /// Writes the raw model blob to a file.
    #[command(flatten)]
    filter: UnpackFilter,
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
  Backend {},
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

fn get_moon_with_name(mut file: impl Read) -> Result<(Moon, String), NbtIoError> {
  quartz_nbt::serde::deserialize_from(&mut file, quartz_nbt::io::Flavor::GzCompressed)
}
fn get_moon(mut file: impl Read) -> Result<Moon, NbtIoError> {
  get_moon_with_name(file).map(|d| d.0)
}

fn main() -> io::Result<()> {
  match Action::parse() {
    #[cfg(feature = "backend")]
    Action::Push {
      avatar,
      modify,
      #[cfg(feature = "unpack")]
      moon,
    } => {
      todo!()
    }
    #[cfg(feature = "unpack")]
    #[cfg(feature = "backend")]
    Action::Pull {
      target,
      avatar_id,
      out,
      cem,
      pack_root,
      modify,
      #[cfg(feature = "unpack")]
      unpack,
    } => {
      todo!()
    }
    Action::Show {
      file,
      verbose,
      parse,
      sources,
      modify,
    } => {
      let file = File::open(file)?;
      // FIXME: don't panic
      let (mut moon, tag_name) = get_moon_with_name(file).expect("loading moon failed");
      if let Err(e) = modify.apply(&mut moon) {
        eprintln!("Failed to apply modifications: {}", e);
        exit(1);
      }
      if parse {
        println!("{moon:#?}");
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
            println!();
            println!("\x1b[1;4mTextures\x1b[21;22;24m");
            for (name, data) in moon.textures.src {
              let data = Array::into_inner(data);
              println!("• \x1b[1m{name}\x1b[21;22;24m {}B", data.len());
            }
          } else {
            println!(
              "• \x1b[1m{} texture{}",
              moon.textures.src.len(),
              if moon.textures.src.len() == 1 {
                ""
              } else {
                "s"
              }
            );
          }
        }
        if !moon.scripts.is_empty() {
          if verbose {
            println!();
            println!("\x1b[1;4mScripts\x1b[21;22;24m");
            for (name, data) in moon.scripts {
              let data = Array::into_inner(data);
              println!("• \x1b[1m{name}\x1b[21;22;24m {}b", data.len());
              if sources {
                println!("{}", String::from_utf8_lossy(&data));
              }
            }
          } else {
            println!(
              "• \x1b[1m{} script{}",
              moon.scripts.len(),
              if moon.scripts.len() == 1 { "" } else { "s" }
            );
          }
        }
        if let Some(models) = moon.models {
          println!("\n\x1b[1;4mModels\x1b[21;22;24m");
          use moon::{ModelData, ModelPart};
          fn recurse_tree(part: &ModelPart, indent: usize) {
            for _ in 0..indent {
              print!("  ")
            }
            match part.data {
              ModelData::Cube { .. } => println!("• \x1b[34m{}\x1b[m", part.name),
              ModelData::Mesh { ref mesh_data } => println!(
                "• \x1b[31m{}\x1b[0;2m ({}f, {}v)\x1b[22m",
                part.name,
                mesh_data.tex.len(),
                mesh_data.uvs.len()
              ),
              ModelData::Group {} => println!("• {}", part.name),
            }
            for m in &part.chld {
              recurse_tree(m, indent + 1);
            }
          }
          for m in &models.chld {
            recurse_tree(m, 0);
          }
        }
      }
    }
    Action::ParseBbmodel { file } => {
      let file = File::open(file)?;
      let data: Result<BBModel, _> = serde_json::from_reader(file);
      println!("{data:#?}");
    }
    Action::Element {
      path,
      hierarchy,
      index,
      modify,
    } => {
      let file = File::open(path)?;
      // FIXME: don't panic
      let (mut moon, tag_name) = get_moon_with_name(file).expect("loading moon failed");
      modify.apply(&mut moon);
      let mut node = moon.models.unwrap();
      for i in index {
        node = node.chld.into_vec().swap_remove(i);
      }
      if hierarchy {
        if let Ok(value) = node.hierarchy() {
          println!("{}", serde_json::to_string(&value).unwrap());
          println!("Textures: {:?}", value.textures());
        } else {
          panic!("user error");
        }
      } else {
        let mut elements = vec![];
        let item = node.convert_elements(&mut elements);
        for element in elements {
          let value = serde_json::to_string(&element).unwrap();
          println!("{value}");
        }
        let value = serde_json::to_string(&item).unwrap();
        println!("{value}");
      }
    }
    Action::Pack { .. } => todo!(),
    #[cfg(feature = "unpack")]
    Action::Unpack {
      file,
      out,
      modify,
      paths,
      folder,
      filter,
    } => {
      let file = File::open(file)?;
      // FIXME: don't panic
      let mut moon = get_moon(file).expect("no opening moon");
      modify.apply(&mut moon);
      let Moon {
        textures: moon::Textures { src, .. },
        scripts,
        animations,
        models,
        metadata,
        resources,
      } = moon;
      let mut contents = HashMap::<PathBuf, &[u8]>::new();
      let mut omitted = 0;
      macro_rules! add_if_whitelisted {
        ($name:expr => $data:expr) => {
          let name: &str = $name;
          'a: {
            if let Some(paths) = &paths {
              let mut whitelisted = false;
              for prefix in paths {
                if if prefix.ends_with("/") {
                  name.starts_with(prefix)
                } else {
                  name == *prefix
                } {
                  let data: &[u8] = $data;
                  contents.insert(out.join(Path::new(&name)), data);
                  break 'a;
                }
              }
              omitted += 1;
            } else {
              let data: &[u8] = $data;
              contents.insert(out.join(Path::new(&name)), data);
            }
          }
        };
      };
      if filter.textures {
        for (path, data) in &src {
          add_if_whitelisted!(&(path.replace('.', "/") + ".png") => &data.as_ref());
        }
      }
      if filter.scripts {
        for (path, data) in &scripts {
          add_if_whitelisted!(&(path.replace('.', "/") + ".lua") => &data.as_ref());
        }
      }
      if filter.models {
        if let Some(models) = models {
          for part in models.chld.into_vec() {
            add_if_whitelisted!(&(part.name.clone() + ".bbmodel") => {
                let Ok(hier) = part.hierarchy() else { panic!("you smell bad") };
                let model: BBModel = hier.into();
                let json = serde_json::to_string(&model).expect("failed to process model root");
                json.leak().as_bytes()
            });
          }
        }
      }
      if filter.resources {
        for (path, data) in &resources {
          add_if_whitelisted!(&path => &data.as_ref());
        }
      }
      let mut filter_holder: Option<String> = None;
      if filter.manifest {
        let (author, authors) = {
          let v = metadata.authors.split("\n").collect::<Vec<_>>();
          match v[..] {
            ["?"] => (None, vec![]),
            [x] => (Some(metadata.authors), vec![]),
            _ => (None, v.into_iter().map(String::from).collect()),
          }
        };
        let data = serde_json::to_string_pretty(&moon::JsonMetadata {
          name: Some(metadata.name),
          description: Some(metadata.description),
          author: author,
          version: Some(metadata.ver),
          color: metadata.color,
          background: metadata.bg,
          id: None,
          authors,
          autoScripts: None, // TODO
          autoAnims: vec![], // TODO
          ignoredTextures: vec![],
          resources: if filter.resources {
            resources.keys().cloned().collect()
          } else {
            vec![]
          },
          customizations: HashMap::default(), // TODO: generate customizations as needed
        })
        .unwrap();
        add_if_whitelisted!("avatar.json" => filter_holder.insert(data).as_bytes());
      }
      // if models.chld.len() > 0 {
      // eprintln!("warning: extracting models not supported yet")
      // }
      let mut dirs: Vec<_> = contents
        .keys()
        .filter_map(|p| p.parent().map(PathBuf::from))
        .collect();
      dirs.sort();
      dirs.dedup();
      let mut written = 0;
      let mut fails = std::num::Saturating(0i8);
      for dir in dirs {
        if let Err(e) = create_dir_all(&dir) {
          fails += 1;
          eprintln!("failed to mkdir {}: {e}", dir.display());
          contents.retain(|lost, _| {
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
      for (file, data) in contents {
        if let Err(e) = write(&file, data) {
          fails += 1;
          eprintln!("failed to write {}: {e}", file.display());
        } else {
          written += 1;
        }
      }
      eprintln!(
        "wrote {written} file{}{}",
        if written == 1 { "" } else { "s" },
        if omitted > 0 {
          format!(" ({omitted} omitted)")
        } else {
          "".into()
        }
      );
      std::process::exit(fails.0.into())
    }
    Action::Repack {
      file,
      out,
      compress,
      no_compress,
      if_smaller,
      modify,
    } => {
      let mut moon = File::open(&file)?;
      // FIXME: don't panic
      let (mut moon, name) = get_moon_with_name(moon).expect("couldn't load moon");
      modify.apply(&mut moon);
      use flate2::Compression;
      use quartz_nbt::serde as qs;
      let compression = if no_compress {
        Compression::none()
      } else {
        match compress {
          Some(Some(n)) => Compression::new(n),
          Some(None) => Compression::best(),
          None => Compression::default(),
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
    Action::Fok {
      stock,
      first,
      second,
      third,
    } => {
      let mut path = Vec::<u8>::from(env!("FOKDIR"));
      path.extend_from_slice(b"/"); // needed to concatenate paths
      path.extend_from_slice(match (stock, first, second, third) {
        (false, false, false, false) => b"seal.png" as &[u8],
        (true, false, false, false) => b"fok.png" as &[u8],
        (false, true, false, false) => b"seal1.png" as &[u8],
        (false, false, true, false) => b"seal2.png" as &[u8],
        (false, false, false, true) => b"seal3.png" as &[u8],
        _ => unreachable!(),
      });
      println!(
        "\x1b_Gf=100,t=f,a=T,r=10;{}\x1b\\",
        BASE64_STANDARD.encode(&path)
      );
    }
  }
  Ok(())
}
