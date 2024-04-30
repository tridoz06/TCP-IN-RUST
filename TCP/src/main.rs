use serde::{Deserialize, Serialize};

use tokio::io::AsyncBufReadExt;
use tokio::net::UdpSocket;
use tokio::sync::Mutex;
use tokio::time::timeout;

use rand::Rng;

use std::env;
use std::io;
use std::sync::Arc;
use std::time::{Instant, Duration};

use crossterm::cursor;
use crossterm::{execute,terminal::{Clear, ClearType},};

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Packate {
    source_port: u16,
    destination_port: u16,
    source_ip_address: String,
    destination_ip_address: String,
    window: u16,
    syn_flag: u8,
    fyn_flag: u8,
    sequence_number: u32,
    ack_number: u32,
    message: String,
    fragment_number: u32,
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

    if args.len() < 3 {
        println!("ERRORE NELL'ESECUZIONE DEL PROGRAMMA, PASSARE LE PORTE COME ARGOMENTO");
        return;
    } else if args.len() > 3 {
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
        source_ip_address: my_ip.clone(),
        destination_ip_address: his_ip.clone(),
        window: rng.gen_range(100..=200),
        syn_flag: 1,
        fyn_flag: 0,
        sequence_number: rng.gen_range(100..=1000),
        ack_number: 0,
        message: String::new(),
        fragment_number: 0,
    };

    let mut packate_to_receive = Packate {
        source_port: his_port,
        destination_port: my_port,
        source_ip_address: his_ip.clone(),
        destination_ip_address: my_ip.clone(),
        window: rng.gen_range(100..=200),
        syn_flag: 0,
        fyn_flag: 0,
        sequence_number: 0,
        ack_number: 0,
        message: String::new(),
        fragment_number: 0,
    };

    tokio::spawn(async move {
        handle_keyboard_input(input_clone).await;
    });

    let mut start_time = Instant::now();

    let socket = UdpSocket::bind(format!("{}:{}", my_ip, my_port)).await.unwrap();
    let destination = format!("{}:{}", his_ip, his_port);

    let mut memory_of_packate: Vec<Packate> = Vec::new();
    let mut recevied_packates: Vec<Packate> = Vec::new();

    let mut buf = [0u8; 1024];

    let mut serialized_data = bincode::serialize(&packate_to_send).expect("Failed to serialize");

    let mut done_three_way_hand_shake: u8 = 0;
    let waiting_time_for_multipackate = 5;

    loop {
        tokio::select! {
            result = socket.recv_from(&mut buf) => {
                match result {
                    Ok((size, _)) => {
                        packate_to_receive = bincode::deserialize(&buf[..size]).expect("failed to deserialize");
                        start_time = Instant::now();
                    }
                    Err(e) => eprintln!("Errore durante la ricezione dei dati UDP: {}", e),
                }

                if packate_to_receive.syn_flag == 1 {
                    
                    receiving_three_way_handshake(&socket, &destination, waiting_time, &mut packate_to_send, &mut packate_to_receive, &mut done_three_way_hand_shake).await;

                } else if packate_to_receive.fragment_number==0{
                    print_messages( &mut recevied_packates ).await;
                    println!("mesaggio ricevuto = {}", packate_to_receive.message);
                }else
                {
                    recevied_packates.push( packate_to_receive.clone() );
                }

            }

            _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {
                let mut input = shared_input.lock().await;
                if !input.is_empty() {

                    if done_three_way_hand_shake == 0 {
                        done_three_way_hand_shake = 1;
                        transmitting_three_way_handshake(&socket, &destination, waiting_time, &mut packate_to_send, &mut packate_to_receive).await;
                    }else{
                        packate_to_send.message = input.clone();
                        packate_to_send.syn_flag = 0;
                        println!("");

                        if packate_to_send.message.len() <=10{
                            serialized_data = bincode::serialize(&packate_to_send).expect("Failed to serialize");
                            socket.send_to(&serialized_data, destination.clone()).await.expect("failde to send");
                        }else{
                            split_string(&mut packate_to_send, &destination, &mut memory_of_packate, &socket ).await;
                        }

                        *input = String::new();
                        memory_of_packate.push(packate_to_send.clone());
                    }

                }
            }
        }

        let end_time = start_time.elapsed();

        if end_time.as_secs() >= waiting_time_for_multipackate {
            print_messages( &mut recevied_packates ).await;
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

async fn receiving_three_way_handshake( socket: &tokio::net::UdpSocket, destination: &String, waiting_time: Duration, packate_to_send: &mut Packate, packate_to_receive: &mut Packate, done_three_way_hand_shake: &mut u8) {
    packate_to_send.syn_flag = 1;
    packate_to_send.ack_number = packate_to_receive.sequence_number + 1;
    println!("RICEVUTO MESSAGGIO DI RICHIESTA SINCRONIZZAZIONE, MANDO ACK...");

    let serialized_data = bincode::serialize(&packate_to_send).expect("failed to serialize");
    socket.send_to(&serialized_data, destination.clone()).await.expect("failed to send");

    let mut buf = [0; 1024];
    while packate_to_receive.ack_number != packate_to_send.sequence_number + 1 {
        match timeout(waiting_time, socket.recv_from(&mut buf)).await {
            // Ricezione di dati
            Ok(result) => match result {
                Ok((size, _source)) => {
                    *packate_to_receive = bincode::deserialize(&buf[..size]).unwrap();
                    break; // Esci dal ciclo una volta ricevuto il pacchetto
                }
                // Nessun dato disponibile, continua con il ciclo
                Err(_e) => {}
            },
            // Altro errore, interrompi l'esecuzione
            Err(_e) => {
                println!("NESSUNA RISPOSTA RICEVUTA...\n");
            }
        }
    }

    println!("ACK RICEVUTO, PROCEDO ALLA COMUNICAZIONE...");
    packate_to_send.syn_flag = 0;
    *done_three_way_hand_shake = 1;
}

async fn transmitting_three_way_handshake(socket: &tokio::net::UdpSocket,destination: &String,waiting_time: Duration,packet_to_send: &mut Packate,packet_to_receive: &mut Packate) {
    packet_to_send.syn_flag = 1;

    let mut buf = [0; 1024];
    let serialized_data = bincode::serialize(&packet_to_send).expect("failed to serialize");

    while packet_to_receive.syn_flag == 0 {
        socket.send_to(&serialized_data, destination).await.expect("Failed to send SYN packet");
        println!("INVIO PACCHETTO DI SINCRONIZZAZIONE...");

        // Attendi la risposta
        match timeout(waiting_time, socket.recv_from(&mut buf)).await {
            // Ricezione di dati
            Ok(result) => match result {
                Ok((size, _source)) => {
                    *packet_to_receive = bincode::deserialize(&buf[..size]).unwrap();
                    break; // Esci dal ciclo una volta ricevuto il pacchetto
                }
                // Nessun dato disponibile, continua con il ciclo
                Err(_e) => {}
            },
            // Altro errore, interrompi l'esecuzione
            Err(_e) => {
                println!("NESSUNA RISPOSTA RICEVUTA...\n");
            }
        }
    }

    println!("RICEVUTA CONFERMA DI ACK PER IL MIO MESSAGGIO, RISPONDO CON ACK PER CONFERMARE...");

    packet_to_send.syn_flag = 0;
    packet_to_send.ack_number = packet_to_receive.sequence_number + 1;
    packet_to_send.sequence_number = 0;

    let serialized_data = bincode::serialize(&packet_to_send).expect("failed to serialize");

    socket.send_to(&serialized_data, destination).await.expect("Failed to send ACK packet");
}

async fn split_string(to_send: &mut Packate,dest: &String,memory: &mut Vec<Packate>,sock: &tokio::net::UdpSocket) {
    let mut sub_pack = to_send.clone();
    let s = sub_pack.message.clone();
    let mut i = 1;

    let mut start = 0;

    while start < s.len() {
        let end = (start + 10).min(s.len());
        sub_pack.message = s[start..end].to_string();
        start = end;
        sub_pack.fragment_number = i;
        i += 1;
        memory.push(sub_pack.clone());

        let serialized = bincode::serialize(&sub_pack).expect("Failed to serialize");
        sock.send_to(&serialized, dest.clone()).await.expect("failde to send");
    }
}
            
async fn print_messages( vec: &mut Vec<Packate> ){
    let len = vec.len();
    for i in 0..len {
        for j in 0..len - i - 1 {
            if vec[j].sequence_number > vec[j + 1].sequence_number {
                vec.swap(j, j + 1);
            }
        }
    }

    for packate in & mut *vec{
        print!("{}", packate.message);
    }
    vec.clear();
}
