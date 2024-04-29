mod search;
use crate::search::search;

use futures::{stream::StreamExt, SinkExt};
use serde::Deserialize;
use warp::{
    ws::{Message, WebSocket},
    Filter,
};

#[tokio::main]
async fn main() {
    let static_files = warp::get().and(warp::fs::file("./index.html"));
    let image_route = warp::path("images").and(warp::fs::dir("../gathering_the_magic/images/"));

    let websocket_route = warp::path("websocket")
        .and(warp::ws())
        .map(|ws: warp::ws::Ws| ws.on_upgrade(handle_websocket));

    let routes = websocket_route.or(image_route).or(static_files);

    warp::serve(routes).run(([0, 0, 0, 0], 3030)).await;
}

#[derive(Deserialize)]
struct ActionMessage {
    action: String,
    message: Option<String>,
}

async fn handle_websocket(websocket: WebSocket) {
    let (mut tx, mut rx) = websocket.split();
    while let Some(result) = rx.next().await {
        let msg = match result {
            Ok(msg) => msg,
            Err(e) => {
                eprintln!("websocket error: {:?}", e);
                break;
            }
        };
        if msg.is_text() {
            println!("{:?}", msg);
            if let Ok(text) = msg.to_str() {
                if let Ok(action_msg) = serde_json::from_str::<ActionMessage>(text) {
                    handle_action(&action_msg, &mut tx).await;
                }
            }
        } else if msg.is_binary() {
            // println!("Received frame: {:?}", msg.as_bytes().len());
        } else {
            println!("Unknown : {:?}", msg);
        }
    }
}

async fn handle_action(
    action_msg: &ActionMessage,
    tx: &mut (impl SinkExt<Message> + std::marker::Unpin),
) {
    match action_msg.action.as_str() {
        "search" => {
            if let Some(message) = &action_msg.message {
                let results = search(message);
                let reply = Message::text(format!(
                    r#"{{"action": "searchResults", "results": [{}]}}"#,
                    results
                ));
                assert!(tx.send(reply).await.is_ok());
            }
        }
        // "echo" => {
        //     if let Some(message) = &action_msg.message {
        //         let reply = Message::text(format!("Echo: {}", message));
        //         tx.send(reply).await.unwrap();
        //     }
        // },
        // "log" => {
        //     if let Some(message) = &action_msg.message {
        //         println!("Log: {}", message);
        //     }
        // },
        _ => eprintln!("Unknown action: {}", action_msg.action),
    }
}