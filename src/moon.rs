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

use std::collections::HashMap;
use std::ffi::OsStr;
use serde::{Serialize, Deserialize};
use quartz_nbt::{NbtTag, serde::Array};

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
    /// Author(s) of the model. If unspecified, is the single author `"?"`.
    #[serde(default)]
    pub authors: Authors,
    /// The avatar's color (used for e.g. UI theming and the Figura mark). This should ideally be a
    /// hex code, but Figura may accept certain color names.
    #[serde(default)]
    pub color: String,
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

/// Represents the author or authors of an avatar. Figura, for some strange reason, differentiates
/// between the single-author and multi-author case, so I preserve this distinction when
/// deserializing avatars.
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Authors {
    /// One author, or the pseudoauthor `"?"`.
    Author(String),
    /// Multiple authors.
    Authors(Vec<String>),
}
impl Default for Authors {
    fn default() -> Self {
        Authors::Authors(vec![])
    }
}

fn return_true() -> bool { true }

/// Represents one of Figura's supported render types.
// TODO: make enum
pub type RenderType = String;

/// One of the parts on a model. This can be a group, cube, or mesh, and unrelatedly to this
/// distinction can have children. Unlike other Figura types, this is [stored as a
/// *tree*][Moon::models].
#[derive(Default, Debug, Serialize, Deserialize)]
pub struct ModelPart {
    /// The name of this modelpart.
    pub name: String,
    /// This modelpart's children.
    #[serde(default)]
    pub chld: Box<[ModelPart]>,
    /// Presumably animation-related; unsure.
    pub anim: Option<NbtTag>,
    /// Rotation of this model part.
    #[serde(default)]
    pub rot: [f64; 3],
    /// Pivot point of this model part.
    #[serde(default)]
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
}

/// Stores extra data for a modelpart depending on what type of model it has, if any.
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
#[serde(deny_unknown_fields)]
pub enum ModelData {
    /// A group, with no model data.
    Group {},
    /// A cube, which is not a cube (more generally, it's a rectangular prism).
    Cube {
        /// Maps each side of the cube to its UV and texture data.
        cube_data: Sided<Face>,
        /// The point where the cube begins. I'm unsure of what coordinate space this location is
        /// in.
        f: [f64; 3],
        /// The point where the cube begins. May be less than [f][Self::f] for inverted cubes. This
        /// is probably in the same coordinate space as [f][Self::f].
        t: [f64; 3],
        /// The cube's inflate scale. This is equivalent to subtracting this value from each number
        /// in [f][Self::f] and adding it to each value in [t][Self::t], except it doesn't affect
        /// Blockbench's generated UVs.
        #[serde(default)]
        inf: f64,
    },
    /// A mesh, which supports freely adding and moving faces at the expense of file size.
    Mesh {
        /// Data for meshes. To be honest, I'm surprised that Figura didn't flatten this struct.
        mesh_data: MeshData,
    },
}

/// Maps each side of something (such as a cube) to an object.
#[derive(Debug, Serialize, Deserialize)]
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
#[derive(Debug, Serialize, Deserialize)]
pub struct Face {
    /// The texture ID in [Textures::data].
    pub tex: usize,
    /// The UV information (presumably `[x0, y0, x1, y1]`, but I haven't confirmed this).
    pub uv: [f64; 4],
    /// How the face is rotated.
    #[serde(default)]
    pub rot: f64,
}

/// Texture and vertex information for meshes. I'm not even going to try documenting this right
/// now; ping me in a few hours maybe?
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MeshData {
    /// Vertices.
    pub vtx: NbtTag,
    /// Textures, see [Textures::data].
    pub tex: NbtTag,
    /// Faces.
    pub fac: NbtTag,
    /// UVs, aka hell.
    pub uvs: NbtTag,
    /// Extraneous keys not matched.
    #[serde(flatten)]
    excess: NbtTag,
}

impl Default for ModelData {
    fn default() -> Self {
        Self::Group {}
    }
}

/// A parent type determined by Figura. Although usually the parent type can be determined based on
/// the [ModelPart]'s name, Figura for some reason stores a copy anyway. This enum documents each
/// possible parent type.
#[derive(Debug, Serialize, Deserialize)]
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
