use std::collections::{BinaryHeap, HashMap, HashSet};

use game::tetris::*;

const MAX_DEPTH: u8 = 32;
const MAX_NON_T_COST: u8 = 20;

#[derive(Clone, Copy, Hash, Eq)]
pub struct Step {
    piece: PieceState,
    parent: Option<PiecePosition>,
    instruction: Option<Instruction>,
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

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub struct Placement([(i8, i8); 4], SpinKind);

pub struct MoveGenerator {
    // BFS Tree for known step tracking and later path lookup
    tree: HashMap<PiecePosition, Step>,
    // Priority queue
    next: BinaryHeap<Step>,
    // Deduplicated lockable positions
    locked: HashMap<Placement, (PieceState, PiecePosition)>,
}

impl MoveGenerator {
    pub fn generate_for<B: Board>(state: &GameState<B>, use_hold: bool) -> Result<Self, ()> {
        let mut gen = Self {
            tree: HashMap::new(),
            next: BinaryHeap::new(),
            locked: HashMap::new(),
        };

        let current = state.queue.pop_front().unwrap();

        let mut state = state.clone();

        let spawn = state.spawn_next().ok_or(())?;
        gen.generate_internal(&state, spawn);

        if use_hold & state.hold.is_some() {
            let hold = state.spawn_hold(spawn.pos.kind);
            if let Some(hold) = hold {
                gen.generate_internal(&state, hold);
            }
        }

        Ok(gen)
    }

    fn generate_internal<B: Board>(&mut self, state: &GameState<B>, spawn: PieceState) {
        // Conduct a Breadth-first search
        while let Some(step) = self.next.pop() {
            let piece = step.piece;
            let parent = self.tree.get(&piece.pos).unwrap();
            let dropped = state.sonic_drop(&piece);
            if let Some(dropped) = dropped {
                self.check_write(state, parent, state.strafe(piece, -1), Instruction::Left);
                self.check_write(state, parent, state.strafe(piece, 1), Instruction::Right);

                if piece.pos.kind != PieceKind::O {
                    self.check_write(state, parent, state.rotate(piece, true), Instruction::Cw);
                    self.check_write(state, parent, state.rotate(piece, false), Instruction::Ccw);
                }

                if dropped.pos.y != piece.pos.y {
                    self.check_write(state, parent, Some(dropped), Instruction::SonicDrop);
                }

                let placement = Placement(dropped.pos.cells(), dropped.spin);

                self.locked.entry(placement).or_insert((dropped, piece.pos));
            }
        }
    }

    fn check_write<B: Board>(
        &mut self,
        state: &GameState<B>,
        parent: &Step,
        piece: Option<PieceState>,
        instruction: Instruction,
    ) {
        let Some(piece) = piece else { return };
        if state.board.collides(piece.pos) {
            return;
        };

        let cost = if instruction == Instruction::SonicDrop {
            3 * (parent.piece.pos.y - piece.pos.y) as u8
        } else if parent.instruction == Some(instruction) {
            2
        } else {
            1
        };

        let step = Step {
            piece,
            parent: Some(parent.piece.pos),
            instruction: Some(instruction),
            cost: parent.cost + cost,
            depth: parent.depth + 1,
        };

        if self.tree.get(&piece.pos) == None && step.depth < MAX_DEPTH {
            if piece.pos.kind == PieceKind::T || cost < MAX_NON_T_COST {
                // Let's Continue BFS
                self.tree.insert(piece.pos, step);
            }
        }
    }
}
