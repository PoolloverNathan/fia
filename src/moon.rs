//! This module implements parsing, serialization, and deserialization for Figura's internal avatar
//! format (previously known as “moons”). Moons are created when an avatar is selected in Figura's
//! wardrobe, and are stored on the backend when uploading. When Figura creates a moon, a lot of
//! data is lost, complicating reverse conversion. This struct aims to capture everything Figura
//! *does* store, and can be used to:
//!
//! * Analyze avatar size.
//! * Create avatars entirely from Rust code.
//! * Load avatars from the filesystem (e.g. `/figura export avatar`).
//! * Upload avatars to the backend, when I get around to implementing backend connections.

use derivative::Derivative;
use quartz_nbt::{serde::Array, NbtTag};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::borrow::Cow;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::hash::Hash;
use uuid::Uuid;

/// The top-level of a Figura avatar. This structure contains maps for avatar information, but
/// since Figura may add more keys at any time, this cannot be exhaustive.
#[non_exhaustive]
#[serde(deny_unknown_fields)]
#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Moon {
  /// Textures associated with this avatar, found in a bbmodel.
  #[serde(default)]
  pub textures: Textures,
  /// This avatar's scripts (stored as `u8`s since Lua is not neccessarily UTF-8).
  #[serde(default)]
  pub scripts: HashMap<String, Array<Vec<u8>>>,
  /// This avatar's animations. I haven't investigated this struct's layout yet.
  #[serde(default)]
  pub animations: Vec<NbtTag>,
  /// The root of the [ModelPart] hierarchy. This can technically be omitted, although I have
  /// always seen it present in practice.
  #[serde(default)]
  pub models: Option<ModelPart>,
  /// Resources available to [ResourcesAPI]. These are arbitrary binary data blobs included in
  /// the avatar folder and uploaded to the backend. I haven't seen a practical use of resources
  /// yet, but I include them anyway.
  ///
  /// [ResourcesAPI]: https://applejuiceyy.github.io/figs/latest/ResourcesAPI/
  #[serde(default)]
  pub resources: HashMap<String, Array<Vec<u8>>>,
  /// Additional metadata loaded from `avatar.json`.
  #[serde(default)]
  pub metadata: Metadata,
}

/// Stores the mapping of texture data sources and the list of textures available to modelparts.
#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Textures {
  /// Raw texture data. The values of this map are PNG-encoded images, but I'm not masochistic
  /// enough to include PNG deserialization in a Figura avatar parser module.
  #[serde(default)]
  pub src: HashMap<String, Array<Vec<u8>>>,
  /// An indexed list associating each texture ID (used by [Face::tex] and [MeshData::tex]) with
  /// each texture type.
  #[serde(default)]
  pub data: Box<[TextureData]>,
}

/// A set of textures used by modelparts.
#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct TextureData {
  /// The primary texture, which is not given a name suffix.
  pub d: String,
  /// The secondary (emissive) texture, which usually has the same name as [`d`] but with an `_e`
  /// suffixed.
  pub e: Option<String>,
}

/// Unused. I don't remember writing this struct.
#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(missing_docs)]
pub struct Animation {
  #[serde(default)]
  pub r#loop: Option<Loop>,
  #[serde(default)]
  pub name: String,
  #[serde(default)]
  pub ovr: u8,
  #[serde(default)]
  pub mdl: String,
  #[serde(default)]
  pub len: f64,
}

/// A loop mode. This could technically have non-looping, although I have only seen it omitted in
/// practice. You will usually deal with an [`Option<Loop>`][Option] instead, with [None]
/// representing non-looping.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Loop {
  /// The animation will return to the beginning when it hits the end.
  Loop,
  /// The animation will maintain the values of the last keyframe after the end. For legal
  /// reasons, the animation will still be considered playing while holding.
  Hold,
}

