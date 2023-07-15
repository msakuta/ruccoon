use eframe::{
    egui::Painter,
    emath::{Align2, RectTransform},
    epaint::{vec2, Color32, FontId, PathShape, Pos2, Rect, TextureHandle, Vec2},
};

use super::Raccoon;
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
        let min = state.pos.to_vec2() * CELL_SIZE_F;
        let max = min + size;
        let rect = Rect {
            min: min.to_pos2(),
            max: max.to_pos2(),
        };
        const UV: Rect = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
        painter.image(texture.id(), to_screen.transform_rect(rect), UV, state.tint);

        let bar_bg = Rect::from_min_size(
            (state.pos.to_vec2() * CELL_SIZE_F).to_pos2(),
            vec2(CELL_SIZE_F, 10.),
        );
        painter.rect_filled(
            to_screen.transform_rect(bar_bg),
            0.,
            Color32::from_rgb(31, 31, 31),
        );
        let bar_rect = Rect::from_min_size(
            (state.pos.to_vec2() * CELL_SIZE_F).to_pos2(),
            vec2(state.satiety * CELL_SIZE_F, 10.),
        );
        let bar_color = if state.satiety < 0.3 {
            Color32::RED
        } else if state.satiety < 0.6 {
            Color32::YELLOW
        } else {
            Color32::from_rgb(31, 255, 31)
        };
        painter.rect_filled(to_screen.transform_rect(bar_rect), 0., bar_color);

        painter.text(
            to_screen.transform_pos(rect.min),
            Align2::CENTER_TOP,
            state.ate,
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
