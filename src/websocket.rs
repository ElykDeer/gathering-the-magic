use crate::search::search;
use crate::{card_database, image_camera};

use futures::{stream::StreamExt, SinkExt};
use serde::Deserialize;
use warp::ws::{Message, WebSocket};

#[derive(Deserialize)]
struct ActionMessage {
    action: String,
    message: Option<String>,
    count: Option<usize>,
}

pub(crate) async fn handle_websocket(websocket: WebSocket) {
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
            match image_camera::process_frame(msg.as_bytes()) {
                Ok(Some(results)) => {
                    let reply = Message::text(format!(
                        r#"{{"action": "imageResults", "results": [{}]}}"#,
                        results
                    ));
                    assert!(tx.send(reply).await.is_ok());
                }
                Ok(None) => (),
                Err(e) => {
                    eprintln!("{:?}", e);
                }
            };
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
        "incCard" => {
            if let Some(message) = &action_msg.message {
                println!("Incrementing {}", message);
                if card_database::CARD_DATABASE
                    .lock()
                    .unwrap()
                    .inc(message, false)
                {
                    let value = {
                        let scryrs = crate::search::CARDS.lock().unwrap();
                        scryrs
                            .get_card_by_id(&message[..message.rfind('-').unwrap()])
                            .unwrap()
                            .usd()
                    };
                    let reply =
                        Message::text(format!(r#"{{"action": "notify", "price": {}}}"#, value));
                    assert!(tx.send(reply).await.is_ok());
                }
                kill_card();
            } else {
                println!("Error getting message.");
            }
        }
        "decCard" => {
            if let Some(message) = &action_msg.message {
                println!("Decrementing {}", message);
                card_database::CARD_DATABASE
                    .lock()
                    .unwrap()
                    .dec(message, false);
                kill_card();
            } else {
                println!("Error getting message.");
            }
        }
        "incFoil" => {
            if let Some(message) = &action_msg.message {
                println!("Incrementing foil {}", message);
                card_database::CARD_DATABASE
                    .lock()
                    .unwrap()
                    .inc(message, true);
                kill_card();
            } else {
                println!("Error getting message.");
            }
        }
        "decFoil" => {
            if let Some(message) = &action_msg.message {
                println!("Decrementing foil {}", message);
                card_database::CARD_DATABASE
                    .lock()
                    .unwrap()
                    .dec(message, true);
                kill_card();
            } else {
                println!("Error getting message.");
            }
        }
        "history" => {
            println!("Sending history");
            let (total_cards, total_value, cards) =
                card_database::CARD_DATABASE.lock().unwrap().history();
            let reply = Message::text(format!(
                r#"{{"action": "historyResults", "totalCards": "{}", "totalValue": "${:.2}", "cards": [{}]}}"#,
                total_cards, total_value, cards
            ));
            assert!(tx.send(reply).await.is_ok());
        }
        "setCard" => {
            if let Some(message) = &action_msg.message {
                println!(
                    "Setting {} count to {}",
                    message,
                    action_msg.count.unwrap_or_default()
                );
                card_database::CARD_DATABASE.lock().unwrap().set(
                    message,
                    action_msg.count.unwrap_or_default(),
                    false,
                );
            } else {
                println!(
                    "Error getting message {} {:?} {:?}",
                    &action_msg.action, &action_msg.message, &action_msg.count
                );
            }
        }
        "setFoil" => {
            if let Some(message) = &action_msg.message {
                println!(
                    "Setting foil {} count to {}",
                    message,
                    action_msg.count.unwrap_or_default()
                );
                card_database::CARD_DATABASE.lock().unwrap().set(
                    message,
                    action_msg.count.unwrap_or_default(),
                    true,
                );
            } else {
                println!(
                    "Error getting message {} {:?} {:?}",
                    &action_msg.action, &action_msg.message, &action_msg.count
                );
            }
        }
        "reject" => {
            println!("Reject");
            // By marking a card as dead, the next frame it's detected it'll recalculate what the card is
            kill_card()
        }
        _ => eprintln!("Unknown action: {}", action_msg.action),
    }
}

fn kill_card() {
    if let Ok(mut card) = image_camera::CARD.lock() {
        card.alive = false;
    }
}
