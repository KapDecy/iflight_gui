use std::f32::consts::PI;
use std::io::{BufRead, BufReader, Read};
use std::process::exit;
use std::time::Instant;

use bevy::prelude::*;
use crossbeam_channel::Receiver;

#[derive(Resource)]
pub struct Port {
    pub rx: Option<Receiver<Vec<f32>>>,
    pub last_transmition: Option<Instant>,
}

const DELIMITER: u8 = 255;

pub fn open(port_path: &std::path::Path, baudrate: u32) -> Receiver<Vec<f32>> {
    let (tx, rx) = crossbeam_channel::bounded(1);
    let mut port = serialport::new(port_path.to_string_lossy(), baudrate)
        .timeout(std::time::Duration::from_secs(20))
        .open_native();

    while port.is_err() {
        port = serialport::new(port_path.to_string_lossy(), baudrate)
            .timeout(std::time::Duration::from_secs(20))
            .open_native();
    }

    let port = port.unwrap();
    let mut reader = BufReader::new(port);
    std::thread::spawn(move || {
        let mut buf = vec![];
        loop {
            match reader.read_until(DELIMITER, &mut buf) {
                Ok(_n) => {
                    if buf[buf.len() - 2] != 254 {
                        continue;
                    }

                    if buf.len() != 54 {
                        buf.clear();
                        continue;
                    }

                    let mut fbuf = buf[0..48]
                        .chunks_exact(4)
                        .map(|chuck| f32::from_le_bytes(chuck.try_into().unwrap()))
                        .collect::<Vec<f32>>();
                    let time = u32::from_le_bytes(buf[48..52].try_into().unwrap());

                    if 12 == fbuf.len() {
                        fbuf.push(time as f32 / 1.0e6);
                        // println!("{:#?}", fbuf);
                        tx.send(fbuf).unwrap();
                        buf.clear();
                    } else {
                        // println!("last {}, {}: {:?}", buf.last().unwrap(), fbuf.len(), fbuf);
                    }

                    buf.clear();
                }
                Err(e) => {
                    if !matches!(e.kind(), std::io::ErrorKind::TimedOut) {
                        exit(1);
                    }
                }
            }
        }
    });

    rx
}

