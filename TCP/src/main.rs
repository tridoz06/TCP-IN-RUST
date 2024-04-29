use serde::{Deserialize, Serialize};

use tokio::io::AsyncBufReadExt;
use tokio::net::UdpSocket;
use tokio::sync::Mutex;
use tokio::time::timeout;

use rand::Rng;

use std::sync::Arc;
use std::env;
use std::io;
use std::time::Duration;

use crossterm::{execute, terminal::{Clear, ClearType}};
use crossterm::cursor;

#[derive(Debug, Serialize, Deserialize)]


struct Packate {
    source_port: u16,
    destination_port: u16,
    source_IP_address: String,
    destination_IP_address: String,
    window: u16;
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

    let waiting_time = Duration::from_secs(2);

    let mut rng = rand::thread_rng();

    let args: Vec<String> = env::args().collect();

    let mut my_ip = String::from("127.0.0.1");
    let mut his_ip = String::from("127.0.0.1");

    if args.len() < 3{
        println!("ERRORE NELL'ESECUZIONE DEL PROGRAMMA, PASSARE LE PORTE COME ARGOMENTO");
        return;
    }else if args.len() > 3{
        my_ip = args[3].clone();
        his_ip = args[4].clone();
    }

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
        source_port: my_port,
        destination_port: his_port,
        source_IP_address: my_ip.clone(),
        destination_IP_address: his_ip.clone(),
        window = rng.gen_range(100..=200);
        syn_flag: 1,
        fyn_flag: 0,
        sequence_number: rng.gen_range(100..=1000),
        ACK_number: 0,
        message: String::new(),
    };

    let mut packate_to_receive = Packate {
        source_port: his_port,
        destination_port: my_port,
        source_IP_address: his_ip.clone(),
        destination_IP_address: my_ip.clone(),
        syn_flag: 0,
        fyn_flag: 0,
        sequence_number: 0,
        ACK_number: 0,
        message: String::new(),
    };

    tokio::spawn(async move {
        handle_keyboard_input(input_clone).await;
    });

    let socket = UdpSocket::bind(format!("{}:{}", my_ip, my_port)).await.unwrap();
    let destination = format!("{}:{}", his_ip, his_port);

     
    let mut buf = [0u8; 1024];



    //INIZIO THREE WAY HANDSHAKE
    let mut serialized_data = bincode::serialize(&packate_to_send).expect("Failed to serialize");

    let mut done_three_way_hand_shake: u8 = 0;
    //FINE THREE WAY HANDSHAKE

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

                if packate_to_receive.syn_flag == 1 {
                    packate_to_send.syn_flag = 1;
                    packate_to_send.ACK_number = packate_to_receive.sequence_number + 1;
                    println!("RICEVUTO MESSAGGIO DI RICHIESTA SINCRONIZZAZIONE, MANDO ACK...");

                    serialized_data = bincode::serialize(&packate_to_send).expect("failed to serialize");
                    socket.send_to(&serialized_data, destination.clone()).await.expect("failed to send");

                    while packate_to_receive.ACK_number != packate_to_send.sequence_number + 1{
                        
                        match timeout(waiting_time, socket.recv_from(&mut buf)).await {
                            // Ricezione di dati
                            Ok(result) => match result{
                                Ok((size, _source)) => {
                                    packate_to_receive = bincode::deserialize(&buf[..size]).unwrap();
                                    break; // Esci dal ciclo una volta ricevuto il pacchetto
                                }
                                // Nessun dato disponibile, continua con il ciclo
                                Err(e) => { }
                            },
                            // Altro errore, interrompi l'esecuzione
                            Err(e) => {
                                println!("NESSUNA RISPOSTA RICEVUTA...\n");
                            }
                        }    
                    }

                    println!("ACK RICEVUTO, PROCEDO ALLA COMUNICAZIONE...");
                    packate_to_send.syn_flag = 0;
                    done_three_way_hand_shake = 1;

                }
            }


            _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {
                let mut input = shared_input.lock().await;
                if !input.is_empty() {

                    if done_three_way_hand_shake == 0 {
                        done_three_way_hand_shake = 1;

                        packate_to_send.syn_flag = 1;
                        
                        while packate_to_receive.syn_flag == 0{

                            socket.send_to(&serialized_data, destination.clone()).await.expect("Failed to send SYN packet");
                            println!("INVIO PACCHETTO DI SINCORNIZZAZIONE...");
                    
                            // Attendi la risposta
                            match timeout(waiting_time, socket.recv_from(&mut buf)).await {
                                // Ricezione di dati
                                Ok(result) => match result{
                                    Ok((size, _source)) => {
                                        packate_to_receive = bincode::deserialize(&buf[..size]).unwrap();
                                        break; // Esci dal ciclo una volta ricevuto il pacchetto
                                    }
                                    // Nessun dato disponibile, continua con il ciclo
                                    Err(e) => { }
                                },
                                // Altro errore, interrompi l'esecuzione
                                Err(e) => {
                                    println!("NESSUNA RISPOSTA RICEVUTA...\n");
                                }
                            }
                    
                        }

                        println!("RICEVUTA CONFERMA DI ACK PER IL MIO MESSAGGIO, RISPONDO CON ACK PER CONFERMARE...");

                        packate_to_send.syn_flag = 0;
                        packate_to_send.ACK_number = packate_to_receive.sequence_number + 1;
                        packate_to_send.sequence_number = 0;

                        serialized_data = bincode::serialize(&packate_to_send).expect("failed to serialize");

                        socket.send_to(&serialized_data, destination.clone()).await.expect("failde to send");

                    }else{
                        packate_to_send.message = input.clone();
                        packate_to_send.syn_flag = 0;
                        println!("");
                        serialized_data = bincode::serialize(&packate_to_send).expect("Failed to serialize");
                        socket.send_to(&serialized_data, destination.clone()).await.expect("Failed to send");
                        *input = String::new();
                    }

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
