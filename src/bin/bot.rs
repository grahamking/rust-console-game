
use std::env;
use std::os::unix::net;
use std::io::{Read, Write};
use std::thread;
use std::time;
use std::convert::TryInto;
use std::io::ErrorKind;
use std::sync::{Arc, Mutex};
use std::cmp::Ordering;

use rust_console_game::server;
use rust_console_game::dir::Dir;

// must match the order in which players are added in src/lib.rs
const PLAYER_1_ID: u8 = 0;
const PLAYER_2_ID: u8 = 1;

// Commands
// Must match src/server.rs into_input_event
const MOVE: u8 = 1;
const FIRE: u8 = 2;

const USAGE: &str = r#"Usage: bot 1|2
    1 to be player 1, 2 to be player 2. Defaults to player 1.
"#;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() != 1 {
        anyhow::bail!("{}", USAGE);
    }

    let (player_id, opponent_id, sock_path) = match args[0].as_str() {
        "2" =>  (PLAYER_2_ID, PLAYER_1_ID, server::SOCK_NAME_2),
        _ =>  (PLAYER_1_ID, PLAYER_2_ID, server::SOCK_NAME_1),
    };

    let mut sock_out = match net::UnixStream::connect(sock_path) {
        Ok(s) => s,
        Err(e) => anyhow::bail!("Couldn't connecto to {}. {}", sock_path, e),
    };
    let mut sock_in = sock_out.try_clone()?;
    let target_dir_write = Arc::new(Mutex::new(Dir::None));

    let target_dir_read = target_dir_write.clone();
    let writer = thread::spawn(move || {
        let mut move_cmd = vec![MOVE, 99, 0, 0, 0, 0, 0, 0];
        let mut fire_cmd = vec![FIRE, 99, 0, 0, 0, 0, 0, 0];
        let mut is_move = true;
        loop {
            let op_dir = target_dir_read.lock().unwrap().as_num();
            if op_dir != 0 {
                let cmd = if is_move {
                    move_cmd[1] = op_dir;
                    &move_cmd
                } else {
                    fire_cmd[1] = op_dir;
                    &fire_cmd
                };
                if let Err(e) = sock_out.write(cmd) {
                    println!("bot sock write err: {}", e);
                    return;
                }
                is_move = !is_move;
            }
            thread::sleep(time::Duration::from_millis(200));
        }
    });

    let reader = thread::spawn(move || {
        let mut buf = [0u8; 12]; // protocol is units of 12 bytes
        let mut my_pos = (0, 0); // x,y of this bot
        let mut op_pos = (0, 0); // x,y of opponent
        loop {
            if let Err(e) = sock_in.read_exact(&mut buf) {
                match e.kind() {
                    ErrorKind::UnexpectedEof => { // remote closed connection
                        return;
                    },
                    _ => {
                        println!("bot read_exact: {}", e);
                    },
                }
            }
            let es = EntityState::from_network(&buf);
            if es.id == player_id {
                my_pos = (es.x, es.y);
            } else if es.id == opponent_id {
                op_pos = (es.x, es.y);
            }
            let target_dir_1 = match my_pos.0.cmp(&op_pos.0) { // 0 is x
                Ordering::Less => Dir::Right,
                Ordering::Greater => Dir::Left,
                Ordering::Equal => Dir::None,
            };
            let target_dir_2 = match my_pos.1.cmp(&op_pos.1) { // 1 is y
                Ordering::Less => Dir::Down,
                Ordering::Greater => Dir::Up,
                Ordering::Equal => Dir::None,
            };
            *target_dir_write.lock().unwrap() = choose_dir(target_dir_1, target_dir_2);
        }
    });

    writer.join().unwrap();
    reader.join().unwrap();
    Ok(())
}

#[derive(Debug)]
struct EntityState {
    id: u8,
    x: u32,
    y: u32,
    dir: Dir,
    velocity: u8,
    has_shield: bool,
}
impl EntityState {
    fn from_network(msg: &[u8; 12]) -> EntityState {
        //println!("GOT: {:?}", msg);
        let mut e = EntityState{
            id: msg[0],
            dir: Dir::from_num(msg[9]),
            velocity: msg[10],
            has_shield: msg[11] == 1,
            x: 0,
            y: 0,
        };
        // bytes 1..5 (not inclusive) are x position as u32
        let (x_bytes, rest) = msg[1..].split_at(4);
        e.x = u32::from_be_bytes(x_bytes.try_into().unwrap());
        let (y_bytes, _) = rest.split_at(4); // next 4 bytes are y position as u32
        e.y = u32::from_be_bytes(y_bytes.try_into().unwrap());

        e
    }
}

// if either are None return the other
// otherwise choose one at random
fn choose_dir(d1: Dir, d2: Dir) -> Dir {
    if d1 == Dir::None {
        d2
    } else if d2 == Dir::None || rand::random() {
        d1
    } else {
        d2
    }
}
