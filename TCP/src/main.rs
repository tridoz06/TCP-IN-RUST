use serde::{Deserialize, Serialize};

use tokio::io::AsyncBufReadExt;
use tokio::net::UdpSocket;
use tokio::sync::Mutex;
use tokio::time::timeout;

use rand::Rng;

use std::env;
use std::io;
use std::sync::Arc;
use std::time:: {Instant, Duration};
use std::mem;
use crossterm::cursor;
use crossterm::{execute,terminal::{Clear, ClearType},};

#[derive(Debug, Serialize, Deserialize, Clone)]
//dimensiome minima pacchetto 32 byte
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
    packate_size: u32,
    checksum: u16,
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

    let mut start_time_for_receiving = Instant::now();
    let mut start_time_for_ack = Instant::now();

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
        packate_size: 0,
        checksum:0,
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
        packate_size: 0,
        checksum:0,
    };

    tokio::spawn(async move {
        handle_keyboard_input(input_clone).await;
    });

    let socket = UdpSocket::bind(format!("{}:{}", my_ip, my_port)).await.unwrap();
    let destination = format!("{}:{}", his_ip, his_port);

    let dimensiome_minima_pacchetto:u32 = 26;

    let mut memory_of_packate: Vec<Packate> = Vec::new();
    let mut recevied_packates: Vec<Packate> = Vec::new();

    let mut buf = [0u8; 1024];

    let mut serialized_data = bincode::serialize(&packate_to_send).expect("Failed to serialize");

    let mut done_three_way_hand_shake: u8 = 0;
    let _waiting_time_for_multipackate = 5;

    let mut initial_sequence_number:u32 = 0;
    let mut expected_sequence_number:u32 = 0;
    let mut current_sequence_number: u32 = packate_to_send.sequence_number;

    loop {
        tokio::select! {
            
            result = socket.recv_from(&mut buf) => {
                match result {
                    Ok((size, _)) => {
                        packate_to_receive = bincode::deserialize(&buf[..size]).expect("failed to deserialize");
                        start_time_for_receiving = Instant::now();
                        expected_sequence_number += packate_to_receive.packate_size;
                    }
                    Err(e) => eprintln!("Errore durante la ricezione dei dati UDP: {}", e),
                }

                if packate_to_receive.syn_flag == 1 {

                    expected_sequence_number = packate_to_receive.sequence_number ;
                    receiving_three_way_handshake(&socket, &destination, waiting_time, &mut packate_to_send, &mut packate_to_receive, &mut done_three_way_hand_shake, &mut initial_sequence_number).await;

                }else if packate_to_receive.ack_number == current_sequence_number{
                    println!("\nACK RICEVUTO SUI MESSAGGI PRECEDENTI\n");
                    memory_of_packate.clear();
                    start_time_for_ack = Instant::now();
                } else {
                    recevied_packates.push( packate_to_receive.clone() );
                }

            }

            _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {
                let mut input = shared_input.lock().await;
               // println!("parola = <{}>, <{}>",input, input.is_empty());
                if !input.is_empty() {

                    //println!("CURRENT SEQUENCE NUMBER = {}", current_sequence_number);

                    if done_three_way_hand_shake == 0 {
                        done_three_way_hand_shake = 1;
                        transmitting_three_way_handshake(&socket, &destination, waiting_time, &mut packate_to_send, &mut packate_to_receive, &mut initial_sequence_number).await;
                    }else{

                        start_time_for_ack = Instant::now();
                        packate_to_send.message = input.clone();
                        packate_to_send.syn_flag = 0;


                        if packate_to_send.message.len() <= 10 {

                            packate_to_send.packate_size = dimensiome_minima_pacchetto + packate_to_send.source_ip_address.len() as u32 + packate_to_send.destination_ip_address.len() as u32 + packate_to_send.message.len() as u32;
                            packate_to_send.ack_number = 0;
                            packate_to_send.sequence_number = current_sequence_number;
                            current_sequence_number += packate_to_send.packate_size;

                            serialized_data = bincode::serialize(&packate_to_send).expect("Failed to serialize");
                            socket.send_to(&serialized_data, destination.clone()).await.expect("failde to send");
                            memory_of_packate.push(packate_to_send.clone());

                        }else{
                            split_string(&mut packate_to_send, &destination, &mut memory_of_packate, &socket, &mut current_sequence_number ).await;
                        }

                        *input = String::new();
                        
                    }

                }
            }
        }

        let end_time_for_receiving = start_time_for_receiving.elapsed();
        let end_time_for_ack = start_time_for_ack.elapsed();

        if end_time_for_receiving.as_secs() >= 5 && recevied_packates.len() > 0{
            
            if print_messages(&mut recevied_packates).await == 0{
                packate_to_send.message = String::from("");
                packate_to_send.ack_number = expected_sequence_number;
                serialized_data = bincode::serialize(&packate_to_send).expect("failed to serielize");
                socket.send_to( &serialized_data, destination.clone()).await.expect("failed to send ack back");
                packate_to_send.ack_number = 0;
                start_time_for_receiving = Instant::now();
            }
        }

        if end_time_for_ack.as_secs() >= 10 && memory_of_packate.len() > 0 {
            println!("\nRIMANDO BUFFER\n");
            send_buffer(&socket, &destination, &mut memory_of_packate).await;
            start_time_for_ack = Instant::now();
        }
    }
}

