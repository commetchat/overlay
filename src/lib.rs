use base64::prelude::*;

use log::{info, warn};
use piet::ImageBuf;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[cfg(target_os = "windows")]
use crate::backend::winit::WinitBackend;
use crate::{
    backend::{OverlayArgs, OverlayBackend, openvr::OpenVRBackend, winit::WinitBackend},
    render::render_state::{ImageState, RenderState},
    widget::{Widget, WidgetMessage},
};

#[cfg(all(target_os = "linux"))]
use crate::backend::wayland::WaylandBackend;

#[cfg(all(target_os = "linux"))]
use std::env;
use std::{
    collections::BTreeMap,
    io::{self},
    process::exit,
    sync::mpsc::{Sender, channel},
    time::Duration,
};

mod backend;
mod render;
mod widget;

#[derive(PartialEq, Eq, Debug)]
pub enum BackendType {
    Wayland,
    Winit,
    OpenVR,
}

pub fn pick_backend_type() -> BackendType {
    if openvr::is_hmd_present() && openvr::is_runtime_installed() {
        return BackendType::OpenVR;
    }

    #[cfg(target_os = "linux")]
    {
        let session = env::var("XDG_SESSION_TYPE").ok();

        return match session.as_deref() {
            Some("wayland") => BackendType::Wayland,
            _ => return BackendType::Winit,
        };
    }

    #[cfg(target_os = "windows")]
    return BackendType::Winit;
}

pub fn start_backend(backend_type: &BackendType) -> Box<dyn OverlayBackend> {
    let args = OverlayArgs {
        width: 1024,
        height: 1024,
    };

    return match backend_type {
        BackendType::Wayland => Box::new(WaylandBackend::new(&args).unwrap()),
        BackendType::Winit => Box::new(WinitBackend::new(&args)),
        BackendType::OpenVR => Box::new(OpenVRBackend::new(&args)),
    };
}

