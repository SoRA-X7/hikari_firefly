use std::{
    collections::VecDeque,
    fmt::{self, Display},
};

use enumset::{EnumSet, EnumSetType};
use movegen::MoveGenerator;
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};

mod movegen;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Instruction {
    None,
    Left,
    Right,
    Cw,
    Ccw,
    SonicDrop,
}

#[derive(Debug, Hash, PartialOrd, Ord, Serialize, Deserialize, EnumSetType)]
pub enum PieceKind {
    S,
    Z,
    J,
    L,
    T,
    O,
    I,
}

impl fmt::Display for PieceKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl PieceKind {
    pub const fn cells(&self) -> [(i8, i8); 4] {
        match self {
            Self::S => [(-1, 0), (0, 0), (0, 1), (1, 1)],
            Self::Z => [(-1, 1), (0, 0), (0, 1), (1, 0)],
            Self::J => [(-1, 0), (-1, 1), (0, 0), (1, 0)],
            Self::L => [(-1, 0), (0, 0), (1, 0), (1, 1)],
            Self::T => [(-1, 0), (0, 0), (0, 1), (1, 0)],
            Self::O => [(0, 0), (1, 0), (0, 1), (1, 1)],
            Self::I => [(-1, 0), (0, 0), (1, 0), (2, 0)],
        }
    }

