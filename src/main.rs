mod game;

use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use game::{Action, Game};
use serde::Deserialize;
use std::time::{Duration, Instant};

const INDEX: &str = include_str!("../static/index.html");

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum ClientMsg {
    Input { action: Action },
    Restart,
}

#[tokio::main]
async fn main() {
    let addr = std::env::var("TETRIS_ADDR").unwrap_or_else(|_| "127.0.0.1:8090".into());
    let app = Router::new()
        .route("/", get(index))
        .route("/ws", get(ws_handler));
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    println!("tetris listening on http://{addr}");
    axum::serve(listener, app).await.unwrap();
}

async fn index() -> Html<&'static str> {
    Html(INDEX)
}

async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    let mut game = Game::new();
    let mut ticker = tokio::time::interval(Duration::from_millis(16));
    let mut last_drop = Instant::now();
    let mut dirty = true;

    loop {
        tokio::select! {
            incoming = socket.recv() => {
                let Some(Ok(msg)) = incoming else { break };
                match msg {
                    Message::Text(text) => match serde_json::from_str::<ClientMsg>(&text) {
                        Ok(ClientMsg::Input { action }) => {
                            game.apply(action);
                            dirty = true;
                        }
                        Ok(ClientMsg::Restart) => {
                            game = Game::new();
                            last_drop = Instant::now();
                            dirty = true;
                        }
                        Err(_) => {}
                    },
                    Message::Close(_) => break,
                    _ => {}
                }
            }
            _ = ticker.tick() => {
                if last_drop.elapsed() >= game.drop_interval() {
                    game.gravity();
                    last_drop = Instant::now();
                    dirty = true;
                }
            }
        }

        if dirty {
            let json = serde_json::to_string(&game.snapshot()).unwrap();
            if socket.send(Message::Text(json.into())).await.is_err() {
                break;
            }
            dirty = false;
        }
    }
}
