use eframe::{
    egui::{self, Frame, Painter, Response},
    epaint::{pos2, Color32, ColorImage, Pos2, Rect, Vec2},
};
use image::{io::Reader as ImageReader, ImageError};
use rand::{seq::SliceRandom, Rng};

use crate::bg_image::BgImage;

const CELL_SIZE: usize = 64;
const CELL_SIZE_F: f32 = CELL_SIZE as f32;
const BOARD_SIZE: usize = 12;

struct Rascal {
    pos: Pos2,
    tint: Color32,
}

impl Rascal {
    fn new() -> Self {
        let mut rng = rand::thread_rng();
        Self {
            pos: pos2(
                rng.gen_range(0..BOARD_SIZE) as f32,
                rng.gen_range(0..BOARD_SIZE) as f32,
            ),
            tint: Color32::from_rgb(rng.gen(), rng.gen(), rng.gen()),
        }
    }
}

pub(crate) struct RusFarmApp {
    bg: BgImage,
    rascal_img: Option<eframe::egui::TextureHandle>,
    rascals: Vec<Rascal>,
}

impl RusFarmApp {
    pub fn new() -> Self {
        Self {
            bg: BgImage::new(),
            rascal_img: None,
            rascals: (0..2).map(|_| Rascal::new()).collect(),
        }
    }

    fn render_bg(
        &mut self,
        response: &Response,
        painter: &Painter,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let file_name = "assets/dirt.png";
        for y in 0..BOARD_SIZE {
            for x in 0..BOARD_SIZE {
                self.bg.paint(
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
        if self.rascal_img.is_none() {
            let file_name = "assets/rascal.png";
            self.rascal_img = Some({
                let img = ImageReader::open(file_name)?.decode()?.into_rgba8();
                let width = img.width();
                let height = img.height();
                let data: Vec<_> = img.to_vec();
                let color_image = egui::ColorImage::from_rgba_unmultiplied(
                    [width as usize, height as usize],
                    &data,
                );
                painter.ctx().load_texture(
                    "rascal",
                    color_image,
                    egui::TextureOptions {
                        magnification: egui::TextureFilter::Nearest,
                        minification: egui::TextureFilter::Linear,
                    },
                )
            });
        }

        if let Some(texture) = self.rascal_img.as_ref() {
            let to_screen = egui::emath::RectTransform::from_to(
                Rect::from_min_size(Pos2::ZERO, response.rect.size()),
                response.rect,
            );
            let size = texture.size_vec2();
            for rascal in &self.rascals {
                let min = rascal.pos.to_vec2() * CELL_SIZE_F;
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
                    rascal.tint,
                );
            }
        }
        Ok(())
    }

    fn animate(&mut self) {
        const DIRECTIONS: [Vec2; 4] = [
            Vec2::new(-1., 0.),
            Vec2::new(0., -1.),
            Vec2::new(1., 0.),
            Vec2::new(0., 1.),
        ];
        for rascal in &mut self.rascals {
            if let Some(direction) = DIRECTIONS.choose(&mut rand::thread_rng()) {
                rascal.pos += *direction;
                if rascal.pos.x < 0. {
                    rascal.pos.x = 0.;
                } else if BOARD_SIZE as f32 <= rascal.pos.x {
                    rascal.pos.x = BOARD_SIZE as f32;
                }
                if rascal.pos.y < 0. {
                    rascal.pos.y = 0.;
                } else if BOARD_SIZE as f32 <= rascal.pos.y {
                    rascal.pos.y = BOARD_SIZE as f32;
                }
            }
        }
    }
}

impl eframe::App for RusFarmApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint();
        self.animate();
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
