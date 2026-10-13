//! `serde` support for [`Wall`] (feature `serde`).
//!
//! A `Wall` serializes exactly like a [`std::time::SystemTime`] does under
//! serde: a struct named `SystemTime` with the fields `secs_since_epoch`
//! (`u64`) and `nanos_since_epoch` (`u32`). A `Wall` field can therefore
//! replace a `SystemTime` field without changing the wire format, and data
//! written by either deserializes into the other.
//!
//! As with `SystemTime`, readings before the Unix epoch cannot be
//! serialized and produce a serializer error.
//!
//! [`Monotonic`](crate::Monotonic) deliberately has no serde support: a
//! monotonic reading is only meaningful inside the process that took it.

use std::time::{SystemTime as StdSystemTime, UNIX_EPOCH as STD_UNIX_EPOCH};

use serde::de::{Deserialize, Deserializer, Error as _};
use serde::ser::{Error as _, Serialize, Serializer};

use crate::Wall;

/// Serializes as serde's `SystemTime` representation:
/// `{ "secs_since_epoch": u64, "nanos_since_epoch": u32 }`.
///
/// # Errors
///
/// Fails for readings before the Unix epoch, exactly as serializing a
/// `SystemTime` does.
///
/// # Examples
///
/// ```
/// use clock_lib::Wall;
///
/// let stamp = Wall::from_unix_millis(1_700_000_000_250).expect("in range");
/// let json = serde_json::to_string(&stamp).expect("post-epoch");
/// assert_eq!(json, r#"{"secs_since_epoch":1700000000,"nanos_since_epoch":250000000}"#);
/// ```
impl Serialize for Wall {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let since_epoch = self
            .epoch_offset()
            .map_err(|_| S::Error::custom("SystemTime must be later than UNIX_EPOCH"))?;
        let as_std = STD_UNIX_EPOCH
            .checked_add(since_epoch)
            .ok_or_else(|| S::Error::custom("Wall is out of range for std::time::SystemTime"))?;
        as_std.serialize(serializer)
    }
}

/// Deserializes serde's `SystemTime` representation.
///
/// # Errors
///
/// Fails if the input is not a valid `SystemTime` encoding, or if the
/// moment is not representable on this platform.
///
/// # Examples
///
/// ```
/// use clock_lib::Wall;
///
/// let json = r#"{"secs_since_epoch":86400,"nanos_since_epoch":0}"#;
/// let stamp: Wall = serde_json::from_str(json).expect("valid");
/// assert_eq!(stamp.to_string(), "1970-01-02T00:00:00Z");
/// ```
impl<'de> Deserialize<'de> for Wall {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let as_std = StdSystemTime::deserialize(deserializer)?;
        let since_epoch = as_std
            .duration_since(STD_UNIX_EPOCH)
            .map_err(|_| D::Error::custom("SystemTime must be later than UNIX_EPOCH"))?;
        Self::UNIX_EPOCH
            .checked_add(since_epoch)
            .ok_or_else(|| D::Error::custom("timestamp is out of range for this platform"))
    }
}
