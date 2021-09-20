
use std::env;
use std::thread;
use std::time;
use std::sync::{Arc, Mutex};
use std::cmp::Ordering;

use rs_sdk::{connect, Dir, Player, SDKError};


const USAGE: &str = r#"Usage: bot 1|2
    1 to be player 1, 2 to be player 2. Defaults to player 1.
"#;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() != 1 {
        anyhow::bail!("{}", USAGE);
    }

    let (player, opponent) = match args[0].as_str() {
        "2" =>  (Player::Two, Player::One),
        _ =>  (Player::One, Player::Two),
    };

    let (mut b_in, mut b_out) = connect(player)?;

    let target_dir_write = Arc::new(Mutex::new(Dir::None));
    let target_dir_read = target_dir_write.clone();

    let writer = thread::spawn(move || {
        let mut is_move = true;
        loop {
            let op_dir = *target_dir_read.lock().unwrap();
            if op_dir != Dir::None {
                let res = if is_move {
                    b_out.dir(op_dir)
                } else {
                    b_out.fire(op_dir)
                };
                if let Err(e) = res {
                    println!("Err sending command: {}", e);
                    return;
                }
                is_move = !is_move;
            }
            thread::sleep(time::Duration::from_millis(200));
        }
    });

    let reader = thread::spawn(move || {
        let mut my_pos = (0, 0); // x,y of this bot
        let mut op_pos = (0, 0); // x,y of opponent
        loop {
            let es = match b_in.get_next_entity() {
                Ok(es) => es,
                Err(err) => match err {
                    SDKError::Stop => {
                        return;
                    },
                    SDKError::Misc(inner) => {
                        println!("bot read_exact: {}", inner);
                        return;
                    },
                }
            };
            if es.is_player(player) {
                my_pos = es.pos();
            } else if es.is_player(opponent) {
                op_pos = es.pos();
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
