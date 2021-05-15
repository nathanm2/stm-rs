//! A library for building or parsing a MIPI Trace Wrapper Protocol (TWP) stream (aka ARM Trace
//! Formatter Protocol).
//!
//! TWP allows multiple trace streams to combined into a single stream.  It is compatible with with
//! ARM's **Trace Formatter Protocol** emitted by [ARM CoreSight] trace sinks.  A TWP stream is
//! sometimes called **TPIU frames**.
//!
//! The MIPI TWP specification can be found [here].
//!
//! [here]: https://www.mipi.org/specifications/twp
//! [ARM CoreSight]: https://developer.arm.com/documentation/ihi0029/e
//!
pub use error::*;
pub use frame_parser::*;

pub mod error;
pub mod frame_parser;
pub mod stream_builder;
