// disable console on windows for release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::f32::consts::PI;
use std::io::Cursor;

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy::winit::WinitWindows;
use bevy::DefaultPlugins;
use bevy_egui::egui::{Color32, Frame, RichText};
use bevy_egui::{egui, EguiContexts, EguiPlugin};
// use bevy_infinite_grid::{InfiniteGrid, InfiniteGridBundle, InfiniteGridPlugin};
use bevy_obj::ObjPlugin;
use gui::gyro::{open, open_tcp, GyroComponent, GyroPlugin, Port};
use winit::window::Icon;

fn main() {
    // let (cmd_tx, drone_state_rx) = open(std::path::Path::new("/dev/ttyUSB0"), 57600);
    let (cmd_tx, drone_state_rx) = open_tcp("99.22.0.1:9922");

    App::new()
        .insert_resource(Port {
            rx: Some(drone_state_rx),
            // rx: Some(open_tcp("99.22.0.1:9922")),
            last_transmission: None,
        })
        // .insert_resource(Msaa::Off)
        // .insert_resource(ClearColor(
        //     Color::rgb(1., 0.4, 0.4),
        // ))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy game".to_string(), // ToDo
                resolution: (800., 600.).into(),
                canvas: Some("#bevy".to_owned()),
                ..default()
            }),
            ..default()
        }))
        // .add_plugins(InfiniteGridPlugin)
        .add_plugins(EguiPlugin)
        .add_plugins(ObjPlugin)
        .add_plugins(GyroPlugin)
        .add_systems(
            Startup,
            (set_window_icon, setup_camera, configure_visuals_system),
        )
        .add_systems(Update, (ui_example_system,))
        .run();
}

// Sets the icon on windows and X11
fn set_window_icon(
    windows: NonSend<WinitWindows>,
    primary_window: Query<Entity, With<PrimaryWindow>>,
) {
    let primary_entity = primary_window.single();
    let primary = windows.get_window(primary_entity).unwrap();
    let icon_buf = Cursor::new(include_bytes!(
        // "../build/macos/AppIcon.iconset/icon_256x256.png"
        "../build/mtuci_logo.png"
    ));
    if let Ok(image) = image::load(icon_buf, image::ImageFormat::Png) {
        let image = image.into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        let icon = Icon::from_rgba(rgba, width, height).unwrap();
        primary.set_window_icon(Some(icon));
    };
}

fn setup_camera(mut commands: Commands) {
    // commands.spawn(
    //     InfiniteGridBundle {
    //         grid: InfiniteGrid {
    //             // shadow_color: None,
    //             ..Default::default()
    //         },
    //         ..Default::default()
    //     },
    // );
    commands.spawn(PointLightBundle {
        // transform: Transform::from_xyz(5.0, 8.0, 2.0),
        transform: Transform::from_xyz(0.0, 12.0, 0.0),
        point_light: PointLight {
            intensity: 1600.0, // lumens - roughly a 100W non-halogen incandescent bulb
            color: Color::WHITE,
            shadows_enabled: false,
            ..default()
        },
        ..default()
    });
    commands.spawn(Camera3dBundle {
        // transform: Transform::from_xyz(0., 0., -15.).looking_at(Vec3::new(0., 0., 0.), Vec3::Y),
        transform: Transform::from_xyz(0., 0.0, 15.).looking_at(Vec3::new(0., 0., 0.), Vec3::Y),
        // transform: Transform::from_xyz(0., 15., 0.).looking_at(Vec3::new(0., 0., 0.), Vec3::X),
        ..default()
    });
}

fn configure_visuals_system(mut contexts: EguiContexts) {
    contexts.ctx_mut().set_visuals(egui::Visuals {
        window_rounding: 0.0.into(),
        ..Default::default()
    });
}

fn ui_example_system(mut contexts: EguiContexts, mut query: Query<&mut GyroComponent>) {
    let ctx = contexts.ctx_mut();
    let mut gyro = query.iter_mut().next().unwrap();

    egui::CentralPanel::default()
        .frame(Frame::default().fill(Color32::TRANSPARENT))
        .show(ctx, |ui| {
            // let rt = egui::widgets::Label::new(
            //     RichText::new("Accelerometer weight in camputations")
            //         .background_color(Color32::BLUE)
            //         .color(Color32::YELLOW)
            //         .size(15.),
            // );
            // ui.add(rt);
            // ui.add(egui::Slider::new(&mut gyro.acc_weight, 0.0..=1.0));
            // let to_deg = |x: f32| x * 180.0 / PI;
            // ui.label(format!("pitch: {:.0} deg", to_deg(gyro.pitch_roll.0)));
            // ui.label(format!("roll: {:.0} deg", to_deg(gyro.pitch_roll.1)));
        });
}
