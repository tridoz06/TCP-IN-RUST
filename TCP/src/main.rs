use serde::{Deserialize, Serialize};

use tokio::io::AsyncBufReadExt;
use tokio::net::UdpSocket;
use tokio::sync::Mutex;

use std::sync::Arc;
use std::env;
use std::io;

use crossterm::{execute, terminal::{Clear, ClearType}};
use crossterm::cursor;



#[derive(Debug, Serialize, Deserialize)]
struct Packate {
    source_port: u16,
    destination_port: u16,
    source_IP_address: u32,
    destination_IP_address: u32,
    syn_flag: u8,
    fyn_flag: u8,
    sequence_number: u32,
    ACK_number: u32,
    message: String,
}


#[tokio::main]
async fn main() {

    if let Err(err) = execute!(io::stdout(), Clear(ClearType::All)) {
        eprintln!("Errore durante la pulizia del terminale: {}", err);
    }

    // Muove il cursore all'inizio del terminale
    if let Err(err) = execute!(io::stdout(), cursor::MoveTo(0, 0)) {
        eprintln!("Errore durante lo spostamento del cursore: {}", err);
    }




    let shared_input = Arc::new(Mutex::new(String::new()));
    let input_clone = Arc::clone(&shared_input);

    let args: Vec<String> = env::args().collect();

    let my_port: u16 = match args[1].parse() {
        Ok(num) => num,
        Err(_) => {
            eprintln!("L'argomento non è un numero valido");
            return;
        }
    };
    
    let his_port: u16 = match args[2].parse() {
        Ok(num) => num,
        Err(_) => {
            eprintln!("L'argomento non è un numero valido");
            return;
        }
    }; 

    let mut packate_to_send = Packate {
        source_port: 0,
        destination_port: 0,
        source_IP_address: 0,
        destination_IP_address: 0,
        syn_flag: 0,
        fyn_flag: 0,
        sequence_number: 0,
        ACK_number: 0,
        message: String::new(),
    };

    let mut packate_to_receive = Packate {
        source_port: 0,
        destination_port: 0,
        source_IP_address: 0,
        destination_IP_address: 0,
        syn_flag: 0,
        fyn_flag: 0,
        sequence_number: 0,
        ACK_number: 0,
        message: String::new(),
    };

    tokio::spawn(async move {
        handle_keyboard_input(input_clone).await;
    });

    let socket = UdpSocket::bind(format!("127.0.0.1:{}", my_port)).await.unwrap();
    let destination = format!("127.0.0.1:{}", his_port);

    let mut buf = [0u8; 1024];

    loop {
        tokio::select! {
            result = socket.recv_from(&mut buf) => {
                match result {
                    Ok((size, _)) => {
                        packate_to_receive = bincode::deserialize(&buf[..size]).expect("failed to deserialize");
                        println!("mesaggio ricevuto = {}", packate_to_receive.message);
                    }
                    Err(e) => eprintln!("Errore durante la ricezione dei dati UDP: {}", e),
                }
            }

            _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {
                let mut input = shared_input.lock().await;
                if !input.is_empty() {
                    packate_to_send.message = input.clone();
                    println!("");
                    let serialized_data = bincode::serialize(&packate_to_send).expect("Failed to serialize");
                    socket.send_to(&serialized_data, destination.clone()).await.expect("Failed to send");
                    *input = String::new();
                }
            }
        }
    }
}


async fn handle_keyboard_input(shared_input: Arc<Mutex<String>>) {
    loop {
        let stdin = tokio::io::stdin();
        let mut reader = tokio::io::BufReader::new(stdin);

        let mut line = String::new();
        if let Ok(_) = reader.read_line(&mut line).await {
            let mut input = shared_input.lock().await;
            *input = line;
        } else {
            eprintln!("Errore durante la lettura dell'input dalla tastiera");
        }
    }
}
