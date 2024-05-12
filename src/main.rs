mod card;
mod card_database;
mod image;
mod image_camera;
mod search;
mod text_extraction;
mod websocket;
use crate::search::search;

use warp::Filter;

#[tokio::main]
async fn main() {
    println!("Initializing...");
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

    let static_files = warp::get().and(warp::fs::file("./index.html"));
    let image_route = warp::path("images").and(warp::fs::dir("./images/"));

    let websocket_route = warp::path("websocket")
        .and(warp::ws())
        .map(|ws: warp::ws::Ws| ws.on_upgrade(websocket::handle_websocket));

    let routes = websocket_route.or(image_route).or(static_files);

    {
        let _unused = search::ID_TO_FILES.lock().unwrap();
    }

    println!("Starting web server...");

    // Either spawn the server and run the visualizer, or just await the server
    // tokio::spawn(warp::serve(routes).run(([0, 0, 0, 0], 3030)));
    warp::serve(routes).run(([0, 0, 0, 0], 3030)).await;

    // image::run_visualizer().await.unwrap();
}
