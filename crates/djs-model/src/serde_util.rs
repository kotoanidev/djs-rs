//! Small serde helpers shared by the model types.

/// `#[serde(default = "crate::serde_util::default_true")]` for booleans that are `true` when
/// absent (D-055).
pub(crate) const fn default_true() -> bool {
    true
}
