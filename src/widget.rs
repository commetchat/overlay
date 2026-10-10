use std::{format, io::Write, sync::mpsc::Receiver};

use log::{info, trace, warn};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::Message;

pub struct Widget {
    pub num_requests: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WidgetMessage {
    pub action: String,
    pub api: String,
    pub data: serde_json::Value,
    pub response: Option<serde_json::Value>,

    #[serde(rename = "requestId")]
    pub request_id: String,

    #[serde(rename = "widgetId")]
    pub widget_id: String,
}

const WIDGET_ID: &str = "chat.commet.voip_overlay";

impl Widget {
    pub fn send_from_widget(&mut self, action: &str, data: serde_json::Value) {
        let request_id = format!("widgetapi-{}", self.num_requests);

        let payload = serde_json::json!({
            "api": "fromWidget",
            "widgetId": WIDGET_ID,
            "requestId": request_id,
            "action": action,
            "data": data
        });

        let msg = payload.to_string() + "\n";

        self.num_requests += 1;

        trace!("Sending message from widget: {}", msg);

        std::io::stdout().write_all(msg.as_bytes()).unwrap();
        std::io::stdout().flush().unwrap();
    }

    pub fn send_response(&mut self, message: &WidgetMessage, data: serde_json::Value) {
        let payload = serde_json::json!({
            "api": "toWidget",
            "widgetId": WIDGET_ID,
            "requestId": message.request_id,
            "action": message.action,
            "data": message.data,
            "response": data,
        });

        let msg = payload.to_string() + "\n";

        trace!("Sending response from widget: {}", msg);

        std::io::stdout().write_all(msg.as_bytes()).unwrap();
        std::io::stdout().flush().unwrap();
    }

    pub fn read_message(&mut self, receiver: &Receiver<Message>) -> Option<WidgetMessage> {
        match receiver.try_recv() {
            Ok(msg) => match msg {
                Message::Line(data) => {
                    info!("Received data! {}", data);

                    let parsed = serde_json::from_str::<WidgetMessage>(&data);

                    match parsed {
                        Ok(p) => {
                            info!("Parsed: {:#?}", p);

                            self.handle_message(&p);

                            return Some(p);
                        }
                        Err(_) => {
                            warn!("Failed to parse json");
                            return None;
                        }
                    }
                }
            },
            Err(_) => return None,
        }
    }

    pub fn handle_message(&mut self, msg: &WidgetMessage) {
        match msg.action.as_str() {
            "capabilities" => {
                self.send_response(msg, json!({
                    "capabilities": [
                        "org.matrix.msc4039.download_file",
                        "chat.commet.voip_overlay.voip_state"
                    ]
                }));
            }
            _ => {}
        }
    }
}