use std::net::TcpStream;
pub fn open_tcp() -> Receiver<Vec<f32>> {
    let (tx, rx) = crossbeam_channel::bounded(1);

    let mut stream = TcpStream::connect("99.22.0.1:9922").unwrap();

    std::thread::spawn(move || {
        loop {
            let mut buf = [0u8; 54];
            stream.read_exact(&mut buf).unwrap();
            // println!("{:?}", buf);
            // println!("read: {:?}", );
            // if buf[buf.len() - 2] != 254 {
            //     continue;
            // }

            let mut fbuf = buf[0..48]
                .chunks_exact(4)
                .map(|chuck| f32::from_le_bytes(chuck.try_into().unwrap()))
                .collect::<Vec<f32>>();
            let time = u32::from_le_bytes(buf[48..52].try_into().unwrap());

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
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub z: Option<f32>,
    pub signy: f32,
    pub offset: (f32, f32, f32),
    pub variant: DroneVariant,
}

#[derive(Component)]
pub enum DroneVariant {
    Gyro,
    Acc,
    Both,
}

pub enum GyroState {
    Calibration(Vec<(f32, f32, f32)>),
    Active,
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

pub fn gyro_spawn(
    mut coms: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    coms.spawn((
        PbrBundle {
            mesh: asset_server.load("Drone2.obj"),
            material: materials.add(StandardMaterial {
                base_color: Color::RED,
                ..Default::default()
            }),
            transform: Transform::from_scale([1.; 3].into()),
            ..default()
        },
        GyroComponent {
            acc_weight: INIT_ACC_WEIGHT,
            state: GyroState::Calibration(vec![]),
            x: None,
            y: None,
            z: None,
            signy: 1.0,
            offset: (0.0, 0.0, 0.0),
            variant: DroneVariant::Both,
        },
    ));
    coms.spawn((
        PbrBundle {
            mesh: asset_server.load("Drone2.obj"),
            material: materials.add(StandardMaterial {
                base_color: Color::BLUE,
                ..Default::default()
            }),
            // transform: Transform::from_scale([1.; 3].into()),
            transform: Transform::from_xyz(7., 0., 0.).with_scale([1.; 3].into()),
            ..default()
        },
        GyroComponent {
            acc_weight: INIT_ACC_WEIGHT,
            state: GyroState::Calibration(vec![]),
            x: None,
            y: None,
            z: None,
            signy: 1.0,
            offset: (0.0, 0.0, 0.0),
            variant: DroneVariant::Gyro,
        },
    ));
    coms.spawn((
        PbrBundle {
            mesh: asset_server.load("Drone2.obj"),
            material: materials.add(StandardMaterial {
                base_color: Color::YELLOW,
                ..Default::default()
            }),
            // transform: Transform::from_scale([1.; 3].into()),
            transform: Transform::from_xyz(-7., 0., 0.).with_scale([1.; 3].into()),
            ..default()
        },
        GyroComponent {
            acc_weight: INIT_ACC_WEIGHT,
            state: GyroState::Calibration(vec![]),
            x: None,
            y: None,
            z: None,
            signy: 1.0,
            offset: (0.0, 0.0, 0.0),
            variant: DroneVariant::Acc,
        },
    ));
}

pub fn gyro_update(mut port: ResMut<Port>, mut query: Query<(&mut Transform, &mut GyroComponent)>) {
    // let last_transmit = port.last_transmition;
    if let Some(p) = port.rx.clone() {
        match p.try_recv() {
            Ok(v) => {
                let now = Instant::now();
                // println!("{:#?}", v);
                // let (mut telo, mut gyro) = query.iter_mut().next().unwrap();
                for (mut telo, mut gyro) in query.iter_mut() {
                    match &mut gyro.state {
                        GyroState::Calibration(cal_v) => {
                            cal_v.push((v[0], v[1], v[2]));
                            if cal_v.len() > 100 {
                                let mean_x =
                                    cal_v.iter().map(|x| x.0).sum::<f32>() / cal_v.len() as f32;
                                let mean_y =
                                    cal_v.iter().map(|x| x.1).sum::<f32>() / cal_v.len() as f32;
                                let mean_z =
                                    cal_v.iter().map(|x| x.2).sum::<f32>() / cal_v.len() as f32;
                                gyro.offset = (mean_x, mean_y, mean_z);
                                gyro.state = GyroState::Active;
                            }
                        }
                        GyroState::Active => {
                            match gyro.variant {
                                DroneVariant::Gyro => {
                                    let gx = (v[0] - gyro.offset.0) * v[12] * PI / 180.;
                                    let gz = (v[1] - gyro.offset.1) * v[12] * PI / 180.;

                                    if gyro.x.is_some() {
                                        let prevx = gyro.x.unwrap();
                                        let prevz = gyro.z.unwrap();

                                        let gyrox = prevx + gx;
                                        let gyroz = prevz + gz;

                                        let xr = gyrox;
                                        let zr = gyroz;

                                        // println!("gyro x: {gyrox}\ngyroz: {gyroz}\n\nroll: {roll}\npitch: {pitch}\n\nestimated x: {xr}\nestimated z: {zr}\n\n\n");
                                        println!(
                                            "only gyrox: {}\nonly gyroz: {}",
                                            gyrox / PI * 180.0,
                                            gyroz / PI * 180.0
                                        );
                                        telo.rotation =
                                            Quat::from_euler(EulerRot::XYZ, xr, 0.0, zr);
                                        gyro.x = Some(xr);
                                        gyro.z = Some(zr);
                                    } else {
                                        gyro.x = Some(0.);
                                        gyro.y = Some(0.);
                                        gyro.z = Some(0.);
                                    }
                                }
                                DroneVariant::Acc => {
                                    // raw acc data
                                    let rx = -v[3];
                                    let ry = v[5];
                                    let rz = -v[4];

                                    let roll = f32::atan2(rz, (rx * rx + ry * ry).sqrt());
                                    // let roll = 0.0;
                                    let pitch = f32::atan2(-rx, (ry * ry + rz * rz).sqrt());

                                    // let roll = -rz.atan2(ry);
                                    // let pitch = -rx.atan2(ry);

                                    println!(
                                        "only roll: {}\nonly pitch: {}\n\n",
                                        roll / PI * 180.,
                                        pitch / PI * 180.
                                    );

                                    telo.rotation =
                                        Quat::from_euler(EulerRot::XYZ, roll, 0.0, pitch);
                                }
                                DroneVariant::Both => {
                                    let gx = (v[0] - gyro.offset.0) * v[12] * PI / 180.;
                                    let gz = (v[1] - gyro.offset.1) * v[12] * PI / 180.;

                                    // row acc data
                                    let rx = -v[3];
                                    let ry = v[5];
                                    let rz = -v[4];

                                    let signy = ry.signum();

                                    let roll = f32::atan2(rz, signy * (rx * rx + ry * ry).sqrt());
                                    let pitch = f32::atan2(-rx, signy * (ry * ry + rz * rz).sqrt());

                                    if gyro.x.is_some() {
                                        let aw = gyro.acc_weight;

                                        let prevx = gyro.x.unwrap();
                                        let prevz = gyro.z.unwrap();

                                        let gyrox = (prevx + gx) % 360.;
                                        let gyroz = (prevz + gz) % 360.;

                                        let xr = if signy < 0. {
                                            gyrox
                                        } else {
                                            if gyro.signy < 0. {
                                                roll
                                            } else {
                                                roll * aw + gyrox * (1. - aw)
                                            }
                                        };

                                        let zr = if signy < 0. {
                                            gyroz
                                        } else {
                                            if gyro.signy < 0. {
                                                pitch
                                            } else {
                                                pitch * aw + gyroz * (1. - aw)
                                            }
                                        };

                                        println!("sign: {}\n\ngyroxt: {}\ngyrozt: {}\n\nroll: {}\npitch: {}\n\nestimated x: {}\nestimated z: {}\n\n\n",
                                                signy,
                                                gyrox / PI * 180.,
                                                gyroz / PI * 180.,
                                                roll / PI * 180.,
                                                pitch / PI * 180.,
                                                xr / PI * 180.,
                                                zr / PI * 180.,
                                            );

                                        telo.rotation = Quat::IDENTITY;
                                        telo.rotate_local_x(xr);
                                        telo.rotate_local_z(zr);

                                        // if signy < 0.0 {
                                        //     telo.rotate_local_z(-PI);
                                        // }

                                        // telo.rotation =
                                        //     Quat::from_euler(EulerRot::XYZ, xr, 0.0, zr).to_euler(EulerRot::XYZ);

                                        // let rotation_this_frame =
                                        //     Quat::from_axis_angle(Vec3::X, xr)
                                        //         * Quat::from_axis_angle(Vec3::Z, zr);
                                        // telo.rotation = Quat::NAN * rotation_this_frame;

                                        gyro.signy = signy;
                                        gyro.x = Some(xr);
                                        gyro.z = Some(zr);
                                    } else {
                                        gyro.x = Some(roll);
                                        gyro.y = Some(0.);
                                        gyro.z = Some(pitch);
                                    }
                                }
                            }
                        }
                    }
                }

                port.last_transmition = Some(now);
            }
            Err(_) => {}
        };
    }
}
