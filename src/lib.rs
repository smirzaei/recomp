#![cfg_attr(docsrs, feature(doc_cfg))]

//! Reusable components.

pub mod component;

#[cfg_attr(docsrs, doc(cfg(feature = "postgres")))]
#[cfg(feature = "postgres")]
pub mod postgres;
