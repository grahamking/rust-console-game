use crossterm::event;
use crossterm::event::KeyCode;
use std::error::Error;
use std::time::Duration;

use crate::dir::Dir;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum InputEvent {
    Move { player_id: u8, dir: Dir },
    Fire { player_id: u8 },
    Quit,
}

pub fn wait_for_keypress() {
    let _ = event::read().unwrap();
}

pub fn events() -> Result<Vec<InputEvent>, Box<dyn Error>> {
    let mut events = Vec::new();
    while event::poll(Duration::from_secs(0))? {
        let ev = event::read()?;
        let e = match ev {
            event::Event::Key(e) => e,
            _ => {
                break;
            }
        };
        match e.code {
            // quit
            KeyCode::Esc => {
                // make the main loop and hence the program exit
                events.push(InputEvent::Quit);
                break;
            }

            // player one keys
            KeyCode::Char('w') => events.push(InputEvent::Move {
                player_id: 1,
                dir: Dir::Up,
            }),
            KeyCode::Char('s') => events.push(InputEvent::Move {
                player_id: 1,
                dir: Dir::Down,
            }),
            KeyCode::Char('a') => events.push(InputEvent::Move {
                player_id: 1,
                dir: Dir::Left,
            }),
            KeyCode::Char('d') => events.push(InputEvent::Move {
                player_id: 1,
                dir: Dir::Right,
            }),
            KeyCode::Tab => events.push(InputEvent::Fire { player_id: 1 }),

            // player two keys
            KeyCode::Up => events.push(InputEvent::Move {
                player_id: 2,
                dir: Dir::Up,
            }),
            KeyCode::Down => events.push(InputEvent::Move {
                player_id: 2,
                dir: Dir::Down,
            }),
            KeyCode::Left => events.push(InputEvent::Move {
                player_id: 2,
                dir: Dir::Left,
            }),
            KeyCode::Right => events.push(InputEvent::Move {
                player_id: 2,
                dir: Dir::Right,
            }),
            KeyCode::Char('m') => events.push(InputEvent::Fire { player_id: 2 }),
            _ => (),
        };
    }
    Ok(events)
}
