use actix_web::{
    middleware::Logger,
    web::{self, Data, Payload},
    App, HttpRequest, HttpResponse, HttpServer,
};
use actix_ws::Message;
use bevy_snake::board::{Board, BoardSettings, Direction};
use futures::future::{pending, select_all};
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::{
    select,
    sync::{
        mpsc::{channel, Receiver, Sender},
        Mutex,
    },
    time::{interval, Instant},
};

#[tokio::main]
async fn main() {
    colog::init();

    let (client_tx, client_rx) = channel(1);

    // start the web server
    let web = tokio::spawn(async {
        // build our application with a route
        HttpServer::new(move || {
            App::new()
                .wrap(Logger::default())
                .service(web::resource("/").to(|| async { "Hello world!" }))
                .service(web::resource("/board").to(board))
                .service(web::resource("/ws").to(snake_ws))
                .app_data(Data::new(client_tx.clone()))
        })
        .bind(("0.0.0.0", 1234))
        .unwrap()
        .run()
        .await
        .unwrap();
    });

    // start the game
    let game = tokio::spawn(game_loop(client_rx));

    // exit if either the web server or game loop exits
    tokio::select! {
        _ = web => { error!("web server exited"); }
        _ = game => { error!("game loop exited"); }
    }
}

async fn game_loop(mut register_client: Receiver<Client>) {
    let mut clients = Clients::new();
    let mut board = Board::new(BoardSettings::default());

    loop {
        select! {
            // register a new client
            client = register_client.recv() => {
                let Some(client) = client else {
                    error!("register_client channel closed");
                    break;
                };
                info!("new client registered at index {}", clients.clients.len());
                clients.push(client);
            }
            // loop through all clients and handle their game commands
            command = clients.next_command() => {
                match command {
                    GameCommands::Input { direction } => {
                        if let Err(e) = board.tick_board(&[Some(direction)]) {
                            error!("{}", e);
                            break;
                        }

                        clients.broadcast(GameUpdates::Board { board: board.clone() }).await;

                        println!("{:?}", board);
                    }
                }
            }
        }
    }
}

struct Clients {
    clients: Vec<Client>,
}

impl Clients {
    fn new() -> Self {
        Self {
            clients: Vec::new(),
        }
    }

    fn push(&mut self, client: Client) {
        self.clients.push(client);
    }

    async fn next_command(&mut self) -> GameCommands {
        loop {
            if self.clients.is_empty() {
                // return pending future if there are no clients
                return pending().await;
            }

            let (game_command, index, _) = select_all(
                self.clients
                    .iter_mut()
                    .map(|client| Box::pin(client.game_commands.recv())),
            )
            .await;

            if let Some(game_command) = game_command {
                return game_command;
            }

            self.clients.remove(index);
        }
    }

    async fn broadcast(&mut self, game_update: GameUpdates) {
        let mut delete = Vec::new();
        for (index, client) in self.clients.iter_mut().enumerate() {
            if let Err(e) = client.game_updates.send(game_update.clone()).await {
                error!("{}", e);
                delete.push(index);
            }
        }
        for index in delete.into_iter().rev() {
            self.clients.remove(index);
        }
    }
}

struct Client {
    game_commands: Receiver<GameCommands>,
    game_updates: Sender<GameUpdates>,
}

impl Client {
    fn new() -> (Self, Sender<GameCommands>, Receiver<GameUpdates>) {
        let (game_commands_tx, game_commands_rx) = channel(1);
        let (game_updates_tx, game_updates_rx) = channel(1);

        (
            Self {
                game_commands: game_commands_rx,
                game_updates: game_updates_tx,
            },
            game_commands_tx,
            game_updates_rx,
        )
    }
}

#[derive(Debug, Deserialize)]
enum GameCommands {
    Input { direction: Direction },
}

#[derive(Debug, Clone, Serialize)]
enum GameUpdates {
    Board { board: Board },
}

async fn board(board: Data<Mutex<Option<Board>>>) -> HttpResponse {
    HttpResponse::Ok().json(board.lock().await.clone())
}

async fn snake_ws(
    req: HttpRequest,
    stream: Payload,
    client_tx: Data<Sender<Client>>,
) -> Result<HttpResponse, actix_web::Error> {
    let (res, session, msg_stream) = actix_ws::handle(&req, stream)?;

    // spawn websocket handler (and don't await it) so that the response is returned immediately
    actix_web::rt::spawn(snake_ws_handler(session, msg_stream, (**client_tx).clone()));

    Ok(res)
}

async fn snake_ws_handler(
    mut session: actix_ws::Session,
    mut msg_stream: actix_ws::MessageStream,
    client_tx: Sender<Client>,
) {
    info!("web socket connected");

    let mut last_heartbeat = Instant::now();
    let mut interval = interval(Duration::from_secs(5));

    let (client, game_commands, mut game_updates) = Client::new();
    client_tx.send(client).await.unwrap();

    let reason = loop {
        // create "next client timeout check" future
        let tick = interval.tick();

        tokio::select! {
            // received a board update from the game
            update = game_updates.recv() => {
                match update {
                    Some(GameUpdates::Board { board }) => {
                        if let Err(e) = session.text(serde_json::to_string(&board).unwrap()).await {
                            error!("{}", e);
                            break None;
                        }
                    }

                    None => {
                        break None;
                    }
                }
            }

            // received message from WebSocket client
            msg = msg_stream.recv() => {
                match msg {
                    Some(Ok(msg)) => match msg {
                        Message::Text(text) => {
                            let command = match serde_json::from_str::<GameCommands>(&text) {
                                Ok(input) => input,
                                Err(err) => {
                                    session.text(format!("invalid input: {}", err)).await.unwrap();
                                    error!("{}", err);
                                    break None;
                                }
                            };

                            if let Err(e) = game_commands.send(command).await {
                                error!("{}", e);
                                break None;
                            }
                        }

                        Message::Binary(_) => {
                            session.text("i dont want your binary data").await.unwrap();
                        }

                        Message::Close(reason) => {
                            break reason;
                        }

                        Message::Ping(bytes) => {
                            last_heartbeat = Instant::now();
                            session.pong(&bytes).await.ok();
                        }

                        Message::Pong(_) => {
                            last_heartbeat = Instant::now();
                        }

                        Message::Continuation(_) => {
                            warn!("no support for continuation frames");
                        }

                        Message::Nop => {}
                    }

                    Some(Err(err)) => {
                        error!("{}", err);
                        break None;
                    }

                    None => break None,
                }
            }

            // heartbeat interval ticked
            _ = tick => {
                // if no heartbeat ping/pong received recently, close the connection
                if Instant::now().duration_since(last_heartbeat) > Duration::from_secs(10) {
                    info!("client has not sent heartbeat in over 10s; disconnecting");

                    break None;
                }

                // send heartbeat ping
                let _ = session.ping(b"").await;
            }
        }
    };

    // attempt to close connection gracefully
    let _ = session.close(reason).await;

    info!("disconnected");
}
