use super::*;
use std::{
    cmp::Reverse,
    collections::{BinaryHeap, HashMap},
};

const MAX_DEPTH: u16 = 32;
const MAX_NON_T_COST: u16 = 20;

#[derive(Clone, Copy, Hash, Eq)]
pub struct Step {
    piece: PieceState,
    parent: Option<PiecePosition>,
    instruction: Option<Instruction>,
    cost: u16,
    depth: u16,
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

#[derive(Clone, Hash, PartialEq, Eq)]
pub enum Path {
    HoldOnly,
    Normal {
        hold: bool,
        cost: u16,
        instructions: Vec<Instruction>,
        piece: PieceState,
    },
}

pub struct MoveGenerator {
    original_piece: PieceKind,
    // BFS Tree for known step tracking and later path lookup
    tree: HashMap<PiecePosition, Step>,
    // Priority queue
    next: BinaryHeap<Reverse<Step>>,
    // Deduplicated lockable positions
    locked: HashMap<Placement, (PieceState, PiecePosition, u16)>,
    hold_only_move: bool,
}

impl MoveGenerator {
    pub fn generate_for<B: Board>(state: &GameState<B>, use_hold: bool) -> Result<Self, ()> {
        let mut state = state.clone();

        let spawn = state.spawn_next().ok_or(())?;

        let mut gen = Self {
            original_piece: spawn.pos.kind,
            tree: HashMap::new(),
            next: BinaryHeap::new(),
            locked: HashMap::new(),
            hold_only_move: false,
        };
        // eprintln!("spawn {:?}", spawn);
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
            // eprintln!("take {:?}", &step.0.piece);
            let piece = step.0.piece;
            let parent = *self.tree.get(&piece.pos).unwrap();
            let dropped = state.sonic_drop(piece);
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

                self.locked
                    .entry(placement)
                    .or_insert((dropped, piece.pos, step.0.cost));
            }
        }
        // eprintln!("{}", self.locked.len());
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
            3 * (parent.piece.pos.y - piece.pos.y) as u16
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
            .map(|(piece, _, _)| Move::Place(*piece))
            .collect::<Vec<_>>();
        if self.hold_only_move {
            vec.push(Move::Hold);
        }
        vec
    }

    pub fn moves_with_cost(&self) -> Vec<(Move, u16)> {
        let mut vec = self
            .locked
            .values()
            .map(|(piece, _, cost)| (Move::Place(*piece), *cost))
            .collect::<Vec<_>>();
        if self.hold_only_move {
            vec.push((Move::Hold, 0));
        }
        vec
    }

    pub fn rebuild_path(&self, mv: Move) -> Path {
        match mv {
            Move::Hold => Path::HoldOnly,
            Move::Place(piece) => {
                let mut instructions = Vec::new();
                let mut piece = piece;
                let mut parent = self.tree[&piece.pos].parent;
                while let Some(p) = parent {
                    let step = self.tree[&p];
                    instructions.push(step.instruction.unwrap());
                    piece = step.piece;
                    parent = step.parent;
                }
                instructions.reverse();
                Path::Normal {
                    hold: piece.pos.kind != self.original_piece,
                    cost: self.tree[&piece.pos].cost,
                    instructions,
                    piece,
                }
            }
        }
    }
}