/// Extra avatar data found almost-exactly in `avatar.json`. This is usually safe to dump to JSON
/// directly (via e.g. [serde_json]).
#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Metadata {
  /// The avatar's UUID, for some reasom?
  #[serde(default)]
  pub uuid: String,
  /// Author(s) of the model, separated by newlines. If unspecified, is the single author `"?"`.
  #[serde(default)]
  pub authors: String,
  /// The avatar's color (used for e.g. UI theming and the Figura mark). This should ideally be a
  /// hex code, but Figura may accept certain color names.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub color: Option<String>,
  /// Avatar's background color. Read from JSON and completely unused.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub bg: Option<String>,
  #[allow(missing_docs)]
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub id: Option<String>,
  /// The display name of the avatar. Rarely used in Figura.
  #[serde(default)]
  pub name: String,
  /// The display text of the avatar in the avatar list. This is not normally visible once the
  /// avatar is loaded, and is only visible under the avatar's name in the wardrobe.
  #[serde(default)]
  pub description: String,
  /// Target Figura version, if specified.
  #[serde(default)]
  pub ver: String,
}

/// Avatar metadata as stored in avatar.json. Used for serialization.
#[derive(Default, Debug, Serialize, Deserialize)]
pub struct JsonMetadata {
  /// The name of the avatar displayed in the picker; defaults to the avatar folder.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub name: Option<String>,
  /// The description of this avatar for the sidebar.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub description: Option<String>,
  /// A single author. Wrapped into a single element of [authors].
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub author: Option<String>,
  /// The target Figura version of this avatar; defaults to the Figura version loading this avatar.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub version: Option<String>,
  /// The color of this avatar for Figura UI accent or the Figura mark.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub color: Option<String>,
  /// Unused in base Figura, but preserved nonetheless.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub background: Option<String>,
  /// Unused in base Figura; seeminly for avatar loading?
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub id: Option<String>,

  /// A list of authors. Mutually exclusive with [author].
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub authors: Vec<String>,
  /// A list of scripts to execute automatically. If [None], all scripts will be executed in alphabetical order.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub autoScripts: Option<Vec<String>>,
  /// A list of animations (usually looping) to play automatically.
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub autoAnims: Vec<String>,
  /// A list of texture names to delete from the avatar. I'm currently unsure what happens to faces with ignored textures.
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub ignoredTextures: Vec<String>,
  /// List of glob patterns to load and store arbitrary files from the avatar folder. This may be used for *shenanigans*: see the Colormagic library.
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub resources: Vec<String>,

  /// Map of [ModelPart] paths (with groups separated by dots) to [Customizations].
  #[serde(default, skip_serializing_if = "HashMap::is_empty")]
  pub customizations: HashMap<String, Customization>,
}

/// A customization allows modifying the behavior of a model part in a manner that dissociates it from its imported Blockbench element. The [Unpack](crate::Action::Unpack) task may generate customizations for models that are impossible to represent in base Blockbench.
#[derive(Debug, Serialize, Deserialize)]
pub struct Customization {
  // TODO
}

fn return_true() -> bool {
  true
}

/// Represents one of Figura's supported render types.
// TODO: make enum
pub type RenderType = String;

