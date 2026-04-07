mod modal;

use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::stdout;
use std::rc::Rc;

use clm_core::buffer::BufferId;
use clm_core::editor::{EditorState, SharedState};
use clm_core::event::{
    DispatchDescriptor, Event as ClmEvent, EventBus, EventKind, PropertyKey,
    Resolver, SortKey, Subscription, SubscriptionProperty,
};
use crossterm::cursor::MoveTo;
use crossterm::execute;
use crossterm::style::Print;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
    enable_raw_mode,
};

fn main() -> anyhow::Result<()> {
    let file = "./Cargo.toml";
    let state = Rc::new(RefCell::new(EditorState::from_file(file)?));
    let mut bus = EventBus::new();

    bus.subscribe(Subscription {
        plugin_id: clm_core::event::PluginId(0),
        kind: EventKind("key_input".to_string()),
        properties: HashMap::from([(
            PropertyKey("priority".to_string()),
            Box::new(100) as SubscriptionProperty,
        )]),
        handler: Box::new(modal::ModalPlugin::new(state.clone())),
    });
    bus.register_resolver(
        SortKey("priority".to_string()),
        PropertyKey("priority".to_string()),
        Box::new(|priority: Option<&Box<dyn Any + 'static>>| {
            let Some(priority) = priority
            else {
                return i32::MIN;
            };
            priority.downcast_ref::<i32>().copied().unwrap_or(i32::MIN)
        }) as Resolver,
    );

    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen, MoveTo(0, 0))?;

    let mut size = crossterm::terminal::size()?;

    loop {
        use crossterm::event::{Event, read};
        match read()? {
            Event::Key(key_event) => {
                bus.emit(
                    ClmEvent {
                        kind: EventKind("key_input".to_string()),
                        payload: Box::new(key_event),
                    },
                    DispatchDescriptor {
                        consumable: true,
                        sort_keys: vec![SortKey("priority".to_string())],
                    },
                );
            }
            Event::Resize(width, height) => {
                size = (width, height);
            }
            _ => {}
        }

        while bus.dispatch_next() {}

        render(state.clone(), size)?;

        if !state.borrow().running {
            break;
        }
    }

    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)?;
    Ok(())
}

fn render(
    state: SharedState,
    size: (u16, u16),
) -> anyhow::Result<()> {
    use crossterm::terminal::{Clear, ClearType};
    execute!(stdout(), Clear(ClearType::All))?;
    let state = state.borrow();
    // バッファーの表示
    for row in 0..size.1.saturating_sub(1) {
        if let Some(line) = state.buffer.rope().get_line(row as usize) {
            execute!(
                stdout(),
                MoveTo(0, row),
                Print(truncate_to_width(
                    &line.chars().collect::<String>(),
                    size.0 as usize
                ))
            )?;
        }
    }
    // カーソルの設定
    let cursor = state.cursor;
    execute!(stdout(), MoveTo(cursor.col as u16, cursor.row as u16))?;
    Ok(())
}

fn truncate_to_width(line: &str, max_width: usize) -> &str {
    use unicode_width::UnicodeWidthChar;
    let mut width = 0;
    for (i, c) in line.char_indices() {
        let w = c.width().unwrap_or(0);
        if width + w > max_width {
            return &line[..i];
        }
        width += w;
    }
    line
}
