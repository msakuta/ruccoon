use eframe::{
    egui::{Color32, Painter, Rect, Vec2},
    emath::RectTransform,
};

use crate::app::CELL_SIZE_F;

const BULLET_SIZE_F: f32 = 12.;

pub struct Bullet {
    /// Position in cell coordinates
    pub pos: Vec2,
    pub velo: Vec2,
}

impl Bullet {
    pub fn render(&self, painter: &Painter, to_screen: &RectTransform) {
        let min = self.pos * CELL_SIZE_F - Vec2::new(0.5, 0.5) * BULLET_SIZE_F;
        let max = min + Vec2::splat(BULLET_SIZE_F);
        let rect = Rect {
            min: min.to_pos2(),
            max: max.to_pos2(),
        };
        // const UV: Rect = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
        // painter.image(texture.id(), to_screen.transform_rect(rect), UV, state.tint);

        let color = Color32::YELLOW;

        painter.rect_filled(to_screen.transform_rect(rect), 0., color);
    }
}
