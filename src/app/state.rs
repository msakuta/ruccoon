use eframe::egui::Pos2;

use super::{Hole, MapCell};

pub(crate) struct RaccoonAppState {
    pub map: Vec<MapCell>,
    pub items: Vec<Pos2>,
    pub holes: Vec<Hole>,
}
