#![warn(missing_docs)]

use std::collections::HashMap;
use std::ffi::OsStr;
use serde::{Serialize, Deserialize};
use serde_repr::{Serialize_repr, Deserialize_repr};
use serde_json::{Value, Number, Map};
type Any = Option<Value>;
type Object = Map<Value, Value>;

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct BBModel {
    pub activity_tracker: Any,
    #[serde(default)]
    pub animation_variable_placeholders: String,
    #[serde(default)]
    pub animations: Vec<Animation>,
    pub box_uv: Any,
    #[serde(default)]
    pub elements: Vec<Element>,
    pub export_options: Any,
    pub meta: Meta,
    pub model_identifier: Option<String>,
    pub name: Option<String>,
    pub outliner: Vec<OutlinerItem>,
    pub reference_images: Any,
    pub resolution: Resolution,
    pub textures: Vec<Texture>,
    pub timeline_setups: Vec<Value>,
    pub unhandled_root_fields: Any,
    pub variable_placeholder_buttons: Vec<Value>,
    pub variable_placeholders: String,
    pub visible_box: Option<[Number; 3]>,
    pub texture_groups: Any,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Resolution {
    height: usize,
    width: usize,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Texture {
    folder: String,
    frame_interpolate: Option<bool>,
    layers: Any,
    frame_order: String,
    frame_order_type: String,
    frame_time: usize,
    group: Option<String>,
    height: usize,
    id: String,
    internal: bool,
    layers_enabled: bool,
    mode: Any,
    name: String,
    namespace: String,
    particle: bool,
    path: String,
    relative_path: Option<String>,
    render_mode: String,
    render_sides: String,
    saved: bool,
    source: String,
    sync_to_project: String,
    #[serde(default)]
    use_as_default: bool,
    uuid: String,
    uv_height: usize,
    uv_width: usize,
    visible: bool,
    width: usize,
}

/// Contains metadata about this model important for making sense of the contents.
#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Meta {
    /// The model's format version. Although this is stored, it is ignored when serializing or
    /// deserializing.
    format_version: FormatVersion,
    /// The model format. This is usually "generic" or "free".
    model_format: String,
    /// Whether Box UV is enabled in Project Settings. This differs from whether individual cubes
    /// use Box UV.
    #[serde(default)]
    box_uv: bool,
}

/// One animation in the model.
#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Animation {
    /// A Molang expression that evaluates to the animation's time. This is only useful for
    /// Bedrock; it is completely ignored by Figura.
    // TODO: is is used in Blockbench?
    anim_time_update: String,
    /// The bones that this animation animates.
    #[serde(default)]
    animators: HashMap<String, Animator>,
    /// A multiplier for this animation's strength. Although Figura has something similar, I don't
    /// know if it's actually used.
    blend_weight: String,
    /// This animation's length; usually the last keyframe's [time][Keyframe::time].
    length: f64,
    /// Whether this animation will loop, and how. The permissible values are unknown.
    r#loop: Any,
    /// How long to wait before looping.
    loop_delay: String,
    /// This animation's name.
    name: String,
    /// In Figura, this specifies whether the animation will override the vanilla animations on
    /// parent types.
    r#override: bool,
    /// Whether this animation is selected.
    // TODO: what does this mean?
    selected: bool,
    /// The precision of keyframes, as expressed by the reciprocal of the step value.
    snapping: u32,
    /// How long to wait before starting, I think.
    start_delay: String,
    /// This animation's unique identifier.
    uuid: String,
    /// Markers?
    markers: Any,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Animator {
    /// This animator's identifier. I don't know what this means.
    r#type: String,
    /// The name(?) of this animator.
    #[serde(alias = "bone")]
    name: String,
    /// The keyframes on this animation.
    keyframes: Vec<Keyframe>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Keyframe {
    /// What channel this keyframe is on.
    pub channel: String,
    pub uniform: Any,
    /// The keyframe's color, or -1 if no color is specified. Did you know keyframes could be
    /// colored?
    pub color: i8,
    /// Why the fuck are there multiple?
    pub data_points: Vec<XYZ<SoN>>,
    /// The interpolation style of this keyframe.
    pub interpolation: String,
    /// When this keyframe is.
    pub time: f64,
    /// Why does everything have a uuid?
    pub uuid: String,
    /// Whether the bézier is linked.
    pub bezier_linked: Option<bool>,
    pub bezier_left_time: Option<[f64; 3]>,
    pub bezier_left_value: Option<[f64; 3]>,
    pub bezier_right_time: Option<[f64; 3]>,
    pub bezier_right_value: Option<[f64; 3]>,
}

/// A value in three axes.
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct XYZ<T> {
    pub x: T,
    pub y: T,
    pub z: T,
}

/// A string or number, since Blockbench accepts both.
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SoN {
    /// A string, such as a Molang expression.
    String(String),
    /// A constant number.
    Number(f64),
}

fn coerce_keyframes<'de, D: serde::Deserializer<'de>>(de: D) -> Result<f64, D::Error> {
    use serde::de::{Visitor, Error};
    use std::fmt::{self, Formatter};
    struct ConvertToFloatVisitor;
    impl<'de> Visitor<'de> for ConvertToFloatVisitor {
        type Value = f64;
        fn expecting(&self, fmt: &mut Formatter) -> fmt::Result {
            write!(fmt, "anything that smells like a string")
        }
        fn visit_i32<E>(self, v: i32) -> Result<f64, E> { Ok(v.into()) }
        fn visit_u32<E>(self, v: u32) -> Result<f64, E> { Ok(v.into()) }
        fn visit_f64<E>(self, v: f64) -> Result<f64, E> { Ok(v) }
        fn visit_str<E: Error>(self, v: &str) -> Result<f64, E> {
            match v.parse() {
                Ok(f) => Ok(f),
                Err(e) => todo!(),
            }
        }
    }
    de.deserialize_any(ConvertToFloatVisitor)
}

/// One of the 4.x Blockbench format versions.
#[derive(Debug, Serialize, Deserialize, Default)]
#[allow(missing_docs)]
pub enum FormatVersion {
    #[default]
    #[serde(rename = "4.10")]
    V4_10,
    #[serde(rename = "4.9")]
    V4_9,
    #[serde(rename = "4.8")]
    V4_8,
    #[serde(rename = "4.7")]
    V4_7,
    #[serde(rename = "4.6")]
    V4_6,
    #[serde(rename = "4.5")]
    V4_5,
    #[serde(rename = "4.4")]
    V4_4,
    #[serde(rename = "4.3")]
    V4_3,
    #[serde(rename = "4.2")]
    V4_2,
    #[serde(rename = "4.1")]
    V4_1,
    #[serde(rename = "4.0")]
    V4_0,
}

/// An intermediate element and outliner tree.
///
/// This struct does not represent a Blockbench type. Instead, it represents a tree for elements
/// and groups that is *part* of a model. It's always possible to extract the hierarchy from a
/// model, but not the other way around.
#[derive(Debug, Serialize, Deserialize, Default)]
#[allow(missing_docs)]
pub struct Hierarchy {
    pub elements: Vec<Element>,
    pub outliner: Vec<OutlinerItem>,
}

impl From<BBModel> for Hierarchy {
    fn from(BBModel { elements, outliner, .. }: BBModel) -> Hierarchy {
        Hierarchy { elements, outliner }
    }
}

fn return_true() -> bool { true }

/// Common information between all types of elements.
#[derive(Debug, Serialize, Deserialize)]
pub struct Element {
    /// The pivot point of this cube.
    #[serde(default)]
    pub origin: [f64; 3],
    /// The cube's name.
    pub name: String,
    pub uuid: String, // good enough
    pub visibility: Option<bool>,
    #[serde(default)]
    pub locked: bool,
    pub render_order: Any,
    /// Whether the cube can be mirrored.
    #[serde(default = "return_true")]
    pub allow_mirror_modeling: bool,
    /// Whether the cube should be exported. If this is disabled, Figura completely ignores the
    /// cube (not even adding it to the modelpart hiearchy).
    pub export: Option<bool>,
    pub color: u8,
    #[serde(default)]
    pub rotation: [f64; 3],
    /// Extension data for each type of modelpart.
    #[serde(flatten)]
    pub extra: ElementType,
}

/// Either a group in the outliner, or the UUID of a cube.
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OutlinerItem {
    Group(Group),
    Element(String),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
struct BoxedUUID(String);
impl Default for BoxedUUID {
    fn default() -> BoxedUUID {
        BoxedUUID(uuid::Uuid::new_v4().to_string())
    }
}

/// Represents a group in the outliner.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Group {
    pub name: String,
    #[serde(default)]
    pub origin: [f64; 3],
    #[serde(default)]
    pub color: u8,
    pub uuid: BoxedUUID,
    #[serde(default = "return_true")]
    pub export: bool,
    #[serde(default)]
    pub mirror_uv: bool,
    #[serde(default)]
    pub isOpen: bool,
    #[serde(default)]
    pub locked: bool,
    #[serde(default = "return_true")]
    pub visibility: bool,
    #[serde(default)]
    pub autouv: u8,
    #[serde(default)]
    pub children: Vec<OutlinerItem>,
}

impl Default for Group {
    fn default() -> Group {
        Group {
            name: Default::default(),
            origin: Default::default(),
            color: 0,
            uuid: Default::default(),
            export: true,
            mirror_uv: false,
            isOpen: false,
            locked: false,
            visibility: true,
            autouv: 0,
            children: vec![],
        }
    }
}

/// A type of element with a model, excluding groups.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(tag = "type")]
pub enum ElementType {
    /// A ~~cube~~ rectangular prism.
    #[serde(rename = "cube")]
    Cube {
        /// The cube's position, in some space.
        from: [f64; 3],
        /// Where the cube ends.
        to: [f64; 3],
        /// The UV position of this cube.
        uv_offset: Option<[f64; 2]>,
        /// The faces on this cube.
        faces: Faces,
        box_uv: Any,
        rescale: bool,
        autouv: u8,
        light_emission: Option<u8>,
        mirror_uv: Option<bool>,
        inflate: Option<f64>,
        shade: Any,
    },
    /// A mesh, with free vertices.
    #[serde(rename = "mesh")]
    Mesh {
        vertices: HashMap<String, [f64; 3]>,
        faces: HashMap<String, MeshFace>,
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MeshFace {
    pub uv: HashMap<String, [f64; 2]>,
    pub vertices: Vec<String>,
    pub texture: Option<usize>,
}

/// A [Face] for each side of a cube. This is just [crate::moon::Side] with different field names.
#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Faces {
    pub north: Option<Face>,
    pub east:  Option<Face>,
    pub south: Option<Face>,
    pub west:  Option<Face>,
    pub up:    Option<Face>,
    pub down:  Option<Face>,
}

/// The texture and UV position of a face.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Face {
    pub uv: [f64; 4],
    pub texture: Option<usize>,
    #[serde(default)]
    pub rotation: f64,
}
