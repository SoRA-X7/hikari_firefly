use eframe::{
    egui::{Color32, Pos2, Rect, Shape, Stroke, Ui},
    epaint::RectShape,
};
use game::tetris::{CellKind, ColoredBoard, GameState, PieceKind};

const fn cell_color(c: CellKind) -> Color32 {
    // https://flatuicolors.com/palette/de
    match c {
        CellKind::None => Color32::BLACK,
        CellKind::I => Color32::from_rgb(69, 170, 242),
        CellKind::O => Color32::from_rgb(255, 211, 42),
        CellKind::T => Color32::from_rgb(165, 94, 234),
        CellKind::S => Color32::from_rgb(38, 222, 129),
        CellKind::Z => Color32::from_rgb(252, 92, 101),
        CellKind::L => Color32::from_rgb(253, 150, 68),
        CellKind::J => Color32::from_rgb(75, 123, 236),
        CellKind::Gbg => Color32::from_rgb(165, 177, 194),
    }
}

pub fn render_tetris(ui: &mut Ui, state: &GameState<ColoredBoard>) {
    let d = 20.0;

    let mut shapes = vec![];
    for x in 0..10 {
        let col = &state.board.cols[x];
        for y in 0..24 {
            let cell = col[y];

            let pos1 = Pos2 {
                y: x as f32 * d,
                x: y as f32 * d,
            };
            let pos2 = Pos2 {
                y: pos1.y + d,
                x: pos1.x + d,
            };

            if cell != CellKind::None {
                shapes.push(Shape::Rect(RectShape::filled(
                    Rect::from_two_pos(pos1, pos2),
                    4.0,
                    cell_color(cell),
                )));
            }
        }
    }

    ui.painter().extend(shapes.into_iter());
}
