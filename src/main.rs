use std::sync::Mutex;
use wayland_client::{
    backend::ObjectId,
    event_created_child,
    globals::{registry_queue_init, GlobalListContents},
    protocol::wl_registry,
    Connection, Dispatch, Proxy, QueueHandle,
};
use wayland_protocols_wlr::foreign_toplevel::v1::client::{
    zwlr_foreign_toplevel_handle_v1::{self, ZwlrForeignToplevelHandleV1},
    zwlr_foreign_toplevel_manager_v1::{self, ZwlrForeignToplevelManagerV1},
};

struct AppState {
    toplevel_count: usize,
    // This assumes that there is only one active toplevel at a time.
    active_id: Option<ObjectId>,
}

struct Data {
    title: Mutex<String>,
}

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for AppState {
    fn event(
        _state: &mut AppState,
        _proxy: &wl_registry::WlRegistry,
        _event: wl_registry::Event,
        _data: &GlobalListContents,
        _conn: &Connection,
        _qhandle: &QueueHandle<AppState>,
    ) {
    }
}

impl Dispatch<ZwlrForeignToplevelManagerV1, ()> for AppState {
    event_created_child!(AppState, ZwlrForeignToplevelManagerV1, [
        0 => (ZwlrForeignToplevelHandleV1, Data { title: Mutex::new(String::new()) })
    ]);

    fn event(
        state: &mut AppState,
        _proxy: &ZwlrForeignToplevelManagerV1,
        _event: zwlr_foreign_toplevel_manager_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<AppState>,
    ) {
        match _event {
            zwlr_foreign_toplevel_manager_v1::Event::Toplevel { .. } => {
                state.toplevel_count += 1;
            }
            zwlr_foreign_toplevel_manager_v1::Event::Finished => {
                state.toplevel_count = 0;
                state.active_id = None;
            }
            _ => {}
        }
    }
}

impl Dispatch<ZwlrForeignToplevelHandleV1, Data, AppState> for AppState {
    fn event(
        app_state: &mut AppState,
        proxy: &ZwlrForeignToplevelHandleV1,
        event: <ZwlrForeignToplevelHandleV1 as wayland_client::Proxy>::Event,
        data: &Data,
        _conn: &Connection,
        _qhandle: &QueueHandle<AppState>,
    ) {
        let id = proxy.id();
        match event {
            zwlr_foreign_toplevel_handle_v1::Event::Title { title } => {
                *data.title.lock().unwrap() = title;
            }
            zwlr_foreign_toplevel_handle_v1::Event::State { state } => {
                let is_now_active = state.iter().find(|&&x| x == 2).is_some();
                if is_now_active {
                    app_state.active_id = Some(id);
                } else {
                    if app_state.active_id == Some(id) {
                        app_state.active_id = None;
                    }
                }
            }
            zwlr_foreign_toplevel_handle_v1::Event::Done => {
                if app_state.active_id == Some(id) {
                    println!("{}", data.title.lock().unwrap());
                }
            }
            zwlr_foreign_toplevel_handle_v1::Event::Closed => {
                app_state.toplevel_count -= 1;
                if app_state.active_id == Some(id) {
                    app_state.active_id = None;
                }
                if app_state.toplevel_count == 0 {}
            }
            // Ignore other events
            _ => {}
        }
    }
}

impl AppState {}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let connection = Connection::connect_to_env()?;
    let (globals, mut queue) = registry_queue_init::<AppState>(&connection)?;
    let qh = queue.handle();
    let mut state = AppState {
        toplevel_count: 0,
        active_id: None,
    };
    let _foreign_toplevel_manager: ZwlrForeignToplevelManagerV1 = globals.bind(&qh, 3..=3, ())?;
    queue.roundtrip(&mut state)?;
    let mut last_active_id = None;
    loop {
        queue.blocking_dispatch(&mut state)?;
        if state.active_id != last_active_id && state.active_id.is_none() {
            println!();
        }
        last_active_id = state.active_id.clone();
    }
}
