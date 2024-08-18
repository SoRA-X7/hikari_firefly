use std::{
    cmp::Reverse,
    collections::{BinaryHeap, HashMap, HashSet},
};

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
    next: BinaryHeap<Reverse<Step>>,
    // Deduplicated lockable positions
    pub locked: HashMap<Placement, (PieceState, PiecePosition)>,
    hold_only_move: bool,
}

impl MoveGenerator {
    pub fn generate_for<B: Board>(state: &GameState<B>, use_hold: bool) -> Result<Self, ()> {
        let mut gen = Self {
            tree: HashMap::new(),
            next: BinaryHeap::new(),
            locked: HashMap::new(),
            hold_only_move: false,
        };

        let mut state = state.clone();

        let spawn = state.spawn_next().ok_or(())?;
        // println!("spawn {:?}", spawn);
        gen.generate_internal(&state, spawn);

        if use_hold {
            if state.hold.is_some() {
                let hold = state.spawn_hold(spawn.pos.kind);
                if let Some(hold) = hold {
                    if hold.pos.kind != spawn.pos.kind {
                        gen.generate_internal(&state, hold);
                    }
                }
            } else {
                gen.hold_only_move = true;
            }
        }

        Ok(gen)
    }

    fn generate_internal<B: Board>(&mut self, state: &GameState<B>, spawn: PieceState) {
        let root_step = Step {
            parent: None,
            piece: spawn,
            instruction: None,
            cost: 0,
            depth: 0,
        };

        self.tree.insert(spawn.pos, root_step);
        self.next.push(Reverse(root_step));

        // Conduct a Breadth-first search
        while let Some(step) = self.next.pop() {
            // println!("take {:?}", &step.0.piece);
            let piece = step.0.piece;
            let parent = *self.tree.get(&piece.pos).unwrap();
            let dropped = state.sonic_drop(&piece);
            if let Some(dropped) = dropped {
                self.check_write(
                    state,
                    &parent,
                    state.strafe(piece, (-1, 0)),
                    Instruction::Left,
                );
                self.check_write(
                    state,
                    &parent,
                    state.strafe(piece, (1, 0)),
                    Instruction::Right,
                );

                if piece.pos.kind != PieceKind::O {
                    self.check_write(state, &parent, state.rotate(piece, true), Instruction::Cw);
                    self.check_write(state, &parent, state.rotate(piece, false), Instruction::Ccw);
                }

                if dropped.pos.y != piece.pos.y {
                    self.check_write(state, &parent, Some(dropped), Instruction::SonicDrop);
                }

                let mut cells = dropped.pos.cells();
                cells.sort();
                let placement = Placement(cells, dropped.spin);

                self.locked.entry(placement).or_insert((dropped, piece.pos));
            }
        }
        // println!("{}", self.locked.len());
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
                // Continue BFS
                self.tree.insert(piece.pos, step);
                self.next.push(Reverse(step));
            }
        }
    }

    pub fn moves(&self) -> Vec<Move> {
        let mut vec = self
            .locked
            .values()
            .map(|(piece, _)| Move::Place(*piece))
            .collect::<Vec<_>>();
        if self.hold_only_move {
            vec.push(Move::Hold);
        }
        vec
    }
}
