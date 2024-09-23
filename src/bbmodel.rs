use std::collections::HashMap;
use std::ffi::OsStr;
use serde::{Serialize, Deserialize};
use quartz_nbt::serde::Array;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct BBModel {
    pub meta: Meta,
    
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Meta {
    format_version: FormatVersion,
    model_format: ModelFormat,
    box_uv: bool,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub enum FormatVersion {
    #[default]
    #[serde(rename = "4.10")]
    V4_10,
    #[serde(rename = "4.10")]
    V4_9,
    #[serde(rename = "4.10")]
    V4_8,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ModelFormat {
    #[default]
    Generic,
    Figura,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Element {
    Cube {
        from: [f64; 3],
        to: [f64; 3],
        origin: [f64; 3],
        uv_offset: [f64; 3],
        faces: Faces,
        uuid: String, // good enough
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Faces {
    north: Option<Face>,
    east:  Option<Face>,
    south: Option<Face>,
    west:  Option<Face>,
    up:    Option<Face>,
    down:  Option<Face>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Face {
    uv: [f64; 4],
    texture: usize,
}
