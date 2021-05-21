//! A library for parsing or building a MIPI Trace Wrapper Protocol (TWP) stream.
//!
//! TWP allows multiple trace streams to combined into a single stream.  It is often used in
//! embedded, ARM-based SoC systems.
//!
//! TWP is compatible with ARM's [CoreSight Trace Formatter][ARM01] specification (aka TPIU).
//!
//! The MIPI TWP specification can be found [here][MIPI01].
//!
//! [MIPI01]: https://www.mipi.org/specifications/twp
//! [ARM01]: https://developer.arm.com/documentation/ihi0029/e
//!
pub use error::*;
pub use frame_parser::*;
pub use layer_parser::*;

pub mod error;
pub mod frame_parser;
pub mod layer_parser;
pub mod stream_builder;
