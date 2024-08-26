use game::tetris::*;

use super::{Accumulator, Evaluator};

#[derive(Debug, Clone)]
pub struct Weights {
    no_tspin: bool,
    bump_sum: i32,
    bump_sum_sq: i32,
    well_depth: i32,
    well_x: [i32; 10],
    max_height_diff: i32,
    max_height: i32,
    top_50: i32,
    top_75: i32,
    cavities: i32,
    cavities_sq: i32,
    overhangs: i32,
    overhangs_sq: i32,
    covered_cells: i32,
    covered_cells_sq: i32,
    row_transitions: i32,
    downstack: i32,
    move_time: i32,
    danger: i32,
    b2b_continue: i32,
    b2b_destroy: i32,
    placement_height: i32,
    perfect: i32,
    ren: i32,
    clear1: i32,
    clear2: i32,
    clear3: i32,
    clear4: i32,
    t_mini1: i32,
    t_mini2: i32,
    t_spin1: i32,
    t_spin2: i32,
    t_spin3: i32,
    wasted_t: i32,
    hold_t: i32,
}

impl Default for Weights {
    fn default() -> Self {
        Self {
            clear1: -500,
            clear2: -375,
            clear3: -160,
            clear4: 800,
            t_spin1: 220,
            t_spin2: 1000,
            t_spin3: 1270,
            t_mini1: -880,
            t_mini2: -500,
            perfect: 3800,
            // t_hole: 200,
            // tst_hole: 180,
            // fin_hole: 150,
            b2b_continue: 340,
            b2b_destroy: -380,
            ren: 100,
            wasted_t: -300,
            hold_t: 10,
            bump_sum: 20,
            bump_sum_sq: -10,
            max_height_diff: 0,
            well_depth: 100,
            max_height: 20,
            top_50: -60,
            top_75: -300,
            danger: -20,
            placement_height: 0,
            move_time: -6,
            cavities: -400,
            cavities_sq: -10,
            overhangs: -170,
            overhangs_sq: -2,
            covered_cells: 0,
            covered_cells_sq: 0,
            row_transitions: -5,
            well_x: [150, -100, 300, 20, 60, 60, 15, 280, -110, 140],
            no_tspin: false,
            downstack: 0,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct StandardEvaluator {
    weights: Weights,
}

#[derive(Debug, Clone, Copy)]
pub struct Value {
    value: [i32; 3],
    spike: i32,
}

#[derive(Debug, Clone, Copy)]
pub struct Reward {
    eval: i32,
    attack: u32,
}

impl Default for Value {
    fn default() -> Self {
        Self {
            value: [0; 3],
            spike: 0,
        }
    }
}

impl Accumulator for Value {
    type Reward = Reward;

    fn accumulate(&self, next: Reward) -> Self {
        let mut value = self.value;
        value[2] += next.eval;
        let spike = if next.attack > 0 { self.spike + 1 } else { 0 };
        Value { value, spike }
    }

    fn select_score(&self) -> i32 {
        self.value.iter().sum()
    }
}

impl Evaluator for StandardEvaluator {
    type Accumulator = Value;
    type TransientReward = Reward;

    fn evaluate_state(&self, state: &GameState<BitBoard>) -> Self::Accumulator {
        puffin::profile_function!();
        let mut field_safety = 0;
        let mut field_power = 0;

        /*
        if !self.weights.no_tspin {
            let t_piece_count = (clone.bag.has(PieceKind::T) as i32)
                + (clone.hold == Some(PieceKind::T)) as i32
                + (clone.bag.len() <= 3) as i32;

            for _ in 0..t_piece_count {
                if let Some(t_hole) = self.scan_tspin(&clone, &max_heights, max_height) {
                    field_power += t_hole.lines * weights.t_hole;
                    self.after_spin(&mut clone, t_hole, piece_shapes);
                    clone.get_columns(&mut columns, &mut max_heights);
                    max_height -= t_hole.lines;
                    continue;
                }

                if let Some(tst) = self.scan_tst(&clone, &max_heights, max_height) {
                    field_power += tst.lines * weights.tst_hole;
                    self.after_spin(&mut clone, tst, piece_shapes);
                    clone.get_columns(&mut columns, &mut max_heights);
                    max_height -= tst.lines;
                    continue;
                }

                break;
            }
        }
        */

        let (well_column, well_depth) = calc_well_x_and_depth(&state.board);

        let (bump_sum, bump_sq_sum) = calc_bumpiness(&state.board, well_column);
        field_safety += bump_sum as i32 * self.weights.bump_sum;
        field_safety += bump_sq_sum as i32 * self.weights.bump_sum_sq;

        field_power += well_depth as i32 * self.weights.well_depth;
        if well_depth >= 2 {
            field_power += self.weights.well_x[well_column as usize];
        }

        let max_diff = (0..9)
            .filter(|&x| x != well_column)
            .map(|x| u32::abs_diff(state.board.height_of(x), state.board.height_of(x + 1)))
            .max()
            .unwrap();

        let max_height = (0..10).map(|x| state.board.height_of(x)).max().unwrap();

        field_safety += max_diff as i32 * self.weights.max_height_diff;
        field_safety += max_height as i32 * self.weights.max_height;
        field_safety += i32::max(max_height as i32 - 10, 0) * self.weights.top_50;
        field_safety += i32::max(max_height as i32 - 15, 0) * self.weights.top_75;

        let cav_ovh = cavities_and_overhangs(&state.board);
        field_safety += cav_ovh.0 * self.weights.cavities;
        field_safety += cav_ovh.0 * cav_ovh.0 * self.weights.cavities_sq;
        field_safety += cav_ovh.1 * self.weights.overhangs;
        field_safety += cav_ovh.1 * cav_ovh.1 * self.weights.overhangs_sq;

        let (covered, covered_sq) = covered_cells(&state.board);
        field_safety += covered as i32 * self.weights.covered_cells;
        field_safety += covered_sq as i32 * self.weights.covered_cells_sq;

        let transitions = (0..64)
            .map(|y| {
                let row = state.board.get_row(y);
                let lines = (row | 0b1_00000_00000) ^ (1 | row << 1);
                lines.count_ones() as i32
            })
            .sum::<i32>();

        field_safety += transitions * self.weights.row_transitions;
        field_power += (state.ren as f32).log2() as i32 * self.weights.downstack;

        Value {
            value: [field_safety, field_power, 0],
            spike: 0,
        }
    }

    fn evaluate_move(
        &self,
        mv: Move,
        placement: PlacementResult,
        state: &GameState<BitBoard>,
    ) -> Self::TransientReward {
        let prev_b2b = state.b2b;

        let mut move_score = 0;

        let mut time = 0;
        if let Move::Place(piece) = mv {
            // Locking delay
            if !placement.is_pc && placement.lines_cleared > 0 {
                time += 40;
            }
            move_score += time * self.weights.move_time;

            let max_danger_height = [0, 1, 2, 7, 8, 9]
                .iter()
                .map(|&x| state.board.height_of(x))
                .max()
                .unwrap();
            move_score += i32::max(max_danger_height as i32 - 15, 0) * time * self.weights.danger;

            if placement.lines_cleared > 0 && placement.is_b2b_clear {
                move_score += self.weights.b2b_continue;
            }
            if prev_b2b && !placement.is_b2b_clear {
                move_score += self.weights.b2b_destroy;
            }

            move_score += piece.pos.y as i32 * self.weights.placement_height;

            if placement.is_pc {
                move_score += self.weights.perfect;
            } else {
                let ren_attack = ren_attack(state.ren) as i32;
                move_score += self.weights.ren * ren_attack * ren_attack;
                move_score += match placement.spin {
                    SpinKind::None => match placement.lines_cleared {
                        1 => self.weights.clear1,
                        2 => self.weights.clear2,
                        3 => self.weights.clear3,
                        4 => self.weights.clear4,
                        _ => 0,
                    },
                    SpinKind::Full => match placement.lines_cleared {
                        1 => self.weights.t_spin1,
                        2 => self.weights.t_spin2,
                        3 => self.weights.t_spin3,
                        _ => 0,
                    },
                    SpinKind::Mini => match placement.lines_cleared {
                        1 => self.weights.t_mini1,
                        2 => self.weights.t_mini2,
                        _ => 0,
                    },
                };
            }

            if piece.pos.kind == PieceKind::T {
                if !(piece.spin == SpinKind::Full && placement.lines_cleared > 0) {
                    move_score += self.weights.wasted_t;
                }
            }
        }

        if state.hold == Some(PieceKind::T) {
            move_score += self.weights.hold_t;
        }

        Reward {
            eval: move_score,
            attack: placement.attack(),
        }
    }
}

#[inline(always)]
/// Returns the number of covered cells and the sum of squares of the number of covered cells.
/// A cell is considered covered if there is a block above it.
fn covered_cells(board: &BitBoard) -> (u32, u32) {
    let mut covered = 0;
    let mut sq = 0;

    for x in 0..10 {
        for y in (0..(board.height_of(x).max(3) - 2 - 1)).rev() {
            if board.occupied((x as i8, y as i8)) {
                continue;
            }
            let cells = board.height_of(x) - y - 1;
            covered += cells;
            sq += cells * cells;
        }
    }

    (covered, sq)
}

#[inline(always)]
/// Returns the well position and depth.
/// "Well" means the column with the lowest height.
/// Depth is the number of lines below the well that is ready to be cleared.
fn calc_well_x_and_depth(board: &BitBoard) -> (i8, u32) {
    let well = (1..10).min_by_key(|&x| board.height_of(x)).unwrap_or(0);

    let mut depth = 0;
    for x in 0..10 {
        let mut y = board.height_of(x) as i32 - 1;
        while y >= 0 && board.occupied((x as i8, y as i8)) {
            y -= 1;
        }
        if y >= 0 {
            depth += y;
        }
    }

    (well, depth as u32)
}

/// Returns the bumpiness and the sum of squares of the bumpiness.
/// Bumpiness is the sum of the absolute differences in height between adjacent columns excluding the well.
#[inline(always)]
fn calc_bumpiness(board: &BitBoard, well: i8) -> (u32, u32) {
    let mut bumpiness = 0;
    let mut bumpiness_sq = 0;

    let mut prev = 0;

    for x in 0..10i8 {
        if x == well {
            continue;
        }
        if prev >= 0 {
            let dh = u32::abs_diff(prev, board.height_of(x));
            bumpiness += dh;
            bumpiness_sq += dh * dh;
        }

        prev = board.height_of(x);
    }

    (bumpiness, bumpiness_sq)
}
/*
struct TSpinHole {
    x: i8,
    y: i8,
    rot: Rotation,
    lines: u8,
}

#[inline(always)]
fn scan_tspin(board: &BitBoard) -> Option<TSpinHole> {
    let best = None;
    for x in 0..10 {
        for y in (0..board.height_of(x) as i8).rev() {
            let pos = (x, y);
            board.
        }
    }
    best
}

#[inline(always)]
fn scan_tst(
    &self,
    board: &SimpleBoard,
    max_heights: &[u8],
    max_height: usize,
) -> Option<SimpleHole> {
    for y in (0..max_height).rev() {
        for x in 0..10 {
            if max_heights[x] as usize > y {
                continue;
            }
            if !board.tst_possible(x as i32, y as i32) {
                continue;
            }
            if board.tst(x as i32, y as i32).lines == 3 {
                return Some(SimpleHole { x, y, lines: 3 });
            }
        }
    }
    None
}

#[inline(always)]
fn after_spin(&self, board: &mut SimpleBoard, hole: SimpleHole, piece_shapes: &[u32x4x4]) {
    let piece = piece_shapes[PieceKind::T as usize];
    for x in 0..4 {
        for y in 0..4 {
            let tx = hole.x + x as usize;
            let ty = hole.y + y as usize;
            if piece.cells[y][x] && !board.occupied_unbounded(tx as usize, ty as i32) {
                board.clear(tx as usize, ty as i32);
            }
        }
    }
}
*/

/// Evaluates the holes in the playfield.
///
/// The first returned value is the number of cells that make up fully enclosed spaces (cavities).
/// The second is the number of cells that make up partially enclosed spaces (overhangs).
fn cavities_and_overhangs(board: &BitBoard) -> (i32, i32) {
    let mut cavities = 0;
    let mut overhangs = 0;

    for x in 0..10 {
        for y in 0..board.height_of(x) as i32 {
            if board.occupied((x, y as i8)) || y >= board.height_of(x) as i32 {
                continue;
            }

            if x > 1 {
                if board.height_of(x - 1) as i32 <= y - 1 && board.height_of(x - 2) as i32 <= y {
                    overhangs += 1;
                    continue;
                }
            }

            if x < 8 {
                if board.height_of(x + 1) as i32 <= y - 1 && board.height_of(x + 2) as i32 <= y {
                    overhangs += 1;
                    continue;
                }
            }

            cavities += 1;
        }
    }

    (cavities, overhangs)
}