    pub const fn rotation_offsets(&self, rotation: Rotation) -> [(i8, i8); 5] {
        match self {
            Self::O => match rotation {
                Rotation::North => [(0, 0); 5],
                Rotation::East => [(0, -1); 5],
                Rotation::South => [(-1, -1); 5],
                Rotation::West => [(-1, 0); 5],
            },
            Self::I => match rotation {
                Rotation::North => [(0, 0), (-1, 0), (2, 0), (-1, 0), (2, 0)],
                Rotation::East => [(-1, 0), (0, 0), (0, 0), (0, 1), (0, -2)],
                Rotation::South => [(-1, 1), (1, 1), (-2, 1), (1, 0), (-2, 0)],
                Rotation::West => [(0, 1), (0, 1), (0, 1), (0, -1), (0, 2)],
            },
            _ => match rotation {
                Rotation::North => [(0, 0); 5],
                Rotation::East => [(0, 0), (1, 0), (1, -1), (0, 2), (1, 2)],
                Rotation::South => [(0, 0); 5],
                Rotation::West => [(0, 0), (-1, 0), (-1, -1), (0, 2), (-1, 2)],
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum CellKind {
    None,
    S,
    Z,
    J,
    L,
    T,
    O,
    I,
    Gbg,
}

impl From<PieceKind> for CellKind {
    fn from(value: PieceKind) -> Self {
        match value {
            PieceKind::S => Self::S,
            PieceKind::Z => Self::Z,
            PieceKind::J => Self::J,
            PieceKind::L => Self::L,
            PieceKind::T => Self::T,
            PieceKind::O => Self::O,
            PieceKind::I => Self::I,
        }
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
pub enum Rotation {
    North,
    East,
    South,
    West,
}

impl Rotation {
    pub const fn rotate_cell(&self, (x, y): (i8, i8)) -> (i8, i8) {
        match self {
            Rotation::North => (x, y),
            Rotation::East => (y, -x),
            Rotation::South => (-x, y),
            Rotation::West => (-y, x),
        }
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
pub struct PiecePosition {
    #[serde(rename = "type")]
    pub kind: PieceKind,
    pub x: i8,
    pub y: i8,
    #[serde(rename = "orientation")]
    pub rot: Rotation,
}

impl PiecePosition {
    pub fn cells(&self) -> [(i8, i8); 4] {
        self.kind.cells().map(|(x, y)| {
            let (x, y) = self.rot.rotate_cell((x, y));
            let x = x + self.x;
            let y = y + self.y;
            (x, y)
        })
    }
    pub fn translate(&self, delta: (i8, i8)) -> Self {
        let mut this = self.clone();
        this.x += delta.0;
        this.y += delta.1;
        this
    }
}

impl Display for PiecePosition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}@({},{})`{:?}", self.kind, self.x, self.y, self.rot)
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
pub enum SpinKind {
    None,
    Mini,
    Full,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
pub struct PieceState {
    #[serde(rename = "location")]
    pub pos: PiecePosition,
    pub spin: SpinKind,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Move {
    Hold, // hold_only
    Place(PieceState),
}

impl PieceState {
    pub const fn new(kind: PieceKind, (x, y): (i8, i8), rot: Rotation) -> Self {
        PieceState {
            pos: PiecePosition { kind, x, y, rot },
            spin: SpinKind::None,
        }
    }

    pub fn translate(self, delta: (i8, i8)) -> Self {
        PieceState {
            pos: self.pos.translate(delta),
            spin: self.spin,
        }
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct PlacementResult {
    pub lines_cleared: u32,
    pub ren: i32,
    pub spin: SpinKind,
    pub is_b2b_clear: bool,
    pub is_pc: bool,
    pub death: bool,
}

impl Default for PlacementResult {
    fn default() -> Self {
        Self {
            lines_cleared: 0,
            ren: -1,
            spin: SpinKind::None,
            is_b2b_clear: false,
            is_pc: false,
            death: false,
        }
    }
}

impl PlacementResult {
    pub fn attack(&self) -> u32 {
        if self.lines_cleared == 0 {
            0
        } else if self.is_pc {
            10
        } else {
            let base = match self.spin {
                SpinKind::None | SpinKind::Mini => match self.lines_cleared {
                    1 => 0,
                    2 => 1,
                    3 => 2,
                    4 => 4,
                    _ => 0,
                },
                SpinKind::Full => 2 * self.lines_cleared,
            };
            let b2b = if self.is_b2b_clear { 1 } else { 0 };
            base + b2b + ren_attack(self.ren)
        }
    }
}

/// A 7-bag implementation as per guideline.
/// If the bag is full, the internal set must be empty.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct SevenBag(pub EnumSet<PieceKind>);

impl SevenBag {
    pub fn has(&self, piece: PieceKind) -> bool {
        self.0.is_empty() || self.0.contains(piece)
    }

    pub fn take(&mut self, piece: PieceKind) {
        if self.0.is_empty() {
            self.0 = EnumSet::all();
        }
        assert!(self.0.contains(piece));
        self.0.remove(piece);
    }

    pub fn take_rand(&mut self) -> PieceKind {
        if self.0.is_empty() {
            self.0 = EnumSet::all();
        }
        let idx = thread_rng().gen_range(0..self.0.len());
        let piece = self.0.iter().skip(idx).next().unwrap();
        self.0.remove(piece);
        piece
    }

    pub fn put(&mut self, piece: PieceKind) {
        assert!(!self.0.contains(piece));
        self.0.insert(piece);

        // the set must be empty if this bag is full
        if self.0.len() == 7 {
            self.0.clear();
        }
    }
}

impl Default for SevenBag {
    fn default() -> Self {
        Self(EnumSet::empty())
    }
}

pub trait Board: Clone + Default {
    fn occupied(&self, pos: (i8, i8)) -> bool;
    fn height_of(&self, x: i8) -> u32;
    fn is_empty(&self) -> bool;
    fn distance_to_ground(&self, pos: (i8, i8)) -> u32;
    fn add_piece_and_clear(&mut self, piece: PieceState) -> u32;

    fn collides(&self, piece: PiecePosition) -> bool {
        piece
            .cells()
            .iter()
            .any(|(x, y)| *x < 0 || 10 <= *x || *y < 0 || 64 <= *y || self.occupied((*x, *y)))
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Default, Deserialize)]
#[serde(from = "Vec<[Option<char>; 10]>")]
pub struct BitBoard {
    pub cols: [u64; 10],
}

impl From<Vec<[Option<char>; 10]>> for BitBoard {
    fn from(v: Vec<[Option<char>; 10]>) -> Self {
        let mut cols = [0; 10];
        for x in 0..10 {
            for y in 0..40 {
                if v[y][x].is_some() {
                    cols[x] |= 1 << y;
                }
            }
        }
        BitBoard { cols }
    }
}

impl Board for BitBoard {
    fn occupied(&self, (x, y): (i8, i8)) -> bool {
        x < 0 || 10 <= x || y < 0 || 64 <= y || self.cols[x as usize] & (1u64 << y) > 0
    }

    fn is_empty(&self) -> bool {
        self.cols.iter().fold(0, |acc, col| acc | col) == 0
    }

    fn distance_to_ground(&self, (x, y): (i8, i8)) -> u32 {
        debug_assert!(0 <= x && x < 10);
        debug_assert!(0 <= y && y < u64::BITS as i8);
        if y == 0 {
            0
        } else {
            (!self.cols[x as usize] << (u64::BITS as i8 - y)).leading_ones()
        }
    }

    fn add_piece_and_clear(&mut self, piece: PieceState) -> u32 {
        piece.pos.cells().iter().for_each(|(x, y)| {
            self.cols[*x as usize] ^= 1u64 << y;
        });
        let cleared = self.cols.iter().fold(u64::MAX, |acc, col| acc & col);
        self.cols
            .iter_mut()
            .for_each(|col| clear_lines(col, cleared));
        cleared.count_ones()
    }

    fn height_of(&self, x: i8) -> u32 {
        debug_assert!(0 <= x && x < 10);
        u64::BITS - self.cols[x as usize].leading_zeros()
    }
}

impl BitBoard {
    pub fn get_row(&self, y: i8) -> u64 {
        self.cols
            .iter()
            .enumerate()
            .fold(0, |acc, (x, col)| acc | (col >> y & 1) << x)
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ColoredBoard {
    pub cols: [[CellKind; 64]; 10],
}

impl Default for ColoredBoard {
    fn default() -> Self {
        Self {
            cols: [[CellKind::None; 64]; 10],
        }
    }
}

impl Board for ColoredBoard {
    fn occupied(&self, (x, y): (i8, i8)) -> bool {
        x < 0 || 10 <= x || y < 0 || 64 <= y || self.cols[x as usize][y as usize] != CellKind::None
    }

    fn is_empty(&self) -> bool {
        self.cols
            .iter()
            .all(|col| col.iter().all(|cell| *cell == CellKind::None))
    }

    fn distance_to_ground(&self, (x, y): (i8, i8)) -> u32 {
        self.cols[x as usize]
            .iter()
            .rev()
            .skip((u64::BITS - (y as u32)) as usize)
            .take_while(|&c| *c == CellKind::None)
            .count() as u32
    }

    fn height_of(&self, x: i8) -> u32 {
        64 - (self.cols[x as usize]
            .iter()
            .rev()
            .take_while(|&c| *c == CellKind::None)
            .count()) as u32
    }

    fn add_piece_and_clear(&mut self, piece: PieceState) -> u32 {
        piece.pos.cells().iter().for_each(|(x, y)| {
            self.cols[*x as usize][*y as usize] = piece.pos.kind.into();
        });
        let mut cleared = vec![];
        for y in 0..64 {
            if self.cols.iter().all(|col| col[y] != CellKind::None) {
                cleared.push(y as u32);
            }
        }

        let mut offset = 0;
        for clear in &cleared {
            for y in (clear - offset)..63 {
                for x in 0..10 {
                    self.cols[y as usize][x] = self.cols[(y + 1) as usize][x];
                }
            }
            for x in 0..10 {
                self.cols[63][x] = CellKind::None;
            }
            offset += 1;
        }
        cleared.len() as u32
    }
}

impl From<Vec<[Option<char>; 10]>> for ColoredBoard {
    fn from(v: Vec<[Option<char>; 10]>) -> Self {
        let mut cols = [[CellKind::None; u64::BITS as usize]; 10];
        for x in 0..10 {
            for y in 0..40 {
                cols[x][y] = v[y][x].map_or(CellKind::None, |c| match c {
                    'S' => CellKind::S,
                    'Z' => CellKind::Z,
                    'J' => CellKind::J,
                    'L' => CellKind::L,
                    'T' => CellKind::T,
                    'O' => CellKind::O,
                    'I' => CellKind::I,
                    'G' => CellKind::Gbg,
                    _ => CellKind::None,
                });
            }
        }
        ColoredBoard { cols }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct GameState<B: Board> {
    pub board: B,
    pub hold: Option<PieceKind>,
    pub queue: VecDeque<PieceKind>,
    pub bag: SevenBag,
    pub b2b: bool,
    pub ren: i32, // defaults to -1, first clear is 0
}

impl<B: Board> GameState<B> {
    pub fn new() -> Self {
        Self {
            board: Default::default(),
            hold: None,
            queue: Default::default(),
            bag: Default::default(),
            b2b: false,
            ren: -1,
        }
    }

    pub fn fulfill_queue(&mut self) -> PieceKind {
        let p = self.bag.take_rand();
        self.queue.push_back(p);
        p
    }

    pub fn spawn_next(&mut self) -> Option<PieceState> {
        let kind = self.queue.pop_front().unwrap();
        self.spawn(kind)
    }

    pub fn spawn_hold(&mut self, current: PieceKind) -> Option<PieceState> {
        debug_assert!(self.hold.is_some());
        let unhold = self.hold.take().unwrap();
        self.hold = Some(current);
        self.spawn(unhold)
    }

    fn spawn(&self, kind: PieceKind) -> Option<PieceState> {
        let mut p = PieceState::new(kind, (4, 19), Rotation::North);
        if self.board.collides(p.pos) {
            p = PieceState::new(kind, (4, 20), Rotation::North);
            if self.board.collides(p.pos) {
                return None;
            }
        }
        Some(p)
    }

    pub fn strafe(&self, piece: PieceState, (dx, dy): (i8, i8)) -> Option<PieceState> {
        let mut piece = piece;
        piece.pos.x += dx;
        piece.pos.y += dy;
        if self.board.collides(piece.pos) {
            None
        } else {
            Some(piece)
        }
    }

    pub fn rotate(&self, piece: PieceState, clockwise: bool) -> Option<PieceState> {
        let from = piece.pos.rot;
        let to = if clockwise {
            match from {
                Rotation::North => Rotation::East,
                Rotation::East => Rotation::South,
                Rotation::South => Rotation::West,
                Rotation::West => Rotation::North,
            }
        } else {
            match from {
                Rotation::North => Rotation::West,
                Rotation::East => Rotation::North,
                Rotation::South => Rotation::East,
                Rotation::West => Rotation::South,
            }
        };

        let from_offsets = piece.pos.kind.rotation_offsets(from);
        let to_offsets = piece.pos.kind.rotation_offsets(to);

        for (i, kick) in from_offsets.iter().zip(to_offsets).enumerate() {
            let offset = (kick.0 .0 - kick.1 .0, kick.0 .1 - kick.1 .1);
            let target = PiecePosition {
                x: piece.pos.x + offset.0,
                y: piece.pos.y + offset.1,
                rot: to,
                ..piece.pos
            };
            // println!("{:?} {:?} {:?}", piece.pos, clockwise, target);
            if !self.board.collides(target) {
                let spin;
                if piece.pos.kind != PieceKind::T {
                    spin = SpinKind::None;
                } else {
                    let corners = [(-1, -1), (1, -1), (-1, 1), (1, 1)]
                        .iter()
                        .filter(|&&(cx, cy)| self.board.occupied((cx + target.x, cy + target.y)))
                        .count();
                    let mini_corners = [(-1, 1), (1, 1)]
                        .iter()
                        .map(|&c| target.rot.rotate_cell(c))
                        .filter(|&(cx, cy)| self.board.occupied((cx + target.x, cy + target.y)))
                        .count();

                    if corners < 3 {
                        spin = SpinKind::None;
                    } else if mini_corners == 2 || i == 4 {
                        spin = SpinKind::Full;
                    } else {
                        spin = SpinKind::Mini;
                    }
                }
                return Some(PieceState { pos: target, spin });
            }
        }
        None
    }

    pub fn sonic_drop(&self, piece: &PieceState) -> Option<PieceState> {
        let distance = piece
            .pos
            .cells()
            .iter()
            .map(|pos| self.board.distance_to_ground(*pos))
            .min()? as i8;
        let spin = if distance == 0 {
            piece.spin
        } else {
            SpinKind::None
        };

        Some(PieceState {
            pos: PiecePosition {
                y: piece.pos.y - distance,
                ..piece.pos
            },
            spin,
        })
    }

    pub fn is_grounded(&self, piece: &PieceState) -> bool {
        self.sonic_drop(piece)
            .map_or(false, |dropped| dropped.pos == piece.pos)
    }

    pub fn place_piece(&mut self, piece: PieceState) -> PlacementResult {
        let death = piece.pos.cells().iter().all(|(_, y)| *y >= 20);
        let lines_cleared = self.board.add_piece_and_clear(piece);
        let is_pc = self.board.is_empty();
        let is_b2b = lines_cleared == 4 || (lines_cleared > 0 && piece.spin != SpinKind::None);
        let is_b2b_clear = self.b2b && is_b2b;
        if lines_cleared > 0 {
            self.ren += 1;
            self.b2b = is_b2b
        } else {
            self.ren = -1;
        }
        PlacementResult {
            lines_cleared,
            is_b2b_clear,
            is_pc,
            ren: self.ren,
            spin: piece.spin,
            death,
        }
    }

    pub fn advance(&mut self, mv: Move) -> PlacementResult {
        let mut current = self.queue.pop_front().expect("queue must not be empty");
        match mv {
            Move::Hold => {
                debug_assert_eq!(self.hold, None);
                self.hold = Some(current);
                PlacementResult::default()
            }
            Move::Place(piece) => {
                if piece.pos.kind != current {
                    let _old = current.clone();
                    current = self.hold.take().expect("hold must not be empty");
                    self.hold = Some(_old);
                }
                debug_assert_eq!(current, piece.pos.kind);
                self.place_piece(piece)
            }
        }
    }

    pub fn add_piece(&mut self, piece: PieceKind) {
        // println!("{:?} / {:?}", self.bag.0, piece);
        debug_assert!(self.bag.has(piece));
        self.bag.take(piece);
        self.queue.push_back(piece);
    }

    pub fn legal_moves(&self, use_hold: bool) -> Result<Vec<Move>, ()> {
        let gen = MoveGenerator::generate_for(self, use_hold)?;
        Ok(gen.moves())
    }
}

#[cfg(all(target_arch = "x86_64", target_feature = "bmi2"))]
fn clear_lines(col: &mut u64, lines: u64) {
    *col = unsafe {
        // SAFETY: #[cfg()] guard ensures that this instruction exists at compile time
        std::arch::x86_64::_pext_u64(*col, !lines)
    };
}

#[cfg(not(all(target_arch = "x86_64", target_feature = "bmi2")))]
fn clear_lines(col: &mut u64, mut lines: u64) {
    while lines != 0 {
        let i = lines.trailing_zeros();
        let mask = (1 << i) - 1;
        *col = *col & mask | *col >> 1 & !mask;
        lines &= !(1 << i);
        lines >>= 1;
    }
}

/// Returns the number of lines cleared by a combo.
pub fn ren_attack(ren: i32) -> u32 {
    const COMBO_ATTACK: [u32; 12] = [0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 4, 5];
    if ren < 0 {
        0
    } else {
        *COMBO_ATTACK.get(ren as usize).unwrap_or(&5)
    }
}

#[macro_export]
macro_rules! bit_board {
    ($($row:expr),*) => {
        BitBoard {
            cols: [0,1,2,3,4,5,6,7,8,9].map(|x| {
                let mut col = 0u64;
                let mut y = 0;
                $(
                    col |= match $row.chars().nth(x as usize).unwrap() {
                        'x' => 1 << y,
                        _ => 0
                    };
                    y += 1;
                )*
                col.reverse_bits() >> (64 - y)
            })
        }
    };
}

#[cfg(test)]
mod test {
    pub use super::*;

    #[test]
    fn test_bit_board_macro() {
        let expected = BitBoard {
            cols: [
                0b1111, 0b1111, 0b1111, 0b1011, 0b0001, 0b0000, 0b0000, 0b1111, 0b1111, 0b1111,
            ],
        };
        let actual = bit_board! {
            "xxxx___xxx",
            "xxx____xxx",
            "xxxx___xxx",
            "xxxxx__xxx"
        };

        assert_eq!(expected, actual);
    }
}
