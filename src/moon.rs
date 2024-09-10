use std::collections::HashMap;
use std::ffi::OsStr;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Moon {
    textures: Textures,
    // scripts: HashMap<String, Box<OsStr>>,
    animations: Box<[Animation]>,
    models: ModelPart,
    metadata: Metadata,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Textures {
    // src: HashMap<String, Box<[u8]>>,
    data: Box<[TextureData]>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TextureData {
    d: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Animation {
    r#loop: Option<Loop>,
    name: String,
    ovr: Option<u8>,
    mdl: String,
    len: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Loop {
    Loop,
    Hold,
}

#[derive(Debug, Serialize, Deserialize)]
struct Metadata {
    authors: Authors,
    color: String,
    name: String,
    ver: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum Authors {
    Author(String),
    Authors(Box<[String]>),
}

#[derive(Debug, Serialize, Deserialize)]
struct ModelPart {
    name: String,
    #[serde(default)]
    chld: Box<[ModelPart]>,
    // anim: Option<TODO>,
    rot: Option<[f64; 3]>,
    piv: Option<[f64; 3]>,
    pt: Option<ParentType>,
}

#[derive(Debug, Serialize, Deserialize)]
enum ParentType {
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
