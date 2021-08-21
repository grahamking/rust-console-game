use crossterm::event;
use crossterm::event::KeyCode;

use std::error::Error;
use std::time::Duration;
use std::sync::{self, Arc};
use std::sync::atomic::{AtomicBool, Ordering};

use std::thread;

use log::error;

use crate::dir::Dir;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum InputEvent {
    Move { entity_id: u8, dir: Dir },
    Fire { entity_id: u8, dir: Dir },
    ToggleShield { entity_id: u8 },
    ChangeWeapon { entity_id: u8 },
    Quit,
}

pub fn start(ch: sync::mpsc::Sender<InputEvent>, frame_gap_ms: u64) -> (thread::JoinHandle<()>, Arc<AtomicBool>) {
    let stop = Arc::new(AtomicBool::new(false));
    let thread_stop = stop.clone();
    let h = thread::spawn(move || {
        let poll_dur = Duration::from_millis(frame_gap_ms / 2);
        while !thread_stop.load(Ordering::SeqCst) {
            match events(poll_dur) {
                Ok(v) => v.into_iter().for_each(|ev| ch.send(ev).unwrap()),
                Err(e) => {
                    error!("Input event err: {}", e);
                    return;
                }
            };

        }
    });
    (h, stop)
}

pub fn wait_for_keypress() {
    let _ = event::read().unwrap();
}

pub fn events(poll_dur: Duration) -> Result<Vec<InputEvent>, Box<dyn Error>> {
    let mut ev = Vec::new();
    // Making poll_dur == 0 maxes out this thread's CPU, so
    // read keypresses for up to half the gap between frames.
    while event::poll(poll_dur)? {
        let e = match event::read()? {
            event::Event::Key(e) => e,
            _ => {
                continue;
            }
        };
        let alt = e.modifiers.contains(event::KeyModifiers::ALT);
        match e.code {
            // quit
            KeyCode::Esc => {
                // make the main loop and hence the program exit
                ev.push(InputEvent::Quit);
                break;
            }

            // player one keys
            KeyCode::Char('w') => ev.push(InputEvent::Move {
                entity_id: 1,
                dir: Dir::Up,
            }),
            KeyCode::Char('W') => ev.push(InputEvent::Fire {
                entity_id: 1,
                dir: Dir::Up,
            }),
            KeyCode::Char('s') => ev.push(InputEvent::Move {
                entity_id: 1,
                dir: Dir::Down,
            }),
            KeyCode::Char('S') => ev.push(InputEvent::Fire {
                entity_id: 1,
                dir: Dir::Down,
            }),
            KeyCode::Char('a') => ev.push(InputEvent::Move {
                entity_id: 1,
                dir: Dir::Left,
            }),
            KeyCode::Char('A') => ev.push(InputEvent::Fire {
                entity_id: 1,
                dir: Dir::Left,
            }),
            KeyCode::Char('d') => ev.push(InputEvent::Move {
                entity_id: 1,
                dir: Dir::Right,
            }),
            KeyCode::Char('D') => ev.push(InputEvent::Fire {
                entity_id: 1,
                dir: Dir::Right,
            }),
            KeyCode::Char('e') => ev.push(InputEvent::ToggleShield { entity_id: 1 }),
            KeyCode::Char('q') => ev.push(InputEvent::ChangeWeapon { entity_id: 1 }),

            // player two keys
            KeyCode::Up => {
                if alt {
                    ev.push(InputEvent::Fire {
                        entity_id: 2,
                        dir: Dir::Up,
                    });
                } else {
                    ev.push(InputEvent::Move {
                        entity_id: 2,
                        dir: Dir::Up,
                    });
                }
            }
            KeyCode::Down => {
                if alt {
                    ev.push(InputEvent::Fire {
                        entity_id: 2,
                        dir: Dir::Down,
                    });
                } else {
                    ev.push(InputEvent::Move {
                        entity_id: 2,
                        dir: Dir::Down,
                    });
                }
            }
            KeyCode::Left => {
                if alt {
                    ev.push(InputEvent::Fire {
                        entity_id: 2,
                        dir: Dir::Left,
                    });
                } else {
                    ev.push(InputEvent::Move {
                        entity_id: 2,
                        dir: Dir::Left,
                    });
                }
            }
            KeyCode::Right => {
                if alt {
                    ev.push(InputEvent::Fire {
                        entity_id: 2,
                        dir: Dir::Right,
                    });
                } else {
                    ev.push(InputEvent::Move {
                        entity_id: 2,
                        dir: Dir::Right,
                    });
                }
            }
            KeyCode::Char('.') => ev.push(InputEvent::ToggleShield { entity_id: 2 }),
            KeyCode::Char(',') => ev.push(InputEvent::ChangeWeapon { entity_id: 2 }),

            _ => (),
        };
    }
    Ok(ev)
}
