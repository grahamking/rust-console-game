use crossterm::event;
use crossterm::event::KeyCode;
use std::error::Error;
use std::time::Duration;

use crate::dir::Dir;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum InputEvent {
    Move { entity_id: u8, dir: Dir },
    Fire { entity_id: u8, kind: FireKind },
    ToggleShield { entity_id: u8 },
    ChangeWeapon { entity_id: u8 },
    Quit,
}

// Different weapon systems. Input module doesn't decide what those are, just reports
// which one was triggered.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum FireKind {
    Up,
    Down,
    Left,
    Right,
}

pub fn wait_for_keypress() {
    let _ = event::read().unwrap();
}

pub fn events() -> Result<Vec<InputEvent>, Box<dyn Error>> {
    let mut ev = Vec::new();
    while event::poll(Duration::from_secs(0))? {
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
                kind: FireKind::Up,
            }),
            KeyCode::Char('s') => ev.push(InputEvent::Move {
                entity_id: 1,
                dir: Dir::Down,
            }),
            KeyCode::Char('S') => ev.push(InputEvent::Fire {
                entity_id: 1,
                kind: FireKind::Down,
            }),
            KeyCode::Char('a') => ev.push(InputEvent::Move {
                entity_id: 1,
                dir: Dir::Left,
            }),
            KeyCode::Char('A') => ev.push(InputEvent::Fire {
                entity_id: 1,
                kind: FireKind::Left,
            }),
            KeyCode::Char('d') => ev.push(InputEvent::Move {
                entity_id: 1,
                dir: Dir::Right,
            }),
            KeyCode::Char('D') => ev.push(InputEvent::Fire {
                entity_id: 1,
                kind: FireKind::Right,
            }),
            KeyCode::Char('e') => ev.push(InputEvent::ToggleShield { entity_id: 1 }),
            KeyCode::Char('q') => ev.push(InputEvent::ChangeWeapon { entity_id: 1 }),

            // player two keys
            KeyCode::Up => {
                if alt {
                    ev.push(InputEvent::Fire {
                        entity_id: 2,
                        kind: FireKind::Up,
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
                        kind: FireKind::Down,
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
                        kind: FireKind::Left,
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
                        kind: FireKind::Right,
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
