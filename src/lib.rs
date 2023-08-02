#![doc = include_str!("../README.md")]
//! ## All together
//!
//! Imagine the following :
//!
//! ```rust
//! # use cmp_by_derive::CmpBy;
//! # use core::cmp::Ordering;
//! #
//! #[derive(Ord, PartialOrd, Eq, PartialEq)]
//! struct Midi {
//!     global_time: usize,
//!     note: Note,
//! }
//!
//! #[derive(CmpBy, Debug)]
//! #[cmp_by(channel(), pitch(), _fields)]
//! enum Note {
//! // ...
//! #    NoteOn {
//! #        pitch: u8,
//! #        channel: u8
//! #    },
//! #    NoteOff {
//! #        pitch: u8,
//! #        channel: u8
//! #    },
//! #    CC ,
//! #    Unsupported {
//! #        raw_data: Vec<u8>,
//! #        channel: u8
//! #    }
//! }
//!
//! impl Note {
//!     fn channel(&self) -> Option<&u8> {
//! #       match self {
//! #           Note::CC => None,
//! #           Note::NoteOn { channel, .. }
//! #           | Note::NoteOff { channel, .. }
//! #           | Note::Unsupported { channel, .. } => Some(channel),
//! #       }
//!     }
//!
//!     fn pitch(&self) -> Option<&u8> {
//! #       match self {
//! #           Note::NoteOn { pitch, .. } | Note::NoteOff { pitch, .. } => Some(pitch),
//! #           _ => None,
//! #       }
//!     }
//! }
//!
//! assert_eq!(
//!     Midi {
//!         global_time: 0,
//!         note: Note::NoteOn {
//!             pitch: 0,
//!             channel: 0,
//!         }
//!     }
//!         .cmp(&Midi {
//!             global_time: 0,
//!             note: Note::NoteOn {
//!                 pitch: 0,
//!                 channel: 0,
//!             }
//!         }),
//!     Ordering::Equal
//! );
//! assert_eq!(
//!     Midi {
//!         global_time: 0,
//!         note: Note::NoteOn {
//!             pitch: 2,
//!             channel: 2,
//!         }
//!     }
//!         .cmp(&Midi {
//!             global_time: 2,
//!             note: Note::NoteOff {
//!                 pitch: 0,
//!                 channel: 0,
//!             }
//!         }),
//!     Ordering::Less
//! );
//! assert_eq!(
//!     Midi {
//!         global_time: 0,
//!         note: Note::NoteOn {
//!             pitch: 2,
//!             channel: 0,
//!         }
//!     }
//!         .cmp(&Midi {
//!             global_time: 0,
//!             note: Note::NoteOff {
//!                 pitch: 0,
//!                 channel: 2,
//!             }
//!         }),
//!     Ordering::Less
//! );
//! assert_eq!(
//!     Midi {
//!         global_time: 0,
//!         note: Note::NoteOn {
//!             pitch: 0,
//!             channel: 0,
//!         }
//!     }
//!         .cmp(&Midi {
//!             global_time: 0,
//!             note: Note::NoteOff {
//!                 pitch: 0,
//!                 channel: 2,
//!             }
//!         }),
//!     Ordering::Less
//! );
//! assert_eq!(
//!     Midi {
//!         global_time: 0,
//!         note: Note::NoteOn {
//!             pitch: 0,
//!             channel: 0,
//!         }
//!     }
//!         .cmp(&Midi {
//!             global_time: 0,
//!             note: Note::NoteOff {
//!                 pitch: 0,
//!                 channel: 0,
//!             }
//!         }),
//!     Ordering::Less
//! );
//! ```
//!
//! Now I have a `Note` enum that will cmp by `global_time`, `channel`, `pitch`, and lastly by variant order ( `enum_sequence` ). Note that `None` is always less than `Some`.
//!
//! Conversely, separate structs such as `NoteOn` may derive from `CmpBy` in order to ignore some fields ( ex: `velocity` may be a `f32`, so we can't directly derive `Ord` ).
use syn::{parse_macro_input, DeriveInput};

mod cmp_by;
mod hash_by;
mod parsing;

/// Fields that should be used for cmping are marked with the attribute `#[cmp_by]`.
/// Other fields will be ignored.
///
/// ```rust
/// # use std::cmp::Ordering;
/// use cmp_by_derive::CmpBy;
///
/// #[derive(CmpBy)]
/// struct Something {
///     #[cmp_by]
///     a: u16,
///     b: u16
/// }
///
/// assert_eq!(Something{a: 2, b: 0}.cmp(&Something{a: 1, b: 1}), Ordering::Greater); // a is compared
/// assert_eq!(Something{a: 1, b: 0}.cmp(&Something{a: 1, b: 1}), Ordering::Equal); // b is ignored
/// ```
/// You can use it the same way with tuple structs:
///
/// ```rust
/// # use std::cmp::Ordering;
/// # use cmp_by_derive::CmpBy;
/// #
/// #[derive(CmpBy)]
/// struct Something(
///     #[cmp_by]
///     u16,
///     #[cmp_by]
///     u32,
///     f32
/// );
///
/// assert_eq!(Something(1, 0, 1.0).cmp(&Something(1, 0, 2.0)), Ordering::Equal); // Compares only specified fields
/// assert_eq!(Something(2, 0, 1.0).cmp(&Something(1, 0, 2.0)), Ordering::Greater); // Compares only specified fields
/// ```
///
///
/// Alternatively, or in combination with, a struct-level or enum-level `#[cmp_by(method1(),method2(),attr1,nested.attr)]` can be declared.
/// This top-level declaration takes precedence, fields comparison will be considered if top-level comparisons are all `eq`.
/// The top-level `cmp_by` attribute takes a list of attributes or method calls; items will be prepended with `self.`.
///
/// ```rust
/// # use std::cmp::Ordering;
/// # use cmp_by_derive::CmpBy;
/// #
/// #[derive(CmpBy)]
/// #[cmp_by(product())]
/// struct Something {
///     #[cmp_by]
///     a: u16,
///     b: u16,
/// }
///
/// impl Something {
///     fn product(&self) -> u16 {
///         self.a * self.b
///     }
/// }
///
/// assert_eq!(Something{a: 1, b: 1}.cmp(&Something{a: 1, b: 2}), Ordering::Less); // method comparison precedes member comparison
/// assert_eq!(Something{a: 2, b: 0}.cmp(&Something{a: 1, b: 0}), Ordering::Greater); // method comparison is equal (0 = 0) so fall back to member comparison
/// ```
///
/// ## Limitation
///
/// - struct-level `cmp_by` attribute always comes before field-level attributes lexicographically.
#[proc_macro_derive(CmpBy, attributes(cmp_by))]
pub fn cmp_by_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    cmp_by::impl_cmp_by_derive(ast).into()
}

#[proc_macro_derive(HashBy, attributes(hash_by))]
pub fn hash_by_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    hash_by::impl_hash_by_derive(ast).into()
}
