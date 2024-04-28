use warp::Filter;
use warp::ws::{WebSocket};
use futures::{SinkExt, stream::StreamExt};

#[tokio::main]
async fn main() {
     let static_files = warp::get()
        .and(warp::fs::file("./index.html"));

    let websocket_route = warp::path("websocket")
        .and(warp::ws())
        .map(|ws: warp::ws::Ws| {
            ws.on_upgrade(handle_websocket)
        });

    let routes = websocket_route.or(static_files);

    warp::serve(routes)
        .run(([0, 0, 0, 0], 3030))
        .await;
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
            tx.send(msg).await.unwrap();
        } else if msg.is_binary() {
            // println!("Received frame: {:?}", msg.as_bytes().len());
        }
    }
}
