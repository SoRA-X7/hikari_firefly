use std::collections::{BinaryHeap, HashMap, HashSet};

use game::tetris::*;

#[derive(Clone, Copy, Hash, Eq)]
pub struct Step {
    piece: PieceState,
    parent: Option<PiecePosition>,
    cost: u8,
    depth: u8,
}

impl PartialEq for Step {
    fn eq(&self, other: &Self) -> bool {
        self.cost.eq(&other.cost)
    }
}

impl Ord for Step {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.cost.cmp(&other.cost)
    }
}

impl PartialOrd for Step {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

pub struct MoveGenerator {
    tree: HashMap<PiecePosition, Step>,
    next: BinaryHeap<Step>,
    locked: HashSet<PieceState>,
}

impl MoveGenerator {
    pub fn generate_for<B: Board>(state: &GameState<B>, use_hold: bool) -> Self {
        let mut gen = Self {
            tree: HashMap::new(),
            next: BinaryHeap::new(),
            locked: HashSet::new(),
        };

        let current = state.queue.pop_front().unwrap();

        let spawn = state.spawn_next();

        gen.generate_internal(state, current);

        if use_hold & state.hold.is_some() {
            state.gen.generate_internal(state, unhold);
        }

        gen
    }

    fn generate_internal<B: Board>(&mut self, state: &GameState<B>, spawn: PieceState) {
        while let Some(step) = self.next.pop() {
            let piece = step.piece;
            let parent = self.tree.get(&piece.pos);
            let dropped = state.sonic_drop(&piece);
            if let Some(dropped) = dropped {
                self.check_write(state.strafe(piece, -1), Instruction::Left);
                self.check_write(state.strafe(piece, 1), Instruction::Right);

                if piece.pos.kind != PieceKind::O {
                    self.check_write(state.rotate(piece, true), Instruction::Cw);
                    self.check_write(state.rotate(piece, false), Instruction::Ccw);
                }

                if dropped.pos.y != piece.pos.y {
                    self.check_write(Some(dropped), Instruction::SonicDrop);
                }
            }
        }
    }

    fn check_write(&mut self, parent: &Step, piece: Option<PieceState>, instruction: Instruction) {
        let Some(piece) = piece else {
            return;
        };

        piece
    }
}
