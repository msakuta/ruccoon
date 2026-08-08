use std::{cell::RefCell, rc::Rc};

use eframe::{
    egui::{Color32, Painter, Vec2},
    emath::RectTransform,
};

use crate::{app::CELL_SIZE_F, raccoon::RaccoonState};

const BULLET_SIZE_F: f32 = 6.;

pub struct Bullet {
    /// Position in cell coordinates
    pub pos: Vec2,
    pub velo: Vec2,
    pub owner: usize,
}

impl Bullet {
    pub fn render(&self, painter: &Painter, to_screen: &RectTransform) {
        let color = Color32::YELLOW;

        painter.circle_filled(
            to_screen.transform_pos((self.pos * CELL_SIZE_F).to_pos2()),
            BULLET_SIZE_F,
            color,
        );
    }

    /// Returns whether the bullet should be alive
    pub fn animate(&mut self, raccoons: &[Rc<RefCell<RaccoonState>>]) -> bool {
        self.pos += self.velo;

        for (i, raccoon) in raccoons.iter().enumerate() {
            if i != self.owner {
                let raccoon = raccoon.borrow();
                if raccoon.pos.distance_sq(self.pos.to_pos2())
                    < ((BULLET_SIZE_F + 1.) / CELL_SIZE_F).powi(2)
                {
                    return false;
                }
            }
        }

        !(self.pos.x < 0.
            || CELL_SIZE_F < self.pos.x
            || self.pos.y < 0.
            || CELL_SIZE_F < self.pos.y)
    }
}
