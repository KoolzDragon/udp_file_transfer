use std::env;
use std::net::SocketAddr;
use tokio::runtime::Runtime;

mod packet;
mod client;
mod server;

fn main() {
    let rt = Runtime::new().unwrap();
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Використання: {} [client|server] <додаткові аргументи>", args[0]);
        return;
    }

    let mode = args[1].as_str();
    match mode {
        "client" => {
            if args.len() < 4 {
                eprintln!("Використання (client): {} client <server_address:port> <file_path>", args[0]);
                return;
            }
            let server_addr: SocketAddr = args[2].parse().expect("Невірна адреса сервера");
            let file_path = args[3].clone();
            rt.block_on(client::run_client(server_addr, file_path));
        }
        "server" => {
            if args.len() < 3 {
                eprintln!("Використання (server): {} server <bind_address:port>", args[0]);
                return;
            }
            let bind_addr: SocketAddr = args[2].parse().expect("Невірна адреса прив'язки");
            rt.block_on(server::run_server(bind_addr));
        }
        _ => {
            eprintln!("Невідомий режим: {}. Виберіть 'client' або 'server'.", mode);
        }
    }
}
