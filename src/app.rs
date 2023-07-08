use std::{cell::RefCell, error::Error, rc::Rc};

use eframe::{
    egui::{self, Frame, Painter, Response},
    epaint::{pos2, Color32, ColorImage, PathShape, Pos2, Rect},
};
use image::{io::Reader as ImageReader, ImageError};

use rand::Rng;
use ruscal::{parse_args, Args};

use crate::{
    bg_image::BgImage,
    rascal::{compile_program, Rascal},
};

pub(crate) const CELL_SIZE: usize = 64;
pub(crate) const CELL_SIZE_F: f32 = CELL_SIZE as f32;
pub(crate) const BOARD_SIZE: usize = 12;
pub(crate) const BOARD_SIZE_I: i32 = BOARD_SIZE as i32;

#[derive(Clone, Copy, Debug)]
pub(crate) enum MapCell {
    Wall,
    Empty,
}

pub(crate) struct RusFarmApp {
    bg: BgImage,
    bg2: BgImage,
    map: Rc<Vec<MapCell>>,
    rascal_img: Option<egui::TextureHandle>,
    rascals: Vec<Rascal>,
    corn_img: Option<egui::TextureHandle>,
    items: Rc<RefCell<Vec<Pos2>>>,
    last_animate: Option<std::time::Instant>,
}

impl RusFarmApp {
    pub fn new() -> Self {
        let args = parse_args(true).unwrap_or_else(|| {
            let mut args = Args::new();
            args.source = Some("scripts/rascal.txt".to_string());
            args
        });

        let mut map = vec![MapCell::Empty; BOARD_SIZE * BOARD_SIZE];
        for i in 0..BOARD_SIZE {
            for j in 0..BOARD_SIZE {
                map[i + BOARD_SIZE * j] = if rand::random::<f32>() < 0.25 {
                    MapCell::Wall
                } else {
                    MapCell::Empty
                };
            }
        }
        let map = Rc::new(map);

        let bytecode = match compile_program(&args) {
            Ok(bytecode) => bytecode,
            Err(e) => panic!("Compile error: {e}"),
        };
        let program = Rc::new(bytecode);
        let items = Rc::new(RefCell::new(vec![]));
        Self {
            bg: BgImage::new(),
            bg2: BgImage::new(),
            map: map.clone(),
            rascal_img: None,
            rascals: (0..2)
                .map(|i| Rascal::new(i, &map, &items, &program))
                .collect(),
            corn_img: None,
            items,
            last_animate: None,
        }
    }

    fn render_bg(
        &mut self,
        response: &Response,
        painter: &Painter,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for y in 0..BOARD_SIZE {
            for x in 0..BOARD_SIZE {
                let (bg, file_name) = match self.map[x + BOARD_SIZE * y] {
                    MapCell::Empty => (&mut self.bg, "assets/dirt.png"),
                    MapCell::Wall => (&mut self.bg2, "assets/wall.png"),
                };
                bg.paint(
                    &response,
                    &painter,
                    (),
                    |_| -> Result<ColorImage, ImageError> {
                        let img = ImageReader::open(file_name)?.decode()?.into_rgb8();
                        let width = img.width();
                        let height = img.height();
                        let data: Vec<_> = img.to_vec();
                        Ok(eframe::egui::ColorImage::from_rgb(
                            [width as usize, height as usize],
                            &data,
                        ))
                    },
                    [x as f32 * CELL_SIZE_F, y as f32 * CELL_SIZE_F],
                    CELL_SIZE_F / 32.,
                )?;
            }
        }

        fn try_insert_with<'a>(
            target: &'a mut Option<egui::TextureHandle>,
            file_name: &str,
            painter: &Painter,
        ) -> Option<&'a egui::TextureHandle> {
            if target.is_none() {
                *target = match try_load_image(file_name, painter) {
                    Ok(res) => Some(res),
                    Err(e) => {
                        eprintln!("try_load_image({}) failed: {e}", file_name);
                        None
                    }
                };
            }
            target.as_ref()
        }

        let to_screen = egui::emath::RectTransform::from_to(
            Rect::from_min_size(Pos2::ZERO, response.rect.size()),
            response.rect,
        );

        if let Some(texture) = try_insert_with(&mut self.rascal_img, "assets/rascal.png", painter) {
            let size = texture.size_vec2();
            for rascal in &self.rascals {
                let state = rascal.state.borrow();
                let min = state.pos.to_vec2() * CELL_SIZE_F;
                let max = min + size;
                let rect = Rect {
                    min: min.to_pos2(),
                    max: max.to_pos2(),
                };
                const UV: Rect = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
                painter.image(texture.id(), to_screen.transform_rect(rect), UV, state.tint);

                if let Some(path) = &state.path {
                    let plot: Vec<_> = path
                        .iter()
                        .map(|node| to_screen.transform_pos(Pos2::from(node)))
                        .collect();
                    painter.add(PathShape::line(plot, (3., state.tint)));
                }
            }
        }

        if let Some(texture) = try_insert_with(&mut self.corn_img, "assets/corn.png", painter) {
            let size = texture.size_vec2();
            for item in self.items.borrow().iter() {
                let min = item.to_vec2() * CELL_SIZE_F;
                let max = min + size;
                let rect = Rect {
                    min: min.to_pos2(),
                    max: max.to_pos2(),
                };
                const UV: Rect = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
                painter.image(
                    texture.id(),
                    to_screen.transform_rect(rect),
                    UV,
                    Color32::WHITE,
                );
            }
        }

        Ok(())
    }

    fn animate(&mut self) {
        for rascal in &self.rascals {
            rascal.animate(&self.rascals, &self.map, &self.items);
        }

        let mut rng = rand::thread_rng();
        if self.items.borrow().len() < 10 && rng.gen::<f64>() < 0.1 {
            let pos = loop {
                let pos = pos2(
                    rng.gen_range(0..BOARD_SIZE) as f32,
                    rng.gen_range(0..BOARD_SIZE) as f32,
                );
                if !is_blocked(pos, &self.map, &self.items.borrow()) {
                    break pos;
                }
            };
            let mut items = self.items.borrow_mut();
            if items.iter().all(|item| *item != pos) {
                items.push(pos);
            }
        }
    }
}

impl eframe::App for RusFarmApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after(std::time::Duration::from_secs(1));
        let now = std::time::Instant::now();
        if !self
            .last_animate
            .is_some_and(|time| !(std::time::Duration::from_secs(1) < now - time))
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

fn try_load_image(
    file_name: &str,
    painter: &Painter,
) -> Result<egui::TextureHandle, Box<dyn Error>> {
    let img = ImageReader::open(file_name)?.decode()?.into_rgba8();
    let width = img.width();
    let height = img.height();
    let data: Vec<_> = img.to_vec();
    let color_image =
        egui::ColorImage::from_rgba_unmultiplied([width as usize, height as usize], &data);
    Ok(painter.ctx().load_texture(
        "rascal",
        color_image,
        egui::TextureOptions {
            magnification: egui::TextureFilter::Nearest,
            minification: egui::TextureFilter::Linear,
        },
    ))
}

fn is_blocked(pos: Pos2, map: &[MapCell], items: &[Pos2]) -> bool {
    if !matches!(
        map[pos.x as usize + pos.y as usize * BOARD_SIZE],
        MapCell::Empty
    ) {
        return true;
    }
    if items.iter().any(|item| *item == pos) {
        return true;
    }
    false
}
