use game::tetris::*;
use serde::{ser::SerializeSeq, Deserialize, Serialize};
use smallvec::SmallVec;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct PieceIdentity {
    cells: [(i8, i8); 4],
    spin: SpinKind,
}

impl From<PieceState> for PieceIdentity {
    fn from(piece: PieceState) -> Self {
        let mut cells = piece.pos.cells();
        // make sure the cells are always placed in the same order for the canonical representation
        // in order to Eq and Hash work correctly
        cells.sort();
        Self {
            cells,
            spin: piece.spin,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Replay {
    player_id: u32,
    frame: u64,
    state: ReplayState,
    action: PieceIdentity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayState {
    #[serde(serialize_with = "serialize_board")]
    board: BitBoard,
    current: PieceKind,
    unhold: PieceKind,
    queue: SmallVec<[PieceKind; 18]>,
    hold: Option<PieceKind>,
    ren: i8,
    b2b: bool,
    bag: SmallVec<[PieceKind; 7]>,
}

fn serialize_board<S>(board: &BitBoard, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let mut cols = serializer.serialize_seq(Some(10))?;
    for x in 0..10 {
        let cells: Vec<bool> = (0..64).map(|y| board.occupied((x, y as i8))).collect();
        cols.serialize_element(&cells)?;
    }
    cols.end()
}
