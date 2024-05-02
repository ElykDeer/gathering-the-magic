mod card;
mod image;
mod image_card_extraction;
// mod image_hash;
mod search;
mod text_extraction;
use crate::search::search;

use futures::{stream::StreamExt, SinkExt};
use serde::Deserialize;
use warp::{
    ws::{Message, WebSocket},
    Filter,
};

#[tokio::main]
async fn main() {
    if !(std::path::Path::new("./scryfall.db").exists()
        && std::path::Path::new("./images/").exists())
    {
        println!("Scryfall data does not exist. Do you want to download it? This will take 3-4 hours, 4.2GB of disk, and is required only once. (y/N)");
        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line");

        if input.trim().eq_ignore_ascii_case("y") {
            scryers::download_all_cards();
        }
    }

    // image_hash::hash_all_cards().unwrap();

    let static_files = warp::get().and(warp::fs::file("./index.html"));
    let image_route = warp::path("images").and(warp::fs::dir("./images/"));

    let websocket_route = warp::path("websocket")
        .and(warp::ws())
        .map(|ws: warp::ws::Ws| ws.on_upgrade(handle_websocket));

    let routes = websocket_route.or(image_route).or(static_files);

    {
        let _unused = search::ALL_FILES.lock().unwrap();
    }

    // Either spawn the server and run the visualizer, or just await the server
    tokio::spawn(warp::serve(routes).run(([0, 0, 0, 0], 3030)));
    // warp::serve(routes).run(([0, 0, 0, 0], 3030)).await;

    image::run_visualizer().await.unwrap();
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
            if let Ok(text) = msg.to_str() {
                if let Ok(action_msg) = serde_json::from_str::<ActionMessage>(text) {
                    handle_action(&action_msg, &mut tx).await;
                }
            }
        } else if msg.is_binary() {
            if let Err(e) = image_card_extraction::process_frame(msg.as_bytes()) {
                eprintln!("{:?}", e);
            }
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
                println!("Searching for {}", message);
                let results = search(message);
                let reply = Message::text(format!(
                    r#"{{"action": "searchResults", "results": [{}]}}"#,
                    results
                ));
                assert!(tx.send(reply).await.is_ok());
            } else {
                println!("Error getting message.");
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
