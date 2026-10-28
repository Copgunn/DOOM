/// Original dirtype_t order, including the non-moving sentinel.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum MoveDirection {
    /// Positive X.
    East = 0,
    /// Positive X and Y, using source component 47000.
    NorthEast = 1,
    /// Positive Y.
    North = 2,
    /// Negative X, positive Y.
    NorthWest = 3,
    /// Negative X.
    West = 4,
    /// Negative X and Y.
    SouthWest = 5,
    /// Negative Y.
    South = 6,
    /// Positive X, negative Y.
    SouthEast = 7,
    /// Source DI_NODIR: P_Move returns false without a collision query.
    NoDirection = 8,
}
impl MoveDirection {
    /// Validate a source direction without indexing the movement table first.
    pub const fn from_raw(value: u8) -> Option<Self> {
        Some(match value {
            0 => Self::East,
            1 => Self::NorthEast,
            2 => Self::North,
            3 => Self::NorthWest,
            4 => Self::West,
            5 => Self::SouthWest,
            6 => Self::South,
            7 => Self::SouthEast,
            8 => Self::NoDirection,
            _ => return None,
        })
    }
    /// Exact source components in raw 16.16 units; no vector normalization.
    pub const fn components(self) -> [i32; 2] {
        match self {
            Self::NoDirection => [0, 0],
            _ => VECTORS[self as usize],
        }
    }
    /// Source opposite lookup, including NODIR mapping to itself.
    pub const fn opposite(self) -> Self {
        match self {
            Self::East => Self::West,
            Self::NorthEast => Self::SouthWest,
            Self::North => Self::South,
            Self::NorthWest => Self::SouthEast,
            Self::West => Self::East,
            Self::SouthWest => Self::NorthEast,
            Self::South => Self::North,
            Self::SouthEast => Self::NorthWest,
            Self::NoDirection => Self::NoDirection,
        }
    }
}
const VECTORS: [[i32; 2]; 8] = [
    [65536, 0],
    [47000, 47000],
    [0, 65536],
    [-47000, 47000],
    [-65536, 0],
    [-47000, -47000],
    [0, -65536],
    [47000, -47000],
];
