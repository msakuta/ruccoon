mod app;
mod bg_image;
mod rascal;

use app::RusFarmApp;

fn main() {
    // Log to stdout (if you run with `RUST_LOG=debug`).
    // tracing_subscriber::fmt::init();

    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "rusfarm",
        native_options,
        Box::new(|_cc| Box::new(RusFarmApp::new())),
    )
    .unwrap();

    // let train = [[0., 0., 0.], [0., 1., 1.], [1., 0., 1.], [1., 1., 1.]];
    // let train = [[0., 0., 0.], [0., 1., 0.], [1., 0., 0.], [1., 1., 1.]];
    // learn(&train);
}
