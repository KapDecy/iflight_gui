use std::f32::consts::{FRAC_PI_2, PI, TAU};
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::mem::size_of;
use std::process::exit;
use std::time::Instant;
use bevy::core::{Pod, Zeroable};

use bevy::math::Vec3Swizzles;
use bevy::prelude::*;
use crossbeam_channel::{Receiver, Sender};

#[derive(Resource)]
pub struct Port {
    pub rx: Option<Receiver<DroneState>>,
    pub last_transmission: Option<Instant>,
}

const DELIMITER: u8 = 255;

#[derive(Clone, Debug)]
pub enum DroneCmd {
    CalibrateAccel,
    CalibrateGyro,
}

// 24 bytes
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
pub struct ImuState {
    pub gyro: [f32; 3],
    pub accel: [f32; 3],
}

// 136 bytes
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
pub struct DroneState {
    pub imu1: ImuState,
    pub imu2: ImuState,
    pub cal_imu1: ImuState,
    pub cal_imu2: ImuState,
    pub main_imu: ImuState,
    pub orientation: [f32; 4], // unit quaternion
}


fn run_communication<S>(mut port: S) -> (Sender<DroneCmd>, Receiver<DroneState>) where S: Read + Write + Send + 'static {
    let (drone_state_tx, drone_state_rx) = crossbeam_channel::bounded(1);
    let (cmd_tx, cmd_rx) = crossbeam_channel::bounded(1);

    println!("Running communication thread...");
    std::thread::spawn(move || {
        let mut buf = [0u8; 1];
        loop {
            println!("get current drone state...");
            if !cmd_rx.is_empty() {
                let cmd = cmd_rx.recv().unwrap();
                match cmd {
                    DroneCmd::CalibrateAccel => {
                        port.write(&[1]).unwrap();
                    }
                    DroneCmd::CalibrateGyro => {
                        port.write(&[2]).unwrap();
                    }
                }
            }


            println!("Send GetState");
            port.write_all(&[3, 2]).unwrap();
            println!("cmd sent.");
            match port.read_exact(&mut buf) {
                Ok(()) => {
                    let drone_state: &DroneState = bytemuck::from_bytes(&buf);
                    drone_state_tx.try_send(drone_state.clone());
                    // drone_state_tx.send(drone_state.clone()).unwrap();
                }
                Err(e) => {
                    if !matches!(e.kind(), std::io::ErrorKind::TimedOut) {
                        exit(1);
                    } else {
                        error!("{:?}", e);
                    }
                }
            }
        }
    });

    (cmd_tx, drone_state_rx)
}

pub fn open(port_path: &std::path::Path, baudrate: u32) -> (Sender<DroneCmd>, Receiver<DroneState>) {
    info!("opening uart");
    let mut port = serialport::new(port_path.to_string_lossy(), baudrate)
        .timeout(std::time::Duration::from_secs(2))
        .open_native();

    while port.is_err() {
        info!("port in err. Retry");
        port = serialport::new(port_path.to_string_lossy(), baudrate)
            .timeout(std::time::Duration::from_secs(2))
            .open_native();
    }

    run_communication(port.unwrap())
}


pub fn open_tcp(addr: &str) -> (Sender<DroneCmd>, Receiver<DroneState>) {
    info!("Opening tcp stream...");
    let stream = std::net::TcpStream::connect(addr).unwrap();
    run_communication(stream)
}

pub struct GyroPlugin;

#[derive(Component)]
pub struct GyroComponent {
    pub variant: DroneVariant,
}

#[derive(Component)]
pub enum DroneVariant {
    Gyro,
    Acc,
    Both,
}

impl Plugin for GyroPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, gyro_spawn)
            .add_systems(Update, gyro_update);
    }
}

// const INIT_ACC_WEIGHT: f32 = 0.;
// const INIT_ACC_WEIGHT: f32 = 0.08;
const INIT_ACC_WEIGHT: f32 = 1.;

fn drone_pbr_bundle(asset_server: &AssetServer, materials: &mut Assets<StandardMaterial>, offset: Vec3, color: Color) -> PbrBundle {
    PbrBundle {
        mesh: asset_server.load("Drone2.obj"),
        material: materials.add(StandardMaterial {
            base_color: color,
            ..Default::default()
        }),
        transform: Transform::from_translation(offset).with_scale([1.; 3].into()),
        ..default()
    }
}

fn gyro_component(variant: DroneVariant) -> GyroComponent {
    GyroComponent {
        variant,
    }
}

pub fn gyro_spawn(
    mut coms: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    coms.spawn((
        drone_pbr_bundle(&asset_server, &mut materials, Vec3::ZERO, Color::RED),
        gyro_component(DroneVariant::Both),
    ));
    coms.spawn((
        drone_pbr_bundle(&asset_server, &mut materials, Vec3::new(7.0, 0.0, 0.0), Color::BLUE),
        gyro_component(DroneVariant::Gyro),
    ));
    coms.spawn((
        drone_pbr_bundle(&asset_server, &mut materials, Vec3::new(-7.0, 0.0, 0.0), Color::YELLOW),
        gyro_component(DroneVariant::Acc),
    ));
}

pub fn gyro_update(mut port: ResMut<Port>, mut query: Query<(&mut Transform, &mut GyroComponent)>) {
    // let last_transmit = port.last_transmition;
    if let Some(p) = port.rx.clone() {
        match p.try_recv() {
            Ok(drone_state) => {
                for (mut g_body, mut gyro) in query.iter_mut() {
                    let rotation = Quat::from_array(drone_state.orientation);

                    let frame_to_g_body_orientation = |q: Quat| {
                        let (v, a) = q.to_axis_angle();
                        Quat::from_axis_angle(Vec3::new(v.y, -v.z, v.x), a)
                    };
                    g_body.rotation = frame_to_g_body_orientation(rotation);
                }
            }
            Err(_) => {}
        };
    }
}