/// One of the parts on a model. This can be a group, cube, or mesh, and unrelatedly to this
/// distinction can have children. Unlike other Figura types, this is [stored as a
/// *tree*][Moon::models].
#[derive(Default, Debug, Serialize, Deserialize, Derivative)]
#[derivative(Hash)]
pub struct ModelPart {
  /// The name of this modelpart.
  pub name: String,
  /// This modelpart's children.
  #[serde(default)]
  pub chld: Box<[ModelPart]>,
  /// Presumably animation-related; unsure. This will be ignored when hashing until it becomes
  /// fully typed.
  #[derivative(Hash = "ignore")]
  pub anim: Option<NbtTag>,
  /// Rotation of this model part. This is floating-point and therefore ignored when hashing.
  #[serde(default)]
  #[derivative(Hash = "ignore")]
  pub rot: [f64; 3],
  /// Pivot point of this model part. This is floating-point and therefore ignored when hashing.
  #[serde(default)]
  #[derivative(Hash = "ignore")]
  pub piv: [f64; 3],
  /// Primary render type (used for primary texture).
  pub primary: Option<RenderType>,
  /// Secondary render type (used for emissive texture, if any).
  pub secondary: Option<RenderType>,
  /// Parent type if the name contains one (or it's applied by a customization).
  pub pt: Option<ParentType>,
  /// Whether this cube is visible.
  #[serde(default = "return_true")]
  pub vsb: bool,
  /// Whether to smooth the part's normals. Overridden if FORCE_SMOOTH_AVATAR is enabled. Only
  /// has an effect when the modelpart has vertices, i.e. is not a group.
  #[serde(default)]
  pub smo: bool,
  /// Extra information that depends on the part type. Since cubes have extra top-level keys,
  /// this can't simply be an externally-tagged enum — instead, the enum is untagged and this
  /// field is flattened.
  #[serde(flatten)]
  pub data: ModelData,
  /// This modelpart's UUID, stored as four ints for compactness.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub nr: Option<[u32; 4]>,
  /// List of collections in this part. The presence of this tag proves a modelpart comes from a Blockbench model.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub cn: Option<Vec<String>>,
  /// List of collections this part is a member of, as indices into a parent part's [`cn`](ModelPart::cn) tag.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub pr: Option<Vec<u32>>,
}

// door pin 6485
use crate::bbmodel::{self, Element, Hierarchy, OutlinerItem};
impl ModelPart {
  /// Returns this part's saved UUID, or guesses one if it does not have one. Uses the given hash to avoid duplicates.
  pub fn get_uuid_with_salt(&self, salt: impl Hash) -> Uuid {
    match self.nr {
      Some(x) => Uuid::from_u64_pair(
        (x[0] as u64) << 32u64 | x[1] as u64,
        (x[2] as u64) << 32u64 | x[3] as u64,
      ),
      None => {
        const CONVERT_NS: Uuid = uuid::uuid!("82703e95-07cb-41eb-8591-0ae63fc1e2db");
        Uuid::new_v5(
          &CONVERT_NS,
          &{
            use std::hash::{DefaultHasher, Hasher};
            let mut h = DefaultHasher::new();
            salt.hash(&mut h);
            self.hash(&mut h);
            h.finish()
          }
          .to_le_bytes(),
        )
      }
    }
  }
  /// Returns this part's saved UUID, or guesses one if it does not have one.
  pub fn get_uuid(&self) -> Uuid {
    match self.nr {
      Some(x) => Uuid::from_u64_pair(
        (x[0] as u64) << 32u64 | x[1] as u64,
        (x[2] as u64) << 32u64 | x[3] as u64,
      ),
      None => {
        const CONVERT_NS: Uuid = uuid::uuid!("82703e95-07cb-41eb-8591-0ae63fc1e2db");
        Uuid::new_v5(
          &CONVERT_NS,
          &{
            use std::hash::{DefaultHasher, Hasher};
            let mut h = DefaultHasher::new();
            self.hash(&mut h);
            h.finish()
          }
          .to_le_bytes(),
        )
      }
    }
  }
  /// Converts the [`ModelPart`]'s hierarchy to an [`OutlinerItem`], writing any leaf [`Element`]s
  /// encountered to the passed vector.
  pub fn convert_elements(self, elements: &mut Vec<Element>) -> OutlinerItem {
    use bbmodel::{ElementType, Group};
    let uuid = self.get_uuid_with_salt(elements.len());
    let ModelPart {
      name,
      chld,
      rot,
      piv,
      pt,
      vsb,
      data,
      ..
    } = self;
    let part = bbmodel::Element {
      allow_mirror_modeling: true,
      color: 0,
      export: Some(true),
      extra: match data {
        ModelData::Group {} => {
          return OutlinerItem::Group(Group {
            name,
            origin: piv,
            children: chld
              .into_vec()
              .into_iter()
              .map(|p: ModelPart| p.convert_elements(elements))
              .collect(),
            uuid: uuid.to_string().into(),
            ..Default::default()
          })
        }
        ModelData::Cube {
          cube_data,
          t,
          f,
          inf,
        } => {
          assert!(chld.len() == 0);
          ElementType::Cube {
            from: f,
            to: t,
            uv_offset: None,
            faces: bbmodel::Faces {
              north: cube_data.n.map(Into::into).unwrap_or_default(),
              east: cube_data.e.map(Into::into).unwrap_or_default(),
              south: cube_data.s.map(Into::into).unwrap_or_default(),
              west: cube_data.w.map(Into::into).unwrap_or_default(),
              up: cube_data.u.map(Into::into).unwrap_or_default(),
              down: cube_data.d.map(Into::into).unwrap_or_default(),
            },
            autouv: 0,
            box_uv: None,
            inflate: Some(inf),
            light_emission: None,
            mirror_uv: false.into(),
            rescale: false,
            shade: None,
          }
        }
        ModelData::Mesh { mesh_data } => {
          return OutlinerItem::Group(Group {
            name,
            origin: piv,
            uuid: uuid.to_string().into(),
            ..Default::default()
          })
        } // TODO: implement mesh conversion
      },
      locked: false,
      name,
      origin: piv,
      render_order: None,
      rotation: rot,
      uuid: uuid.to_string(),
      visibility: Some(vsb),
    };
    let uuid = part.uuid.clone();
    elements.push(part);
    OutlinerItem::Element(uuid)
  }
  /// Creates a [`Hierarchy`] from a ModelPart. The part must be of type [`ModelData::Group`]; if
  /// not, it will be returned to you.
  pub fn hierarchy(self) -> Result<Hierarchy, ModelPart> {
    let mut elements = vec![];
    let ModelData::Group {} = self.data else {
      return Err(self);
    };
    Ok(Hierarchy {
      outliner: self
        .chld
        .into_vec()
        .into_iter()
        .map(|p| p.convert_elements(&mut elements))
        .collect(),
      elements,
    })
  }
}

