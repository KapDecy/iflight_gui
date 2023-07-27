use std::f32::consts::PI;
use std::io::{BufRead, BufReader};
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
    let (rx, tx) = crossbeam_channel::bounded(1);
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
                        rx.send(fbuf).unwrap();
                        buf.clear();
                    } else {
                        println!("last {}, {}: {:?}", buf.last().unwrap(), fbuf.len(), fbuf);
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

    tx
}

pub struct GyroPlugin;

#[derive(Component)]
pub struct GyroComponent {
    state: GyroState,
    x: Option<f32>,
    y: Option<f32>,
    z: Option<f32>,
    offset: (f32, f32, f32),
}

pub(crate) enum GyroState {
    Calibration(Vec<(f32, f32, f32)>),
    Active,
}

impl Plugin for GyroPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, gyro_spawn)
            .add_systems(Update, gyro_update);
    }
}

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
            state: GyroState::Calibration(vec![]),
            x: None,
            y: None,
            z: None,
            offset: (0.0, 0.0, 0.0),
        },
    ));
}

pub fn gyro_update(mut port: ResMut<Port>, mut query: Query<(&mut Transform, &mut GyroComponent)>) {
    // let last_transmit = port.last_transmition;
    if let Some(p) = port.rx.clone() {
        match p.try_recv() {
            Ok(v) => {
                let now = Instant::now();
                println!("{:#?}", v);
                let (mut telo, mut gyro) = query.get_single_mut().unwrap();
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
                        if let Some(then) = port.last_transmition {
                            // let gx =
                            //     (v[0] - gyro.offset.0) * then.elapsed().as_secs_f32() * PI / 180.;
                            // let gy =
                            //     (v[2] - gyro.offset.2) * then.elapsed().as_secs_f32() * PI / 180.;
                            // let gz =
                            //     (v[1] - gyro.offset.1) * then.elapsed().as_secs_f32() * PI / 180.;

                            // row acc data
                            let rx = v[3];
                            let ry = v[5];
                            let rz = v[4];

                            // let r = (rx.powi(2) + ry.powi(2) + rz.powi(2)).sqrt();
                            // let arx = (rx / r).acos();
                            // let ary = (ry / r).acos();
                            // let arz = (rz / r).acos();

                            let miu = 0.001;
                            let roll = ry.atan2((rz * rz + miu * rx * rx).sqrt() * rz.signum());
                            let pitch = (-rx).atan2((ry * ry + rz * rz).sqrt());

                            // println!("roll {}", roll / PI * 180.);
                            // println!("pitch {}", pitch / PI * 180.);

                            if gyro.x.is_some() {
                                // let ax = arx - gyro.x.unwrap();
                                // let ay = ary - gyro.y.unwrap();
                                // let az = arz - gyro.z.unwrap();

                                let ax = roll - gyro.x.unwrap();
                                let az = pitch - gyro.z.unwrap();

                                telo.rotate_local_x(ax);
                                telo.rotate_local_z(az);

                                // telo.rotate_local_x(ax);
                                // telo.rotate_local_y(ay);
                                // telo.rotate_local_z(az);
                            } else {
                                telo.rotate_local_x(roll);
                                telo.rotate_local_z(pitch);

                                // telo.rotate_local_x(arx);
                                // telo.rotate_local_y(ary);
                                // telo.rotate_local_z(arz);
                            }
                            gyro.x = Some(roll);
                            gyro.z = Some(pitch);

                            // gyro.x = Some(arx);
                            // gyro.y = Some(ary);
                            // gyro.z = Some(arz);

                            // roll = arx; pitch = ary; yaw = arz;
                            // qx = np.sin(roll/2) * np.cos(pitch/2) * np.cos(yaw/2) - np.cos(roll/2) * np.sin(pitch/2) * np.sin(yaw/2)
                            // qy = np.cos(roll/2) * np.sin(pitch/2) * np.cos(yaw/2) + np.sin(roll/2) * np.cos(pitch/2) * np.sin(yaw/2)
                            // qz = np.cos(roll/2) * np.cos(pitch/2) * np.sin(yaw/2) - np.sin(roll/2) * np.sin(pitch/2) * np.cos(yaw/2)
                            // qw = np.cos(roll/2) * np.cos(pitch/2) * np.cos(yaw/2) + np.sin(roll/2) * np.sin(pitch/2) * np.sin(yaw/2)

                            // return [qx, qy, qz, qw]

                            // let qx = ((arx / 2.0).sin() * (ary / 2.).cos() * (arz / 2.0).cos())
                            //     - ((arx / 2.0).cos() * (ary / 2.).sin() * (arz / 2.0).sin());

                            // let qy = ((arx / 2.0).cos() * (ary / 2.).sin() * (arz / 2.0).cos())
                            //     + ((arx / 2.0).sin() * (ary / 2.).cos() * (arz / 2.0).sin());

                            // let qz = ((arx / 2.0).cos() * (ary / 2.).cos() * (arz / 2.0).sin())
                            //     - ((arx / 2.0).sin() * (ary / 2.).sin() * (arz / 2.0).cos());

                            // let qw = ((arx / 2.0).cos() * (ary / 2.).cos() * (arz / 2.0).cos())
                            //     + ((arx / 2.0).sin() * (ary / 2.).sin() * (arz / 2.0).sin());

                            // telo.rotation = Quat::from_xyzw(qx, qy, qz, qw);

                            // // angle_accel = arctg( Ay / sqrt( Ax^2 + Az^2 ) )
                            // // let ax = (axr / (azr.powi(2) + ayr.powi(2)).sqrt()).atan();
                            // // let ay = (ayr / (axr.powi(2) + azr.powi(2)).sqrt()).atan();
                            // // let az = (azr / (ayr.powi(2) + axr.powi(2)).sqrt()).atan();
                            // let ax = axr.atan2((azr.powi(2) + ayr.powi(2)).sqrt());
                            // let ay = ayr.atan2((axr.powi(2) + azr.powi(2)).sqrt());
                            // let az = azr.atan2((ayr.powi(2) + axr.powi(2)).sqrt());

                            // // filtered_angle = HPF*( filtered_angle + w* dt) + LPF*(angle_accel); where HPF + LPF = 1
                            // let hpf = 0.9995;
                            // let lpf = 0.0005;
                            // let x = hpf * (gx) - lpf * ax;
                            // let y = hpf * (gy) + lpf * ay;
                            // let z = hpf * (gz) - lpf * az;

                            // Only gyro
                            // telo.rotate_local_x(gx);
                            // telo.rotate_local_y(-gy);
                            // telo.rotate_local_z(gz);
                        }
                    }
                }

                port.last_transmition = Some(now);
            }
            Err(_) => {}
        };
    }
}