async fn send_buffer(socket: &tokio::net::UdpSocket, destination: &String, mem: &mut Vec<Packate>){
    for i in 0..mem.len(){
        let serialized_data = bincode::serialize(&mem[i]).expect("failed to serialize");
        socket.send_to(&serialized_data, destination.clone()).await.expect("failed to send");
    }
}

async fn calculate_checksum(packate_to_calculate_checksum: &mut Packate) {
    let data = bincode::serialize(packate_to_calculate_checksum).expect("failed to serialize");
    let mut sum:u32 = 0;

    for i in (0..data.len()).step_by(2) {
        let word = (data[i] as u16) << 8 | (data[i + 1] as u16);
        sum += word as u32;
    }

    // Aggiungi eventualmente l'ultimo byte se la lunghezza dei dati è dispari
    if data.len() % 2 != 0 {
        sum += (data[data.len() - 1] as u16) as u32;
    }

    // Somma i carry
    while sum >> 16 != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }

    // Calcola il complemento a uno del risultato
    packate_to_calculate_checksum.checksum = !(sum as u16) as u16;
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

async fn receiving_three_way_handshake( socket: &tokio::net::UdpSocket, destination: &String, waiting_time: Duration, packate_to_send: &mut Packate, packate_to_receive: &mut Packate, done_three_way_hand_shake: &mut u8, initial_sequence_number: &mut u32) {
    packate_to_send.syn_flag = 1;
    packate_to_send.ack_number = packate_to_receive.sequence_number + 1;
    println!("RICEVUTO MESSAGGIO DI RICHIESTA SINCRONIZZAZIONE, MANDO ACK...");
    *initial_sequence_number =  packate_to_receive.sequence_number;

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

async fn transmitting_three_way_handshake(socket: &tokio::net::UdpSocket,destination: &String,waiting_time: Duration,packate_to_send: &mut Packate,packate_to_receive: &mut Packate, initial_sequence_number: &mut u32) {
    packate_to_send.syn_flag = 1;

    let mut buf = [0; 1024];
    let serialized_data = bincode::serialize(&packate_to_send).expect("failed to serialize");

    while packate_to_receive.syn_flag == 0 {
        socket.send_to(&serialized_data, destination).await.expect("Failed to send SYN packet");
        println!("INVIO PACCHETTO DI SINCRONIZZAZIONE...");

        match timeout(waiting_time, socket.recv_from(&mut buf)).await {

            Ok(result) => match result {
                Ok((size, _source)) => {
                    *packate_to_receive = bincode::deserialize(&buf[..size]).unwrap();
                    break;
                }

                Err(_e) => {}
            },

            Err(_e) => {
                println!("NESSUNA RISPOSTA RICEVUTA...\n");
            }
        }
    }

    println!("RICEVUTA CONFERMA DI ACK PER IL MIO MESSAGGIO, RISPONDO CON ACK PER CONFERMARE...");
    *initial_sequence_number = packate_to_receive.sequence_number;

    packate_to_send.syn_flag = 0;
    packate_to_send.ack_number = packate_to_receive.sequence_number + 1;
    packate_to_send.sequence_number = 0;

    let serialized_data = bincode::serialize(&packate_to_send).expect("failed to serialize");

    socket.send_to(&serialized_data, destination).await.expect("Failed to send ACK packet");
}

async fn split_string(to_send: &mut Packate, dest: &String, memory: &mut Vec<Packate>, sock: &tokio::net::UdpSocket, sn: &mut u32) {

    let mut sub_pack = to_send.clone();
    let s = sub_pack.message.clone();
    let dimensiome_minima_pacchetto: u32 = 26;
    let mut start = 0;

    while start < s.len() {
        let end = (start + 10).min(s.len());

        sub_pack.message = s[start..end].to_string();
        sub_pack.sequence_number = *sn;
        sub_pack.packate_size = dimensiome_minima_pacchetto + sub_pack.destination_ip_address.len() as u32 + sub_pack.source_ip_address.len() as u32 + sub_pack.message.len() as u32;
        *sn +=  sub_pack.packate_size;

        memory.push(sub_pack.clone());

        //println!("SEQUENCE NUMBER = {} ; PACKATE SIZE = {}", sub_pack.sequence_number, sub_pack.packate_size);
        let serialized = bincode::serialize(&sub_pack).expect("Failed to serialize");
        sock.send_to(&serialized, dest.clone()).await.expect("failde to send");

        // Aggiorna start per puntare alla prossima parte della stringa
        start = end;
    }
}

            
async fn print_messages( vec: &mut Vec<Packate> ) -> u8{
    let len = vec.len();

    for i in 0..len {
        for j in 0..len - i - 1 {
            if vec[j].sequence_number > vec[j + 1].sequence_number {
                vec.swap(j, j + 1);
            }
        }
    }
    

    for i in 0..len - 1 {
       // println!("\nSEQUENCE NUMBER = {} ; PACKATE SIZE = {} ; NEXT SEQUENCE NUMBER = {} ; NESSAGE = {}\n", vec[i].sequence_number, vec[i].packate_size , vec[i+1].sequence_number, vec[i].message);
        
        if ( vec[i].sequence_number + vec[i].packate_size ) != vec[i+1].sequence_number{
            vec.clear();
            println!("\ntrovato errore nella sequenza dei pacchetti");
            return 255;
        }
    }

    for packate in & mut *vec{
        print!("{}", packate.message);
    }
    vec.clear();
    return 0;
}