/// Stores extra data for a modelpart depending on what type of model it has, if any.
#[derive(Debug, Serialize, Deserialize, Derivative)]
#[derivative(Hash)]
#[serde(untagged)]
#[serde(deny_unknown_fields)]
pub enum ModelData {
  /// A group, with no model data.
  Group {},
  /// A cube, which is not a cube (more generally, it's a rectangular prism). Most cube fields
  /// are ignored when hashing!
  Cube {
    /// Maps each side of the cube to its UV and texture data.
    cube_data: Sided<Face>,
    /// The point where the cube begins. I'm unsure of what coordinate space this location is
    /// in.
    #[derivative(Hash = "ignore")]
    f: [f64; 3],
    /// The point where the cube begins. May be less than [f][Self::f] for inverted cubes. This
    /// is probably in the same coordinate space as [f][Self::f].
    #[derivative(Hash = "ignore")]
    t: [f64; 3],
    /// The cube's inflate scale. This is equivalent to subtracting this value from each number
    /// in [f][Self::f] and adding it to each value in [t][Self::t], except it doesn't affect
    /// Blockbench's generated UVs.
    #[serde(default)]
    #[derivative(Hash = "ignore")]
    inf: f64,
  },
  /// A mesh, which supports freely adding and moving faces at the expense of file size.
  Mesh {
    /// Data for meshes. To be honest, I'm surprised that Figura didn't flatten this struct.
    mesh_data: MeshData,
  },
}

/// Maps each side of something (such as a cube) to an object.
#[derive(Debug, Serialize, Deserialize, Hash)]
#[serde(deny_unknown_fields)]
pub struct Sided<S> {
  /// The north face.
  pub n: Option<S>,
  /// The south face.
  pub s: Option<S>,
  /// The upward face.
  pub u: Option<S>,
  /// The downward face.
  pub d: Option<S>,
  /// The west face.
  pub w: Option<S>,
  /// The east face.
  pub e: Option<S>,
}

