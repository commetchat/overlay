use log::{info, trace, warn};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[cfg(target_os = "windows")]
use crate::backend::winit::WinitBackend;
use crate::{
    backend::{OverlayArgs, OverlayBackend, winit::WinitData},
    render::render_state::{RenderState, UserData, UserState},
    widget::{Widget, WidgetMessage},
};

#[cfg(all(target_os = "linux"))]
use crate::backend::wayland::WaylandBackend;

#[cfg(all(target_os = "linux"))]
use std::env;
use std::{
    collections::BTreeMap,
    io::{self, BufReader, Write},
    process::exit,
    sync::mpsc::{Receiver, Sender, channel},
    thread,
    time::Duration,
};

mod backend;
mod render;
mod widget;

pub fn pick_backend() -> Box<dyn OverlayBackend> {
    let args = OverlayArgs {
        width: 1024,
        height: 1024,
    };

    #[cfg(target_os = "linux")]
    {
        let session = env::var("XDG_SESSION_TYPE").ok();

        match session.as_deref() {
            Some("wayland") => {
                info!("Starting with wayland backend");
                return Box::new(WaylandBackend::new(&args).unwrap());
            }
            _ => {
                use crate::backend::winit::WinitBackend;

                info!("Starting with winit backend");
                return Box::new(WinitBackend::new(&args));
            }
        }
    }

    #[cfg(target_os = "windows")]
    return Box::new(WinitBackend::new(&args));
}

pub enum Message {
    Line(String),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VoipMemberState {
    display_name: String,
    avatar_url: Option<String>,
    muted: bool,
    talking: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VoipStateData {
    state: BTreeMap<String, VoipMemberState>,
}

pub fn run_overlay() {
    stderrlog::new()
        .verbosity(log::LevelFilter::Trace)
        .module(module_path!())
        .init()
        .unwrap();

    info!("Starting overlay!");

    let mut overlay = pick_backend();

    let (input_sender, input_receiver) = channel::<Message>();

    std::thread::spawn(|| read_data(input_sender));

    let mut state = RenderState::new();

    let mut widget = widget::Widget { num_requests: 0 };

    widget.send_from_widget("supported_api_versions", json!({}));

    loop {
        let msg = widget.read_message(&input_receiver);

        if let Some(msg) = msg {
            handle_messages(&mut state, msg);
        }

        state.iteration += 1;

        overlay.poll_event_queue(&state);


        std::thread::sleep(Duration::from_millis(35));
    }
}

fn handle_messages(state: &mut RenderState, msg: WidgetMessage) {
    match msg.action.as_str() {
        "chat.commet.voip_overlay.voip_state" => {
            match serde_json::from_value::<VoipStateData>(msg.data) {
                Ok(new_state) => {
                    info!("Received new voip state! {:?}", new_state);

                    state.users = new_state
                        .state
                        .values()
                        .map(|f| {
                            let mut user_state = UserState::Idle;

                            if f.muted {
                                user_state = UserState::Muted;
                            }

                            if f.talking {
                                user_state = UserState::Talking;
                            }

                            UserData {
                                display_name: f.display_name.clone(),
                                state: user_state,
                            }
                        })
                        .collect();

                    state.changed = true;
                }
                Err(_) => {
                    warn!("Failed to parse voip state")
                }
            }
        }
        _ => (),
    }
}

fn read_data(sender: Sender<Message>) {
    let stdin = io::stdin();

    loop {
        let mut buffer = String::new();
        info!("Attempting to read data");
        match stdin.read_line(&mut buffer) {
            Ok(v) => {
                if v == 0 {
                    info!("stdin closed!");
                    exit(0);
                }

                info!("Received data!");
                _ = sender.send(Message::Line(buffer));
            }
            Err(_) => {
                warn!("Error while reading input!")
            }
        }

        std::thread::sleep(Duration::from_millis(20));
    }
}
