use std::sync::mpsc;

use bevy::prelude::*;

use crate::worlds::browser::WryWebView;

#[derive(Debug)]
pub enum AgentCommand {
    Navigate(String),
    EvalJs(String),
    GetUrl,
}

#[derive(Debug, Message)]
pub enum AgentResponse {
    NavigationComplete(String),
    UrlResult(String),
}

#[derive(Resource, Clone)]
pub struct AgentCommandSender(mpsc::Sender<AgentCommand>);

impl AgentCommandSender {
    pub fn navigate(&self, url: &str) {
        let _ = self.0.send(AgentCommand::Navigate(url.to_string()));
    }

    pub fn eval_js(&self, js: &str) {
        let _ = self.0.send(AgentCommand::EvalJs(js.to_string()));
    }

    pub fn get_url(&self) {
        let _ = self.0.send(AgentCommand::GetUrl);
    }
}

struct AgentCommandReceiver(mpsc::Receiver<AgentCommand>);

pub struct AgentPlugin;

impl Plugin for AgentPlugin {
    fn build(&self, app: &mut App) {
        let (tx, rx) = mpsc::channel();

        app.insert_resource(AgentCommandSender(tx))
            .add_message::<AgentResponse>()
            .add_systems(Update, process_agent_commands);

        app.world_mut()
            .insert_non_send_resource(AgentCommandReceiver(rx));
    }
}

fn process_agent_commands(
    receiver: NonSend<AgentCommandReceiver>,
    webview: Option<NonSend<WryWebView>>,
    mut responses: MessageWriter<AgentResponse>,
) {
    while let Ok(cmd) = receiver.0.try_recv() {
        let Some(ref wv) = webview else {
            warn!("AgentCommand {:?} dropped — no WebView active", cmd);
            continue;
        };

        match cmd {
            AgentCommand::Navigate(url) => {
                info!("Agent: navigate → {}", url);
                let _ = wv.webview.load_url(&url);
                responses.write(AgentResponse::NavigationComplete(url));
            }
            AgentCommand::EvalJs(js) => {
                info!("Agent: eval JS ({} chars)", js.len());
                match wv.webview.evaluate_script(&js) {
                    Ok(()) => {}
                    Err(e) => warn!("Agent: JS eval error: {}", e),
                }
            }
            AgentCommand::GetUrl => {
                info!("Agent: get url");
                let url = wv.webview.url().unwrap_or_default();
                responses.write(AgentResponse::UrlResult(url));
            }
        }
    }
}
