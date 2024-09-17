use std::collections::HashMap;
use std::ffi::OsStr;
use serde::{Serialize, Deserialize};
use quartz_nbt::serde::Array;

#[derive(Debug, Serialize, Deserialize)]
pub struct Moon {
    #[serde(default)]
    pub textures: Textures,
    #[serde(default)]
    pub scripts: HashMap<String, Array<Vec<u8>>>,
    #[serde(default)]
    pub animations: Vec<Animation>,
    #[serde(default)]
    pub models: ModelPart,
    #[serde(default)]
    pub resources: HashMap<String, Array<Vec<u8>>>,
    #[serde(default)]
    pub metadata: Metadata,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Textures {
    #[serde(default)]
    pub src: HashMap<String, Array<Vec<u8>>>,
    #[serde(default)]
    pub data: Box<[TextureData]>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TextureData {
    pub d: String,
}

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
pub struct Metadata {
    #[serde(default)]
    pub authors: Authors,
    #[serde(default)]
    pub color: String,
    #[serde(default)]
    pub description: String,
    pub name: String,
    pub description: String,
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

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct ModelPart {
    pub name: String,
    #[serde(default)]
    pub chld: Box<[ModelPart]>,
    // anim: Option<TODO>,
    #[serde(default)]
    pub rot: Option<[f64; 3]>,
    #[serde(default)]
    pub piv: Option<[f64; 3]>,
    #[serde(default)]
    pub pt: Option<ParentType>,
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
