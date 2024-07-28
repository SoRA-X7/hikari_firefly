use eframe::{
    egui::{Color32, Pos2, Rect, Shape, Stroke, Ui},
    epaint::RectShape,
};

pub fn render_tetris(ui: &mut Ui) {
    const N: usize = 10;
    let d = 20.0;

    let mut shapes = vec![];
    for i in 0..N {
        for j in 0..N {
            let pos1 = Pos2 {
                y: i as f32 * d,
                x: j as f32 * d,
            };
            let pos2 = Pos2 {
                y: pos1.y + d,
                x: pos1.x + d,
            };
            if (i + j) % 2 == 0 {
                shapes.push(Shape::Rect(RectShape::filled(
                    Rect::from_two_pos(pos1, pos2),
                    4.0,
                    Color32::RED,
                )));
            } else {
                shapes.push(Shape::Rect(RectShape::filled(
                    Rect::from_two_pos(pos1, pos2),
                    4.0,
                    Color32::BLUE,
                )));
            }
        }
    }

    ui.painter().extend(shapes.into_iter());
}
