use std::cell::RefCell;
use std::io::stdout;
use std::rc::Rc;

use clm_core::editor::EditorState;
use clm_core::event::{
    DispatchDescriptor, Event as ClmEvent, EventBus, EventKind, SortKey,
};
use crossterm::cursor::MoveTo;
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
    enable_raw_mode,
};

pub struct KeyInput {
    pub key: crossterm::event::KeyEvent,
}

fn main() -> anyhow::Result<()> {
    let file = std::env::args().nth(1);
    let state = Rc::new(RefCell::new(EditorState::new(file)?));
    let mut bus = EventBus::new();

    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen, MoveTo(0, 0))?;

    loop {
        use crossterm::event::{Event, read};
        match read()? {
            Event::Key(key_event) => {
                use crossterm::event::KeyCode;
                match key_event.code {
                    // 文字入力
                    KeyCode::Char(c) => {
                        bus.emit(
                            ClmEvent {
                                kind: EventKind("key_input".to_string()),
                                payload: Box::new(c),
                            },
                            DispatchDescriptor {
                                consumable: true,
                                sort_keys: vec![SortKey(
                                    "priority".to_string(),
                                )],
                            },
                        );
                    }
                    KeyCode::Esc => {
                        // 強制終了
                        state.borrow_mut().running = false;
                    }
                    KeyCode::Enter => {}
                    _ => {}
                }
            }
            Event::Resize(_width, _height) => {}
            _ => {}
        }

        while bus.dispatch_next() {}

        render()?;

        if !state.borrow().running {
            break;
        }
    }

    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)?;
    Ok(())
}

fn render() -> anyhow::Result<()> {
    Ok(())
}
