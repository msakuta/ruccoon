use eframe::{
    egui::{Frame, Painter, Response},
    epaint::ColorImage,
};
use image::{io::Reader as ImageReader, ImageError};

use crate::bg_image::BgImage;

pub(crate) struct RusFarmApp {
    bg: BgImage,
}

impl RusFarmApp {
    pub fn new() -> Self {
        Self { bg: BgImage::new() }
    }

    fn render_bg(
        &mut self,
        response: &Response,
        painter: &Painter,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let file_name = "assets/dirt.png";
        for y in 0..16 {
            for x in 0..16 {
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
                    [x as f32 * 32., y as f32 * 32.],
                    1.,
                )?;
            }
        }
        Ok(())
    }
}

impl eframe::App for RusFarmApp {
    fn update(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame) {
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
