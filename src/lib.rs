use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use futures::future::join;
use minus::{
    error::TermError,
    input::{InputClassifier, InputEvent},
    LineNumbers, Pager, SearchMode,
};
use std::env::args;
use std::path::Path;

pub static mut INPUTS: Vec<usize> = vec![];
const VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub fn get_pager() -> Result<Pager, TermError> {
    Pager::new()
}

pub fn arg_parser() -> Result<(String, Pager), TermError> {
    let arguments: Vec<String> = args().collect();
    if arguments.len() < 2 {
        eprintln!("Not enough arguments");
        std::process::exit(1);
    }
    if arguments[1] == "--version" {
        println!("rless {}", VERSION);
        std::process::exit(0);
    }
    let filename = arguments[1].clone();
    let handler = CustomInputHandler {};
    let mut pager = get_pager()?;
    pager.set_input_handler(Box::new(handler));
    pager.set_prompt(&filename);
    Ok((filename, pager))
}

use async_std::fs::read_to_string;

pub async fn read_file(
    name: String,
    pager: minus::PagerMutex,
) -> Result<(), Box<dyn std::error::Error>> {
    let file_path = Path::new(&name).exists();
    if !file_path {
        eprintln!("os error 2: {} does not exist.", &name);
        std::process::exit(2);
    }
    let file_contents = read_to_string(&name).await?;
    let changes = async {
        let mut guard = pager.lock().await;
        guard.push_str(file_contents);
        std::io::Result::<_>::Ok(())
    };

    let (res1, res2) = join(minus::async_std_updating(pager.clone()), changes).await;
    res1?;
    res2?;
    Ok(())
}

pub struct CustomInputHandler {}

type State = usize;

pub static mut UNDO: Vec<State> = vec![];
pub static mut REDO: Vec<State> = vec![];

fn get_line_count() -> usize {
    let mut amount: usize = 1;
    unsafe {
        if !INPUTS.is_empty() {
            amount = INPUTS
                .iter()
                .fold(0, |acc, elem| acc.saturating_mul(10).saturating_add(*elem));
            INPUTS.clear();
        }
    }
    amount
}

fn process_input(start: usize, end: usize) -> Option<InputEvent> {
    unsafe { UNDO.push(start) }

    Some(InputEvent::UpdateUpperMark(end))
}

