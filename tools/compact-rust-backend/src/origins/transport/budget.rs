//! Metadata transport containment, independent of Compact source admission.
//! Changes require separate analysis and measured qualification.
pub const MAX_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_ENTRIES: usize = 131_072;
pub const MAX_PENDING: usize = 4_096;
pub const MAX_PATH_CHARACTERS: usize = 262_144;
