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

                    if buf.len() != 50 {
                        buf.clear();
                        continue;
                    }

                    let fbuf = buf
                        .chunks_exact(4)
                        .map(|chuck| f32::from_le_bytes(chuck.try_into().unwrap()))
                        .collect::<Vec<f32>>();
                    if 12 == fbuf.len() {
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
            if buf[buf.len() - 2] != 254 {
                continue;
            }

            let mut fbuf = buf[0..48]
                .chunks_exact(4)
                .map(|chuck| f32::from_le_bytes(chuck.try_into().unwrap()))
                .collect::<Vec<f32>>();
            let time = u32::from_le_bytes(buf[48..52].try_into().unwrap());

            if 12 == fbuf.len() {
                // TODO
                fbuf.push(time as f32 / 1.0e6);
                println!("{:#?}", fbuf);
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
// const INIT_ACC_WEIGHT: f32 = 0.08;
const INIT_ACC_WEIGHT: f32 = 1.;

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
                                    if let Some(then) = port.last_transmition {
                                        let gx = (v[0] - gyro.offset.0)
                                            * then.elapsed().as_secs_f32()
                                            * PI
                                            / 180.;
                                        let gz = (v[1] - gyro.offset.1)
                                            * then.elapsed().as_secs_f32()
                                            * PI
                                            / 180.;

                                        // row acc data
                                        let rx = -v[3];
                                        let ry = v[5];
                                        let rz = -v[4];

                                        // let sign = -ry.signum();
                                        // println!("sign: {sign}");
                                        let roll = (rz / (rx.powi(2) + ry.powi(2)).sqrt()).atan();
                                        let pitch =
                                            -((rx / (ry.powi(2) + rz.powi(2)).sqrt()).atan());

                                        if gyro.x.is_some() {
                                            let aw = 0.0;

                                            let prevx = gyro.x.unwrap();
                                            let prevz = gyro.z.unwrap();

                                            let gyrox = prevx + gx;
                                            let gyroz = prevz + gz;

                                            let xr = roll * aw + gyrox * (1. - aw);
                                            let zr = pitch * aw + gyroz * (1. - aw);

                                            // println!("gyro x: {gyrox}\ngyroz: {gyroz}\n\nroll: {roll}\npitch: {pitch}\n\nestimated x: {xr}\nestimated z: {zr}\n\n\n");

                                            telo.rotation =
                                                Quat::from_euler(EulerRot::XYZ, xr, 0.0, zr);
                                            gyro.x = Some(xr);
                                            gyro.z = Some(zr);
                                        } else {
                                            gyro.x = Some(roll);
                                            gyro.y = Some(0.);
                                            gyro.z = Some(pitch);
                                        }
                                    }
                                }
                                DroneVariant::Acc => {
                                    if let Some(then) = port.last_transmition {
                                        let gx = (v[0] - gyro.offset.0)
                                            * then.elapsed().as_secs_f32()
                                            * PI
                                            / 180.;
                                        let gz = (v[1] - gyro.offset.1)
                                            * then.elapsed().as_secs_f32()
                                            * PI
                                            / 180.;

                                        // row acc data
                                        let rx = -v[3];
                                        let ry = v[5];
                                        let rz = -v[4];

                                        let sign = -ry.signum();
                                        let roll = (rz / (rx.powi(2) + ry.powi(2)).sqrt()).atan();
                                        let pitch =
                                            -((rx / (ry.powi(2) + rz.powi(2)).sqrt()).atan());

                                        // println!(
                                        //     "sign: {}\nroll: {}\n pitch: {}",
                                        //     sign,
                                        //     roll / PI * 180.,
                                        //     pitch / PI * 180.
                                        // );

                                        if gyro.x.is_some() {
                                            let aw = 1.0;

                                            let prevx = gyro.x.unwrap();
                                            let prevz = gyro.z.unwrap();

                                            let gyrox = prevx + gx;
                                            let gyroz = prevz + gz;

                                            let xr = roll * aw + gyrox * (1. - aw);
                                            let zr = pitch * aw + gyroz * (1. - aw);

                                            // println!("gyro x: {gyrox}\ngyroz: {gyroz}\n\nroll: {roll}\npitch: {pitch}\n\nestimated x: {xr}\nestimated z: {zr}\n\n\n");

                                            telo.rotation =
                                                Quat::from_euler(EulerRot::XYZ, xr, 0.0, zr);
                                            gyro.x = Some(xr);
                                            gyro.z = Some(zr);
                                        } else {
                                            gyro.x = Some(roll);
                                            gyro.y = Some(0.);
                                            gyro.z = Some(pitch);
                                        }
                                    }
                                }
                                DroneVariant::Both => {
                                    if let Some(then) = port.last_transmition {
                                        let gx = (v[0] - gyro.offset.0)
                                            * then.elapsed().as_secs_f32()
                                            * PI
                                            / 180.;
                                        let gz = (v[1] - gyro.offset.1)
                                            * then.elapsed().as_secs_f32()
                                            * PI
                                            / 180.;

                                        // row acc data
                                        let rx = -v[3];
                                        let ry = v[5];
                                        let rz = -v[4];

                                        let signy = ry.signum();
                                        // // let roll = (rz / (rx.powi(2) + ry.powi(2)).sqrt()).atan();
                                        // // let roll = if sign > 0. { roll } else { PI - roll };

                                        let roll =
                                            f32::atan2(rz, signy * (rx * rx + ry * ry).sqrt());

                                        // let pitch =
                                        //     -((rx / (ry.powi(2) + rz.powi(2)).sqrt()).atan());
                                        // let pitch = if sign > 0. { pitch } else { -pitch - PI };
                                        let pitch =
                                            -f32::atan2(rx, signy * (ry * ry + rz * rz).sqrt());

                                        if gyro.x.is_some() {
                                            let aw = gyro.acc_weight;

                                            let prevx = gyro.x.unwrap();
                                            let prevz = gyro.z.unwrap();

                                            let mut gyrox = (prevx + gx) % 360.;
                                            let mut gyroz = (prevz + gz) % 360.;

                                            // if gyrox > 179.5 {
                                            //     println!("1");
                                            //     gyrox -= 360.;
                                            // }
                                            // if gyrox < -179.5 {
                                            //     println!("2");
                                            //     gyrox += 360.;
                                            // }
                                            // if gyroz > 179.5 {
                                            //     println!("3");
                                            //     gyroz -= 360.;
                                            // }
                                            // if gyroz < -179.5 {
                                            //     println!("4");
                                            //     gyroz += 360.;
                                            // }

                                            // let gyrox = gyrox.rem_euclid(360. / 180. * PI);
                                            // let gyroz = gyroz.rem_euclid(360. / 180. * PI);

                                            let xr = roll * aw + gyrox * (1. - aw);
                                            let zr = pitch * aw + gyroz * (1. - aw);

                                            println!("sign: {}\n\ngyroxt: {}\ngyrozt: {}\n\nroll: {}\npitch: {}\n\nestimated x: {}\nestimated z: {}\n\n\n",
                                                signy,
                                                gyrox / PI * 180.,
                                                gyroz / PI * 180.,
                                                roll / PI * 180.,
                                                pitch / PI * 180.,
                                                xr / PI * 180.,
                                                zr / PI * 180.,
                                            );

                                            telo.rotation =
                                                Quat::from_euler(EulerRot::XYZ, xr, 0.0, zr);
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
                }

                port.last_transmition = Some(now);
            }
            Err(_) => {}
        };
    }
}
