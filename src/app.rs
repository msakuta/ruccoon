mod render_bg;

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use eframe::{
    egui::{self, Frame},
    epaint::{pos2, Pos2},
};

use rand::Rng;
use ruscal::{parse_args, Args};

use crate::{
    bg_image::BgImage,
    raccoon::{compile_program, Raccoon},
};

pub(crate) const CELL_SIZE: usize = 64;
pub(crate) const CELL_SIZE_F: f32 = CELL_SIZE as f32;
pub(crate) const BOARD_SIZE: usize = 12;
pub(crate) const BOARD_SIZE_I: i32 = BOARD_SIZE as i32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MapCell {
    Wall,
    Empty(u8),
}

impl MapCell {
    fn is_wall(&self) -> bool {
        matches!(self, Self::Wall)
    }
}

pub(crate) struct Hole {
    pub pos: Pos2,
    pub occupied: Cell<bool>,
}

pub(crate) struct RusFarmApp {
    bg: BgImage,
    weeds_img: Option<egui::TextureHandle>,
    wall_img: Option<egui::TextureHandle>,
    map: Rc<Vec<MapCell>>,
    raccoon_img: Option<egui::TextureHandle>,
    raccoons: Vec<Raccoon>,
    corn_img: Option<egui::TextureHandle>,
    items: Rc<RefCell<Vec<Pos2>>>,
    hole_img: Option<egui::TextureHandle>,
    holes: Rc<Vec<Hole>>,
    last_animate: Option<std::time::Instant>,
    paused: bool,
}

impl RusFarmApp {
    pub fn new() -> Self {
        let args = parse_args(true).unwrap_or_else(|| {
            let mut args = Args::new();
            args.source = Some("scripts/raccoon.rscl".to_string());
            args
        });

        let mut map = vec![MapCell::Empty(0); BOARD_SIZE * BOARD_SIZE];
        let mut rng = rand::thread_rng();
        for i in 0..BOARD_SIZE {
            for j in 0..BOARD_SIZE {
                map[i + BOARD_SIZE * j] = if rng.gen::<f32>() < 0.25 {
                    MapCell::Wall
                } else {
                    MapCell::Empty(rng.gen_range(0..7))
                };
            }
        }
        let holes = Rc::new(
            (0..2)
                .map(|_| Hole {
                    pos: generate_pos(|pos| is_blocked(pos, &map, &[])),
                    occupied: Cell::new(false),
                })
                .collect(),
        );
        let map = Rc::new(map);

        let bytecode = match compile_program(&args) {
            Ok(bytecode) => bytecode,
            Err(e) => panic!("Compile error: {e}"),
        };
        let program = Rc::new(bytecode);
        let items = Rc::new(RefCell::new(vec![]));
        Self {
            bg: BgImage::new(),
            weeds_img: None,
            wall_img: None,
            map: map.clone(),
            raccoon_img: None,
            raccoons: (0..2)
                .map(|i| Raccoon::new(i, &map, &items, &holes, &program, args.debug_output))
                .collect(),
            corn_img: None,
            items,
            hole_img: None,
            holes,
            last_animate: None,
            paused: false,
        }
    }

    fn animate(&mut self) {
        if !self.paused {
            for raccoon in &self.raccoons {
                raccoon.animate(&self.raccoons, &self.map, &self.items, &self.holes);
            }
            // self.paused = true;
        }

        let mut rng = rand::thread_rng();
        if self.items.borrow().len() < 10 && rng.gen::<f64>() < 0.1 {
            let pos = generate_pos(|pos| is_blocked(pos, &self.map, &self.items.borrow()));
            let mut items = self.items.borrow_mut();
            if items.iter().all(|item| *item != pos) {
                items.push(pos);
            }
        }
    }
}

impl eframe::App for RusFarmApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
        let now = std::time::Instant::now();
        if !self
            .last_animate
            .is_some_and(|time| !(std::time::Duration::from_millis(100) < now - time))
        {
            self.animate();
            self.last_animate = Some(now);
        }
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            Frame::canvas(ui.style()).show(ui, |ui| {
                let (response, painter) =
                    ui.allocate_painter(ui.available_size(), eframe::egui::Sense::hover());
                let res = self.render_bg(&response, &painter);
                if let Err(res) = res {
                    eprintln!("image rendering error: {res}");
                }
            });
        });
    }
}

fn is_blocked(pos: Pos2, map: &[MapCell], items: &[Pos2]) -> bool {
    if !matches!(
        map[pos.x as usize + pos.y as usize * BOARD_SIZE],
        MapCell::Empty(_)
    ) {
        return true;
    }
    if items.iter().any(|item| *item == pos) {
        return true;
    }
    false
}

fn generate_pos(blocked: impl Fn(Pos2) -> bool) -> Pos2 {
    let mut rng = rand::thread_rng();
    loop {
        let pos = pos2(
            rng.gen_range(0..BOARD_SIZE) as f32,
            rng.gen_range(0..BOARD_SIZE) as f32,
        );
        if !blocked(pos) {
            return pos;
        }
    }
}
