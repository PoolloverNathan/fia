use std::collections::HashMap;
use std::ffi::OsStr;
use serde::{Serialize, Deserialize};
use quartz_nbt::{NbtTag, serde::Array};

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Moon {
    #[serde(default)]
    pub textures: Textures,
    #[serde(default)]
    pub scripts: HashMap<String, Array<Vec<u8>>>,
    #[serde(default)]
    pub animations: Vec<NbtTag>,
    #[serde(default)]
    pub models: Option<ModelPart>,
    #[serde(default)]
    pub resources: HashMap<String, Array<Vec<u8>>>,
    #[serde(default)]
    pub metadata: Metadata,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Textures {
    #[serde(default)]
    pub src: HashMap<String, Array<Vec<u8>>>,
    #[serde(default)]
    pub data: Box<[TextureData]>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct TextureData {
    pub d: String,
}

#[derive(Default, Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Loop {
    Loop,
    Hold,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Metadata {
    #[serde(default)]
    pub authors: Authors,
    #[serde(default)]
    pub color: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub ver: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Authors {
    Author(String),
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
    /// Extra information that depends on the part type. This is flattened into 
    #[serde(flatten)]
    pub data: ModelData,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
#[serde(deny_unknown_fields)]
pub enum ModelData {
    Group {},
    Cube {
        /// Data for cubes.
        cube_data: Sided<Face>,
        /// Cube 'from' position.
        f: [f64; 3],
        /// Cube 'to' position.
        t: [f64; 3],
        /// Inflate value.
        #[serde(default)]
        inf: f64,
    },
    Mesh {
        /// Data for meshes.
        mesh_data: MeshData,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Sided<S> {
    pub n: Option<S>,
    pub s: Option<S>,
    pub u: Option<S>,
    pub d: Option<S>,
    pub w: Option<S>,
    pub e: Option<S>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Face {
    pub tex: usize,
    pub uv: [f64; 4],
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MeshData {
    pub vtx: NbtTag,
    pub tex: NbtTag,
    pub fac: NbtTag,
    pub uvs: NbtTag,
}

impl Default for ModelData {
    fn default() -> Self {
        Self::Group {}
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ParentType {
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
