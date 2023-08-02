#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui;
use eframe::egui::plot::Line;
use eframe::epaint::Color32;
use gui::gyro::{open, Port};

fn main() -> Result<(), eframe::Error> {
    // env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(800., 600.0)),
        ..Default::default()
    };
    eframe::run_native(
        "My egui App",
        options,
        Box::new(|_cc| Box::<MyApp>::default()),
    )
}

struct MyApp {
    port: Port,
    prev: [f64; 12],
    lines: Vec<Vec<[f64; 2]>>,
    x: f64,
    // name: String,
    // age: u32,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            lines: vec![],
            x: 0.,
            port: Port {
                rx: Some(open(std::path::Path::new("/dev/ttyUSB0"), 115200)),
                last_transmition: None,
            },
            prev: [0.; 12],
        }
    }
}

const COLOR: [Color32; 6] = [
    Color32::LIGHT_BLUE,
    Color32::BLUE,
    Color32::RED,
    Color32::GREEN,
    Color32::YELLOW,
    Color32::WHITE,
];

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let data = self.port.rx.as_ref().unwrap().recv().unwrap();
        for i in 0..6 {
            self.lines
                .push(vec![[self.x, self.prev[i]], [self.x + 1., data[i] as f64]]);
            self.prev[i] = data[i] as f64;
        }
        self.x += 1.;
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::plot::Plot::new("plot").show(ui, |plotui| {
                // plotui.line(Line::new(vec![[0., 0.], [1., 1.]]));
                for (line, color) in self.lines.iter().zip(COLOR.iter().cycle()) {
                    plotui.line(Line::new(line.clone()).color(color.clone()));
                }
            });
        });
        ctx.request_repaint();
    }
}
