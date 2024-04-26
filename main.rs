use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::net::UdpSocket;

use tokio::io::AsyncBufReadExt;

#[tokio::main]
async fn main() {

    // Creazione della stringa condivisa protetta da Mutex per l'input dalla tastiera
    let shared_input = Arc::new(Mutex::new(String::new()));

    // Clonazione del riferimento condiviso per passarlo alla funzione handle_keyboard_input
    let input_clone = Arc::clone(&shared_input);

    let mut other_port = 0, other_ip = 0, other_window = 0;
    let mut expected_sequence_number;


    // Avvio della gestione dell'input dalla tastiera in un task asincrono
    tokio::spawn(async move {
        handle_keyboard_input(input_clone).await;
    });

    // Creazione del socket UDP
    let socket = UdpSocket::bind("127.0.0.1:2000").await.unwrap();

    // Buffer per la ricezione dei dati dal socket UDP
    let mut buf = [0u8; 1024];

    loop {


        tokio::select! {
            // Gestione dell'input UDP
            result = socket.recv_from(&mut buf) => {
                match result {
                    Ok((num_bytes, addr)) => {
                        
                        
                        let message = std::str::from_utf8(&buf[..num_bytes]).unwrap();
                        println!("Ricevuto {} byte dal socket UDP da {}: {}", num_bytes, addr, message);
                        
                        
                        
                        
                        let output_handle_string: u8 = handle_string(message).await;
                        println!("{}",output_handle_string);
                        // Puoi fare qualcosa con il messaggio ricevuto qui
                    }




                    Err(e) => eprintln!("Errore durante la ricezione dei dati UDP: {}", e),
                }
            }

            // Gestione dell'input dalla tastiera
            _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {
                let mut input = shared_input.lock().await;
                if !input.is_empty() {
                    println!("Input dalla tastiera: {}", *input);
                    // Pulisci la stringa condivisa
                    *input = String::new();
                }
            }
            
        }

    
    }

}


async fn handle_string( string_to_handle: &str) -> u8{
    let fields: Vec<&str> ;

    if( string_to_handle[0] == '|' ){

        fields = string_to_handle.split("|").collect();
        

    }else if( string_to_handle[1] == '%' ){

        fields = string_to_handle.split("%").collect();

    }



    let x:u8 = 0;
    x
}


async fn handle_keyboard_input(shared_input: Arc<Mutex<String>>) {
    // Ottiene l'input dalla tastiera
    loop {
        let stdin = tokio::io::stdin();
        let mut reader = tokio::io::BufReader::new(stdin);

        let mut line = String::new();
        if let Ok(_) = reader.read_line(&mut line).await {
            // Scrivi l'input nella stringa condivisa
            let mut input = shared_input.lock().await;
            *input = line;
        } else {
            eprintln!("Errore durante la lettura dell'input dalla tastiera");
        }
    }
}


