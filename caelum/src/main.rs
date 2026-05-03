use clm_core::event::*;
use clm_core::registry::*;
use clm_core::runtime::*;
use clm_core::value::Value;

fn main() -> anyhow::Result<()> {
    init_async_runtime(8);

    register_resolver(
        SortKey("priority".to_string()),
        PropertyKey("priority".to_string()),
        Box::new(|priority: Option<&Value>| {
            let Some(Value::Int(priority)) = priority else {
                return i64::MIN;
            };
            *priority
        }) as Resolver,
    );

    let file = "./deny.toml";
    //let file = "E:/Word/言語学Aポスター/data/all8.txt";

    let (plugin, id) = clm_tui_compositor::EditorTuiPlugin::new();
    add_plugin(plugin);
    add_plugin(clm_tui_driver::TuiPlugin::new());
    add_plugin(clm_buffer::BufferPlugin::new());
    add_plugin(clm_modal::ModalPlugin::new(Some(file), id));
    add_plugin(clm_keymap::KeymapPlugin::new());

    while is_running() {
        park_until_event();
        while dispatch_next() {}
    }

    uninit_plugins();

    Ok(())
}
