use std::fmt;

#[derive(PartialEq, Eq, Clone, Debug, Default)]
pub enum PieceType {
    // movable pieve on the board
    Board,
    // covered piece on the board
    Covered,
    // not your turn
    Inactive,
    // 
    #[default]
    Nope,
    // piece in reserve
    Reserve,
    // a not yet spawned piece on a spawn point
    Spawn,
}

impl fmt::Display for PieceType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = match self {
            PieceType::Board => "board",
            PieceType::Covered => "covered",
            PieceType::Inactive => "inactive",
            PieceType::Nope => "nope",
            PieceType::Reserve => "reserve",
            PieceType::Spawn => "spawn",
        };
        write!(f, "{}", name)
    }
}