use std::collections::HashMap;
use std::ffi::OsStr;
use serde::{Serialize, Deserialize};
use quartz_nbt::serde::Array;

#[derive(Debug, Serialize, Deserialize)]
pub struct Moon {
    pub textures: Textures,
    pub scripts: HashMap<String, Array<Vec<u8>>>,
    pub animations: Box<[Animation]>,
    pub models: ModelPart,
    pub metadata: Metadata,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Textures {
    pub src: HashMap<String, Array<Vec<u8>>>,
    pub data: Box<[TextureData]>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TextureData {
    pub d: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Animation {
    pub r#loop: Option<Loop>,
    pub name: String,
    pub ovr: Option<u8>,
    pub mdl: String,
    pub len: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Loop {
    Loop,
    Hold,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Metadata {
    pub authors: Authors,
    pub color: String,
    pub name: String,
    pub ver: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Authors {
    Author(String),
    Authors(Box<[String]>),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelPart {
    pub name: String,
    #[serde(default)]
    pub chld: Box<[ModelPart]>,
    // anim: Option<TODO>,
    pub rot: Option<[f64; 3]>,
    pub piv: Option<[f64; 3]>,
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
