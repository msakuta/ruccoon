use std::{cell::RefCell, collections::HashMap, rc::Rc};

use eframe::egui::Pos2;

use crate::{bullet::Bullet, raccoon::RaccoonState};

use super::{BOARD_SIZE, Hole, MapCell};

pub(crate) type RaccoonStates = HashMap<usize, Rc<RefCell<RaccoonState>>>;

pub(crate) struct RaccoonAppState {
    pub raccoons: RaccoonStates,
    pub map: Vec<MapCell>,
    pub items: Vec<Pos2>,
    pub holes: Vec<Hole>,
    pub bullets: Vec<Bullet>,
}

impl RaccoonAppState {
    /// Check if a new Raccoon can be inserted at given position, optionally ignoring an existing id.
    pub fn is_blocked(&self, this_id: Option<usize>, pos: Pos2) -> bool {
        if !matches!(
            self.map[pos.x.round() as usize + pos.y.round() as usize * BOARD_SIZE],
            MapCell::Empty(_)
        ) {
            return true;
        }
        if self.raccoons.iter().any(|(i, other)| {
            if this_id != Some(*i) {
                return false;
            }
            let Ok(other_state) = other.try_borrow() else {
                return false;
            };
            other_state.pos.distance_sq(pos) < 1.
        }) {
            return true;
        }
        false
    }
}