impl InputClassifier for CustomInputHandler {
    fn classify_input(
        &self,
        ev: Event,
        upper_mark: usize,
        search_mode: SearchMode,
        ln: LineNumbers,
        message: bool,
        rows: usize,
    ) -> Option<InputEvent> {
        match ev {
            Event::Key(KeyEvent {
                code,
                modifiers: KeyModifiers::NONE,
            }) if code == KeyCode::Char('0') => {
                unsafe { INPUTS.push(0) }
                None
            }

            Event::Key(KeyEvent {
                code,
                modifiers: KeyModifiers::NONE,
            }) if code == KeyCode::Char('1') => {
                unsafe { INPUTS.push(1) }
                None
            }

            Event::Key(KeyEvent {
                code,
                modifiers: KeyModifiers::NONE,
            }) if code == KeyCode::Char('2') => {
                unsafe { INPUTS.push(2) }
                None
            }

            Event::Key(KeyEvent {
                code,
                modifiers: KeyModifiers::NONE,
            }) if code == KeyCode::Char('3') => {
                unsafe { INPUTS.push(3) }
                None
            }

            Event::Key(KeyEvent {
                code,
                modifiers: KeyModifiers::NONE,
            }) if code == KeyCode::Char('4') => {
                unsafe { INPUTS.push(4) }
                None
            }

            Event::Key(KeyEvent {
                code,
                modifiers: KeyModifiers::NONE,
            }) if code == KeyCode::Char('5') => {
                unsafe { INPUTS.push(5) }
                None
            }

            Event::Key(KeyEvent {
                code,
                modifiers: KeyModifiers::NONE,
            }) if code == KeyCode::Char('6') => {
                unsafe { INPUTS.push(6) }
                None
            }

            Event::Key(KeyEvent {
                code,
                modifiers: KeyModifiers::NONE,
            }) if code == KeyCode::Char('7') => {
                unsafe { INPUTS.push(7) }
                None
            }

            Event::Key(KeyEvent {
                code,
                modifiers: KeyModifiers::NONE,
            }) if code == KeyCode::Char('8') => {
                unsafe { INPUTS.push(8) }
                None
            }

            Event::Key(KeyEvent {
                code,
                modifiers: KeyModifiers::NONE,
            }) if code == KeyCode::Char('9') => {
                unsafe { INPUTS.push(9) }
                None
            }

            // Scroll up
            Event::Key(KeyEvent {
                code,
                modifiers: KeyModifiers::NONE,
            }) if code == KeyCode::Up || code == KeyCode::Char('k') => {
                let amount = get_line_count();
                let end_state = upper_mark.saturating_sub(amount.into());
                process_input(upper_mark, end_state)
            }

            // Scroll down
            Event::Key(KeyEvent {
                code,
                modifiers: KeyModifiers::NONE,
            }) if code == KeyCode::Down || code == KeyCode::Char('j') => {
                let amount = get_line_count();
                let end_state = upper_mark.saturating_add(amount.into());
                process_input(upper_mark, end_state)
            }

            // undo
            Event::Key(KeyEvent {
                code,
                modifiers: KeyModifiers::NONE,
            }) if code == KeyCode::Char('u') => {
                let undo_state = unsafe { UNDO.pop() };
                if let Some(state) = undo_state {
                    let end_state = state;
                    unsafe { REDO.push(state) }
                    return Some(InputEvent::UpdateUpperMark(end_state));
                }
                None
            }

            // redo
            Event::Key(KeyEvent {
                code,
                modifiers: KeyModifiers::NONE,
            }) if code == KeyCode::Char('r') => {
                let redo_state = unsafe { REDO.pop() };
                if let Some(state) = redo_state {
                    let end_state = state;
                    unsafe { UNDO.push(state) }
                    return Some(InputEvent::UpdateUpperMark(end_state));
                }
                None
            }

            Event::Key(KeyEvent {
                code: KeyCode::Enter,
                modifiers: KeyModifiers::NONE,
            }) => {
                if message {
                    Some(InputEvent::RestorePrompt)
                } else {
                    let amount = get_line_count();
                    let end_state = upper_mark.saturating_add(amount.into());
                    process_input(upper_mark, end_state)
                }
            }

            // If you press Ctrl-u or u, scroll up half a screen.
            Event::Key(KeyEvent {
                code: KeyCode::Char('u'),
                modifiers,
            }) if modifiers == KeyModifiers::CONTROL || modifiers == KeyModifiers::NONE => {
                let half_screen = (rows / 2) as usize;
                let end_state = upper_mark.saturating_sub(half_screen);
                process_input(upper_mark, end_state)
            }

            // If you press Ctrl-d or d, scroll down half a screen.
            Event::Key(KeyEvent {
                code: KeyCode::Char('d'),
                modifiers,
            }) if modifiers == KeyModifiers::CONTROL || modifiers == KeyModifiers::NONE => {
                let half_screen = (rows / 2) as usize;
                let end_state = upper_mark.saturating_add(half_screen);
                process_input(upper_mark, end_state)
            }

            // Mouse scroll up/down
            Event::Mouse(MouseEvent {
                kind: MouseEventKind::ScrollUp,
                ..
            }) => process_input(upper_mark, upper_mark.saturating_sub(5)),

            Event::Mouse(MouseEvent {
                kind: MouseEventKind::ScrollDown,
                ..
            }) => process_input(upper_mark, upper_mark.saturating_add(5)),

            // Go to top.
            Event::Key(KeyEvent {
                code: KeyCode::Char('g'),
                modifiers: KeyModifiers::NONE,
            }) => process_input(upper_mark, 0),

            // Go to bottom.
            Event::Key(KeyEvent {
                code: KeyCode::Char('g'),
                modifiers: KeyModifiers::SHIFT,
            })
            | Event::Key(KeyEvent {
                code: KeyCode::Char('G'),
                modifiers: KeyModifiers::SHIFT,
            })
            | Event::Key(KeyEvent {
                code: KeyCode::Char('G'),
                modifiers: KeyModifiers::NONE,
            }) => process_input(upper_mark, usize::MAX),

            // Page Up/Down
            Event::Key(KeyEvent {
                code: KeyCode::PageUp,
                modifiers: KeyModifiers::NONE,
            }) => process_input(upper_mark, upper_mark.saturating_sub(rows - 1)),

            Event::Key(KeyEvent {
                code: c,
                modifiers: KeyModifiers::NONE,
            }) if c == KeyCode::PageDown || c == KeyCode::Char(' ') => {
                process_input(upper_mark, upper_mark.saturating_add(rows - 1))
            }

            // Resize event from the terminal.
            Event::Resize(cols, rows) => {
                Some(InputEvent::UpdateTermArea(cols as usize, rows as usize))
            }

            // Switch line number display.
            Event::Key(KeyEvent {
                code: KeyCode::Char('l'),
                modifiers: KeyModifiers::NONE,
            }) => Some(InputEvent::UpdateLineNumber(!ln)),

            // Quit with Ctrl-c or q
            Event::Key(KeyEvent {
                code: KeyCode::Char('q'),
                modifiers: KeyModifiers::NONE,
            })
            | Event::Key(KeyEvent {
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::CONTROL,
            }) => Some(InputEvent::Exit),

            // Search forward with /
            Event::Key(KeyEvent {
                code: KeyCode::Char('/'),
                modifiers: KeyModifiers::NONE,
            }) => Some(InputEvent::Search(SearchMode::Forward)),

            // Search backward with /
            Event::Key(KeyEvent {
                code: KeyCode::Char('?'),
                modifiers: KeyModifiers::NONE,
            }) => Some(InputEvent::Search(SearchMode::Reverse)),

            // Go to next match with n or previous match if searching in reverse
            Event::Key(KeyEvent {
                code: KeyCode::Char('n'),
                modifiers: KeyModifiers::NONE,
            }) => Some(if search_mode == SearchMode::Reverse {
                InputEvent::PrevMatch
            } else {
                InputEvent::NextMatch
            }),

            // Go to previous match with p or next match if searching in reverse
            Event::Key(KeyEvent {
                code: KeyCode::Char('p'),
                modifiers: KeyModifiers::NONE,
            }) => Some(if search_mode == SearchMode::Reverse {
                InputEvent::NextMatch
            } else {
                InputEvent::PrevMatch
            }),
            // Otherwise disregard
            _ => None,
        }
    }
}