pub enum Message {
    Line(String),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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

pub fn run_overlay(dev_mode: bool) {
    stderrlog::new()
        .verbosity(log::LevelFilter::Trace)
        .module(module_path!())
        .init()
        .unwrap();

    info!("Starting overlay!");

    let mut current_backend_type = pick_backend_type();
    let mut overlay = start_backend(&current_backend_type);

    let (input_sender, input_receiver) = channel::<Message>();

    std::thread::spawn(|| read_data(input_sender));

    let mut state = overlay.make_render_state();

    if dev_mode {
        state.users.insert(
            "@test:example.org".to_string(),
            VoipMemberState {
                avatar_url: None,
                display_name: "Luna".to_string(),
                muted: true,
                talking: false,
            },
        );

        state.users.insert(
            "@test2:example.org".to_string(),
            VoipMemberState {
                avatar_url: None,
                display_name: "Pluto".to_string(),
                muted: false,
                talking: true,
            },
        );

        state.users.insert(
            "@test3:example.org".to_string(),
            VoipMemberState {
                avatar_url: None,
                display_name: "Curiosity".to_string(),
                muted: true,
                talking: false,
            },
        );
    }

    let mut widget = widget::Widget { num_requests: 0 };

    widget.send_from_widget("supported_api_versions", json!({}));

    loop {
        let msg = widget.read_message(&input_receiver);

        if let Some(msg) = msg {
            handle_messages(&mut state, &mut widget, msg);
        }

        state.iteration += 1;

        overlay.poll_event_queue(&state);

        state.changed = false;

        let new_type = pick_backend_type();

        if new_type != current_backend_type {
            info!("Hot swapping to different backend: ${:?}", new_type);
            overlay.dispose();

            let mut new_overlay = start_backend(&new_type);
            let mut new_state = new_overlay.make_render_state();

            for user in state.users.iter() {
                new_state.users.insert(user.0.clone(), user.1.clone());
            }

            new_state.changed = true;
            state = new_state;
            overlay = new_overlay;
            current_backend_type = new_type;
        }

        std::thread::sleep(Duration::from_millis(35));
    }
}

fn handle_messages(state: &mut RenderState, widget: &mut Widget, msg: WidgetMessage) {
    match msg.action.as_str() {
        "chat.commet.voip_overlay.voip_state" => {
            handle_voip_state_changed(state, widget, msg);
        }
        "org.matrix.msc4039.download_file" => {
            _ = handle_file_download(state, widget, msg);
        }
        _ => (),
    }
}

enum WidgetError {
    InvalidRequest,
}

fn handle_file_download(
    state: &mut RenderState,
    _widget: &mut Widget,
    msg: WidgetMessage,
) -> Result<(), WidgetError> {
    let content_uri = msg.data.as_object().ok_or(WidgetError::InvalidRequest)?;
    let content_uri = content_uri
        .get("content_uri")
        .ok_or(WidgetError::InvalidRequest)?;
    let content_uri = content_uri.as_str().ok_or(WidgetError::InvalidRequest)?;

    let response = msg.response.ok_or(WidgetError::InvalidRequest)?;
    let file = response.get("file").ok_or(WidgetError::InvalidRequest)?;
    let file = file.as_object().ok_or(WidgetError::InvalidRequest)?;

    if let Some(custom_type) = file.get("__type") {
        let custom_type = custom_type.as_str().ok_or(WidgetError::InvalidRequest)?;

        if custom_type == "Blob" {
            if let Some(data_b64) = file.get("data") {
                let data_b64 = data_b64.as_str().ok_or(WidgetError::InvalidRequest)?;

                let decoded = BASE64_STANDARD
                    .decode(data_b64)
                    .or(Err(WidgetError::InvalidRequest))?;

                info!(
                    "Received image for {} ({} bytes)",
                    content_uri,
                    decoded.len()
                );

                match image::load_from_memory(&decoded) {
                    Ok(img) => {
                        info!("Successfully decoded image!");

                        // resize image to save memory
                        let img = img.resize(
                            state.item_height * 2,
                            state.item_height * 2,
                            image::imageops::FilterType::CatmullRom,
                        );

                        let buf = match img.has_alpha() {
                            true => {
                                let rgb = img.to_rgba8();
                                ImageBuf::from_raw(
                                    rgb.to_vec(),
                                    piet::ImageFormat::RgbaSeparate,
                                    img.width() as usize,
                                    img.height() as usize,
                                )
                            }
                            false => {
                                let rgb = img.to_rgb8();
                                ImageBuf::from_raw(
                                    rgb.to_vec(),
                                    piet::ImageFormat::Rgb,
                                    img.width() as usize,
                                    img.height() as usize,
                                )
                            }
                        };

                        state
                            .images
                            .insert(content_uri.to_string(), ImageState::Received(buf));
                    }
                    Err(_) => {
                        warn!("Failed to decode image");
                        state
                            .images
                            .insert(content_uri.to_string(), ImageState::FailedToDecode);
                    }
                }

                state.changed = true;
            }
        }
    }

    Ok(())
}

fn handle_voip_state_changed(state: &mut RenderState, widget: &mut Widget, msg: WidgetMessage) {
    match serde_json::from_value::<VoipStateData>(msg.data) {
        Ok(new_state) => {
            info!("Received new voip state! {:?}", new_state);

            state.users = new_state.state;

            for (_id, user) in state.users.iter() {
                if let Some(img) = &user.avatar_url {
                    if !state.images.contains_key(img) {
                        info!("Requesting image: {}", img);

                        widget.send_from_widget(
                            "org.matrix.msc4039.download_file",
                            json!({
                                "content_uri": img.clone()
                            }),
                        );

                        state.images.insert(img.clone(), ImageState::Pending);
                    }
                }
            }

            let unneeded_images: Vec<String> = state
                .images
                .iter_mut()
                .filter(|(key, _)| {
                    !state
                        .users
                        .iter()
                        .any(|(_user, s)| s.avatar_url == Some(key.to_string()))
                })
                .map(|i| i.0.clone())
                .collect();

            for image in unneeded_images {
                state.images.remove(&image);
                info!("Removing unneeded image: {}", image);
            }

            state.changed = true;
        }
        Err(_) => {
            warn!("Failed to parse voip state")
        }
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
