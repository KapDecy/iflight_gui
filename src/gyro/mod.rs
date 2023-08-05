use std::f32::consts::{FRAC_PI_2, PI};
use std::io::{BufRead, BufReader, Read};
use std::process::exit;
use std::time::Instant;

use bevy::prelude::*;
use crossbeam_channel::Receiver;

#[derive(Resource)]
pub struct Port {
    pub rx: Option<Receiver<Vec<f32>>>,
    pub last_transmission: Option<Instant>,
}

const DELIMITER: u8 = 255;



pub fn open(port_path: &std::path::Path, baudrate: u32) -> Receiver<Vec<f32>> {
    let (tx, rx) = crossbeam_channel::bounded(1);
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

    let port = port.unwrap();
    let mut reader = BufReader::new(port);
    std::thread::spawn(move || {
        let mut buf = vec![];
        loop {
            buf.clear();
            match reader.read_until(255, &mut buf) {
                Ok(n) => {
                    if buf.len() != 54 {
                        warn!("Skip packet with len {}", buf.len());
                        continue;
                    }
                    if buf[buf.len() - 2] != 254 {
                        continue;
                    }

                    let fbuf = buf
                        .chunks_exact(4)
                        .take(12)
                        .map(|chuck| f32::from_le_bytes(chuck.try_into().unwrap()))
                        .collect::<Vec<f32>>();

                    let time = u32::from_le_bytes(buf[48..52].try_into().unwrap());
                    println!("time: {}", time);

                    if 12 == fbuf.len() {
                        println!("{:#?}", fbuf);
                        tx.send(fbuf).unwrap();
                        // buf.clear();
                    } else {
                        // println!("last {}, {}: {:?}", buf.last().unwrap(), fbuf.len(), fbuf);
                    }

                    // buf.clear();
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

    rx
}
pub fn open_bkp(port_path: &std::path::Path, baudrate: u32) -> Receiver<Vec<f32>> {
    let (tx, rx) = crossbeam_channel::bounded(1);
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

    let port = port.unwrap();
    let mut reader = BufReader::new(port);
    std::thread::spawn(move || {
        let mut buf = vec![];
        loop {
            buf.clear();
            match reader.read_until(255, &mut buf) {
                Ok(n) => {
                    if buf.len() != 54 {
                        warn!("Skip packet with len {}", buf.len());
                        continue;
                    }
                    if buf[buf.len() - 2] != 254 {
                        continue;
                    }

                    let fbuf = buf
                        .chunks_exact(4)
                        .take(12)
                        .map(|chuck| f32::from_le_bytes(chuck.try_into().unwrap()))
                        .collect::<Vec<f32>>();

                    let time = u32::from_le_bytes(buf[48..52].try_into().unwrap());
                    println!("time: {}", time);

                    if 12 == fbuf.len() {
                        println!("{:#?}", fbuf);
                        tx.send(fbuf).unwrap();
                        // buf.clear();
                    } else {
                        // println!("last {}, {}: {:?}", buf.last().unwrap(), fbuf.len(), fbuf);
                    }

                    // buf.clear();
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

    rx
}

use std::net::TcpStream;
use bevy::math::Vec3Swizzles;

pub fn open_tcp() -> Receiver<Vec<f32>> {
    let (tx, rx) = crossbeam_channel::bounded(1);

    let mut stream = TcpStream::connect("99.22.0.1:9922").unwrap();

    std::thread::spawn(move || {
        loop {
            let mut buf = [0u8; 54];
            stream.read_exact(&mut buf).unwrap();
            if buf[buf.len() - 2] != 254 {
                continue;
            }

            let mut fbuf = buf[0..48]
                .chunks_exact(4)
                .map(|chuck| f32::from_le_bytes(chuck.try_into().unwrap()))
                .collect::<Vec<f32>>();
            let time = u32::from_le_bytes(buf[48..52].try_into().unwrap());
            println!("time: {}", time);

            if 12 == fbuf.len() {
                // TODO
                fbuf.push(time as f32 / 1.0e6);
                // println!("{:#?}", fbuf);
                tx.send(fbuf).unwrap();
                // buf.clear();
            } else {
                // println!("last {}, {}: {:?}", buf.last().unwrap(), fbuf.len(), fbuf);
            }

            // buf.clear();
        }
    });

    rx
}

pub struct GyroPlugin;

#[derive(Component)]
pub struct GyroComponent {
    pub acc_weight: f32,
    pub state: GyroState,
    pub rotation: Option<Quat>,
    pub variant: DroneVariant,
}

#[derive(Component)]
pub enum DroneVariant {
    Gyro,
    Acc,
    Both,
}

pub enum GyroState {
    Calibration(Vec<Vec3>),
    Active { offset: Vec3 },
}

impl Plugin for GyroPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, gyro_spawn)
            .add_systems(Update, gyro_update);
    }
}

// const INIT_ACC_WEIGHT: f32 = 0.;
const INIT_ACC_WEIGHT: f32 = 0.08;
// const INIT_ACC_WEIGHT: f32 = 1.;

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
        acc_weight: INIT_ACC_WEIGHT,
        state: GyroState::Calibration(vec![]),
        rotation: None,
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
            Ok(v) => {
                // IN DRONE COORDINATE SYSTEM
                let raw_gyro_vec = Vec3::new(v[0], v[1], v[2]);
                let raw_accel_vec = Vec3::new(v[3], v[4], v[5]);

                let now = Instant::now();
                for (mut g_body, mut gyro) in query.iter_mut() {
                    match &mut gyro.state {
                        GyroState::Calibration(cal_v) => {
                            info!("calibrate");
                            cal_v.push(raw_gyro_vec);
                            if cal_v.len() > 100 {
                                let mean = cal_v.iter().sum::<Vec3>() / cal_v.len() as f32;
                                gyro.state = GyroState::Active { offset: mean };
                            }
                        }
                        GyroState::Active { offset } => {
                            let offset = *offset;

                            let aw = match gyro.variant {
                                DroneVariant::Gyro => 0.0,
                                DroneVariant::Acc => 1.0,
                                DroneVariant::Both => gyro.acc_weight,
                            };

                            let gyro_vec = raw_gyro_vec - offset;

                            if let Some(then) = port.last_transmission {
                                let dt = then.elapsed().as_secs_f32();

                                let orientation_by_gyro = |current_orientation: Quat| {
                                    let gyro_d_rotation = Quat::from_scaled_axis(Vec3::new(-gyro_vec.x, -gyro_vec.y, 0.0) * dt);
                                    gyro_d_rotation * current_orientation
                                };

                                let orientation_by_accel = {
                                    let roll = f32::atan2(-raw_accel_vec.y, raw_accel_vec.z);
                                    let pitch = f32::atan2(raw_accel_vec.x, raw_accel_vec.yz().length());
                                    Quat::from_euler(EulerRot::ZXY, 0.0, roll, pitch)
                                };

                                match gyro.rotation {
                                    Some(current_orientation) => {
                                        let orientation_by_gyro = orientation_by_gyro(current_orientation);
                                        let weighted_orientation = Quat::lerp(orientation_by_gyro, orientation_by_accel, aw);
                                        gyro.rotation = Some(weighted_orientation)
                                    }
                                    None => gyro.rotation = Some(orientation_by_accel),
                                }

                                // convert drone frame coordinate system to graphical
                                let frame_to_g_body_orientation = |q: Quat| {
                                    let (v, a) = q.to_axis_angle();
                                    Quat::from_axis_angle(Vec3::new(v.y, -v.z, v.x), a)
                                };
                                g_body.rotation = frame_to_g_body_orientation(gyro.rotation.unwrap());
                            }
                        }
                    }
                }

                port.last_transmission = Some(now);
            }
            Err(_) => {}
        };
    }
}
