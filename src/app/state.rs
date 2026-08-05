use std::{cell::RefCell, rc::Rc};

use eframe::egui::Pos2;

use crate::{bullet::Bullet, raccoon::RaccoonState};

use super::{Hole, MapCell};

pub(crate) struct RaccoonAppState {
    pub raccoons: Vec<Rc<RefCell<RaccoonState>>>,
    pub map: Vec<MapCell>,
    pub items: Vec<Pos2>,
    pub holes: Vec<Hole>,
    pub bullets: Vec<Bullet>,
}
