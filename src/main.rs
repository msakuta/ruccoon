mod app;
mod bg_image;
mod rascal;

use app::{RusFarmApp, BOARD_SIZE, CELL_SIZE};
use eframe::epaint::vec2;

fn main() {
    let mut native_options = eframe::NativeOptions::default();
    native_options.initial_window_size = Some(vec2(
        (BOARD_SIZE * CELL_SIZE + 16) as f32,
        (BOARD_SIZE * CELL_SIZE + 16) as f32,
    ));
    eframe::run_native(
        "rusfarm",
        native_options,
        Box::new(|_cc| Box::new(RusFarmApp::new())),
    )
    .unwrap();
}
