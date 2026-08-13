use eframe::{
    egui::Painter,
    emath::{Align2, RectTransform},
    epaint::{Color32, FontId, PathShape, Pos2, Rect, TextureHandle, Vec2, vec2},
};

use super::{MAX_HEALTH, Raccoon};
use crate::app::CELL_SIZE_F;

impl Raccoon {
    pub fn render(
        &self,
        painter: &Painter,
        texture: &TextureHandle,
        size: Vec2,
        to_screen: &RectTransform,
        font: FontId,
    ) {
        let state = self.state.borrow();
        let min = state.pos.to_vec2() * CELL_SIZE_F - size * 0.5;
        let max = min + size;
        let rect = Rect {
            min: min.to_pos2(),
            max: max.to_pos2(),
        };
        const UV: Rect = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
        painter.image(texture.id(), to_screen.transform_rect(rect), UV, state.tint);

        let bar_color = if state.satiety < 0.3 {
            Color32::RED
        } else if state.satiety < 0.6 {
            Color32::YELLOW
        } else {
            Color32::from_rgb(31, 255, 31)
        };
        render_bar(min, state.satiety, bar_color, painter, to_screen);

        render_bar(
            min + Vec2::Y * 5.,
            state.health / MAX_HEALTH,
            Color32::GREEN,
            painter,
            to_screen,
        );

        painter.text(
            to_screen.transform_pos(rect.min),
            Align2::CENTER_TOP,
            self.id,
            font.clone(),
            Color32::WHITE,
        );

        if let Some(path) = &state.path {
            let plot: Vec<_> = path
                .iter()
                .map(|node| to_screen.transform_pos(Pos2::from(node)))
                .collect();
            painter.add(PathShape::line(plot, (3., state.tint)));
        }
    }
}

fn render_bar(pos: Vec2, f: f32, color: Color32, painter: &Painter, to_screen: &RectTransform) {
    let bar_min = pos.to_pos2();
    let bar_bg = Rect::from_min_size(bar_min, vec2(CELL_SIZE_F, 5.));

    painter.rect_filled(
        to_screen.transform_rect(bar_bg),
        0.,
        Color32::from_rgb(31, 31, 31),
    );
    let bar_rect = Rect::from_min_size(bar_min, vec2(f * CELL_SIZE_F, 5.));

    painter.rect_filled(to_screen.transform_rect(bar_rect), 0., color);
}
