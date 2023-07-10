use eframe::{
    egui::{self, Painter, Response},
    emath::{Align2, RectTransform},
    epaint::{Color32, ColorImage, FontId, PathShape, Pos2, Rect, Vec2},
};
use image::{io::Reader as ImageReader, ImageError};
use std::error::Error;

use super::{MapCell, RusFarmApp, BOARD_SIZE, CELL_SIZE_F};

impl RusFarmApp {
    pub(super) fn render_bg(
        &mut self,
        response: &Response,
        painter: &Painter,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let to_screen = egui::emath::RectTransform::from_to(
            Rect::from_min_size(Pos2::ZERO, response.rect.size()),
            response.rect,
        );

        for y in 0..BOARD_SIZE {
            for x in 0..BOARD_SIZE {
                match self.map[x + BOARD_SIZE * y] {
                    MapCell::Empty(weed) => {
                        let file_name = "assets/dirt.png";
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
                        if let Some(texture) =
                            try_insert_with(&mut self.weeds_img, "assets/weeds.png", painter)
                        {
                            let scr_rect = to_screen.transform_rect(Rect::from_min_size(
                                egui::pos2((x as f32) * CELL_SIZE_F, (y as f32) * CELL_SIZE_F),
                                Vec2::splat(CELL_SIZE_F),
                            ));
                            const MAX_WEED: f32 = 7.;
                            const DU: f32 = 1. / MAX_WEED;
                            let u = weed as f32 / MAX_WEED;
                            let tex_rect =
                                Rect::from_min_max(egui::pos2(u, 0.), egui::pos2(u + DU, 1.));
                            painter.image(texture.id(), scr_rect, tex_rect, Color32::WHITE)
                        }
                    }
                    MapCell::Wall => {
                        if let Some(texture) =
                            try_insert_with(&mut self.wall_img, "assets/wall.png", painter)
                        {
                            draw_wall(x, y, &self.map, painter, texture, &to_screen);
                        }
                    }
                };
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

        let font = FontId::proportional(18.);

        if let Some(texture) = try_insert_with(&mut self.hole_img, "assets/hole.png", painter) {
            for hole in self.holes.iter() {
                let rect = Rect::from_min_size(
                    (hole.pos.to_vec2() * CELL_SIZE_F).to_pos2(),
                    Vec2::splat(CELL_SIZE_F),
                );
                const UV: Rect = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
                painter.image(
                    texture.id(),
                    to_screen.transform_rect(rect),
                    UV,
                    Color32::WHITE,
                );
            }
        }

        if let Some(texture) = try_insert_with(&mut self.raccoon_img, "assets/raccoon.png", painter)
        {
            let size = texture.size_vec2();
            for raccoon in &self.raccoons {
                let state = raccoon.state.borrow();
                let min = state.pos.to_vec2() * CELL_SIZE_F;
                let max = min + size;
                let rect = Rect {
                    min: min.to_pos2(),
                    max: max.to_pos2(),
                };
                const UV: Rect = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
                painter.image(texture.id(), to_screen.transform_rect(rect), UV, state.tint);
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
}

#[allow(non_upper_case_globals)]
fn draw_wall(
    x: usize,
    y: usize,
    map: &[MapCell],
    painter: &Painter,
    texture: &egui::TextureHandle,
    to_screen: &RectTransform,
) {
    macro_rules! pos2 {
        ($x:expr, $y:expr) => {
            egui::pos2($x as f32 / 64., $y as f32 / 96.)
        };
    }

    const w00: Rect = Rect::from_min_max(pos2!(0, 32), pos2!(16, 48));
    const w10: Rect = Rect::from_min_max(pos2!(16, 32), pos2!(32, 48));
    const w01: Rect = Rect::from_min_max(pos2!(0, 48), pos2!(16, 64));
    const w11: Rect = Rect::from_min_max(pos2!(16, 48), pos2!(32, 64));
    const wl: Rect = Rect::from_min_max(pos2!(32, 64), pos2!(48, 80));
    const wul: Rect = Rect::from_min_max(pos2!(32, 0), pos2!(48, 16));
    const wu: Rect = Rect::from_min_max(pos2!(48, 64), pos2!(64, 80));
    const wr: Rect = Rect::from_min_max(pos2!(48, 80), pos2!(64, 96));
    const wur: Rect = Rect::from_min_max(pos2!(48, 0), pos2!(64, 16));
    const wul2: Rect = Rect::from_min_max(pos2!(48, 48), pos2!(64, 64));
    const wur2: Rect = Rect::from_min_max(pos2!(32, 48), pos2!(48, 64));
    const wd: Rect = Rect::from_min_max(pos2!(32, 80), pos2!(48, 96));
    const wdr: Rect = Rect::from_min_max(pos2!(48, 16), pos2!(64, 32));
    const wdr2: Rect = Rect::from_min_max(pos2!(32, 32), pos2!(48, 48));
    const wdl: Rect = Rect::from_min_max(pos2!(32, 16), pos2!(48, 32));
    const wdl2: Rect = Rect::from_min_max(pos2!(48, 32), pos2!(64, 48));

    let terrain = |x: isize, y: isize| {
        let x = x.max(0).min(BOARD_SIZE as isize - 1) as usize;
        let y = y.max(0).min(BOARD_SIZE as isize - 1) as usize;
        map[x + y * BOARD_SIZE]
    };

    let x = x as isize;
    let y = y as isize;
    let cul = terrain(x - 1, y - 1);
    let cu = terrain(x, y - 1);
    let cur = terrain(x + 1, y - 1);
    let cl = terrain(x - 1, y);
    let cr = terrain(x + 1, y);
    let cdl = terrain(x - 1, y + 1);
    let cd = terrain(x, y + 1);
    let cdr = terrain(x + 1, y + 1);

    let paint = |xofs: f32, yofs: f32, tex_rect| {
        painter.image(
            texture.id(),
            to_screen.transform_rect(Rect::from_min_size(
                egui::pos2(
                    (x as f32 + xofs) * CELL_SIZE_F,
                    (y as f32 + yofs) * CELL_SIZE_F,
                ),
                Vec2::splat(CELL_SIZE_F * 0.5),
            )),
            tex_rect,
            Color32::WHITE,
        )
    };

    let tex_rect = match (cu.is_wall(), cl.is_wall(), cul.is_wall()) {
        (true, true, true) => w00,
        (true, true, false) => wul2,
        (true, false, _) => wl,
        (false, false, _) => wul,
        (false, true, _) => wu,
    };
    paint(0., 0., tex_rect);

    let tex_rect = match (cu.is_wall(), cr.is_wall(), cur.is_wall()) {
        (true, true, true) => w10,
        (true, true, false) => wur2,
        (true, false, _) => wr,
        (false, false, _) => wur,
        (false, true, _) => wu,
    };
    paint(0.5, 0., tex_rect);

    let tex_rect = match (cd.is_wall(), cl.is_wall(), cdl.is_wall()) {
        (true, true, true) => w01,
        (true, true, false) => wdl2,
        (true, false, _) => wl,
        (false, false, _) => wdl,
        (false, true, _) => wd,
    };
    paint(0., 0.5, tex_rect);

    let tex_rect = match (cd.is_wall(), cr.is_wall(), cdr.is_wall()) {
        (true, true, true) => w11,
        (true, true, false) => wdr2,
        (true, false, _) => wr,
        (false, false, _) => wdr,
        (false, true, _) => wd,
    };
    paint(0.5, 0.5, tex_rect);
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
        "raccoon",
        color_image,
        egui::TextureOptions {
            magnification: egui::TextureFilter::Nearest,
            minification: egui::TextureFilter::Linear,
        },
    ))
}