/// Texture and UV information for each face of a cube.
#[serde(deny_unknown_fields)]
#[derive(Debug, Serialize, Deserialize, Derivative)]
#[derivative(Hash)]
pub struct Face {
  /// The texture ID in [Textures::data].
  pub tex: usize,
  /// The UV information (presumably `[x0, y0, x1, y1]`, but I haven't confirmed this). Ignored
  /// when hashing.
  #[derivative(Hash = "ignore")]
  pub uv: [f64; 4],
  /// How the face is rotated. Ignored when hashing.
  #[serde(default)]
  #[derivative(Hash = "ignore")]
  pub rot: f64,
}

impl Into<crate::bbmodel::Face> for Face {
  fn into(self) -> crate::bbmodel::Face {
    crate::bbmodel::Face {
      rotation: self.rot,
      texture: self.tex.into(),
      uv: self.uv,
    }
  }
}

/// Texture and vertex information for meshes. Figura stores this in a very compact manner, but
/// this makes proper interaction from Rust code difficult. Use the
#[derive(Debug, Clone, Serialize, Derivative)]
#[derivative(Hash)]
pub struct MeshData {
  /// The X, Y, and Z position of each vertex, consecutively. These are not considered for
  /// hashing.
  #[derivative(Hash = "ignore")]
  pub vtx: Vec<f64>,
  /// The texture ID (see [Textures::data]) left-shifted 4, plus the number of vertices in the
  /// face.
  pub tex: Vec<u16>,
  /// List of faces. These values are indexes into [`vtx`](Self::vtx); their significance after that is lost to me.
  pub fac: Vec<u32>,
  /// UV vertexes. These are not considered for hashing.
  #[derivative(Hash = "ignore")]
  pub uvs: Vec<f64>,
}

mod mesh {
  use super::MeshData;
  struct Vertex {}
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MeshDataDelegate {
  pub vtx: Vec<f64>,
  pub tex: Vec<u16>,
  pub fac: Fac,
  pub uvs: Vec<f64>,
}

#[allow(missing_docs)]
#[derive(Deserialize)]
#[serde(untagged)]
enum Fac {
  U8(Vec<u8>),
  U16(Vec<u16>),
  U32(Vec<u32>),
}

impl<'de> Deserialize<'de> for MeshData {
  fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
    let data = MeshDataDelegate::deserialize(deserializer)?;
    Ok(MeshData {
      vtx: data.vtx,
      tex: data.tex,
      fac: match data.fac {
        Fac::U8(x) => x.into_iter().map(|x| x.into()).collect(),
        Fac::U16(x) => x.into_iter().map(|x| x.into()).collect(),
        Fac::U32(x) => x.into(),
      },
      uvs: data.uvs,
    })
  }
}

impl Default for ModelData {
  fn default() -> Self {
    Self::Group {}
  }
}

/// A parent type determined by Figura. Although usually the parent type can be determined based on
/// the [ModelPart]'s name, Figura for some reason stores a copy anyway. This enum documents each
/// possible parent type.
#[derive(Debug, Serialize, Deserialize, Hash)]
#[allow(missing_docs)]
pub enum ParentType {
  /// No parent type — follows parent's rotations.
  None,

  // Body
  Head,
  Body,
  LeftArm,
  RightArm,
  LeftLeg,
  RightLeg,
  LeftElytra,
  RightElytra,
  Cape,

  // Misc
  World,
  Hud,
  Camera,
  Skull,
  Portrait,
  Arrow,
  Trident,
  Item,

  // Held
  LeftItemPivot,
  RightItemPivot,
  LeftSpyglassPivot,
  RightSpyglassPivot,
  LeftParrotPivot,
  RightParrotPivot,

  // Armor
  HelmetItemPivot,
  HelmetPivot,
  ChestplatePivot,
  LeftShoulderPivot,
  RightShoulderPivot,
  LeggingsPivot,
  LeftLeggingPivot,
  RightLeggingPivot,
  LeftBootPivot,
  RightBootPivot,
  LeftElytraPivot,
  RightElytraPivot,
}
