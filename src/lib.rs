//! Various utilities and types for interfacing with Figura's avatars. This crate currently
//! includes:
//!
//! * [Loading avatars from a file][Moon::read]
//! * [Running avatars in-memory][crate::runtime]
//! * [Serving avatars to users][Backend::run]
//! packing/unpacking/repacking moon files and editing assets.

#![allow(warnings)]
#![deny(missing_docs)]
#![feature(never_type)]

pub mod moon;
pub use moon::Moon;

pub mod bbmodel;
