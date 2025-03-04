use bevy_snake::server::start_server;

fn main() {
    colog::init();

    start_server(("0.0.0.0", 1234));
}
