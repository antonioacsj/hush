use crossbeam_channel::{unbounded, Receiver, Sender};
use log::{info, warn, LevelFilter};
use num_cpus;
use once_cell::sync::OnceCell;
use sha2::{Digest, Sha256};
use std::env;
use std::fs::{self, File};
use std::io::BufRead;
use std::io::{self, BufReader, Read, Seek, Write};
use std::path::Path;
use std::process;
use std::result::Result;
use std::thread;
use std::time::Instant;

static LOG_ENABLED: OnceCell<bool> = OnceCell::new();
/*
const chunk_size: u64 = 50 * 1024 * 1024; // 50 MB
const BUFFER_SIZE: usize = 8 * 1024; // 8 KB
*/

fn parse_size(size_str: &str) -> Result<u64, &'static str> {
    // Remove any whitespace and convert the string to uppercase
    let size_str = size_str.trim().to_uppercase();

    // Find where the numeric part ends and the unit starts
    let mut number_part = String::new();
    let mut unit_part = String::new();

    for ch in size_str.chars() {
        if ch.is_numeric() {
            number_part.push(ch);
        } else {
            unit_part.push(ch);
        }
    }

    // Parse the numeric part
    let number: u64 = number_part.parse().map_err(|_| "Invalid number")?;

    // Match the unit and convert to bytes
    match unit_part.as_str() {
        "B" => Ok(number),
        "KB" => Ok(number * 1_024),
        "MB" => Ok(number * 1_024 * 1_024),
        "GB" => Ok(number * 1_024 * 1_024 * 1_024),
        "TB" => Ok(number * 1_024 * 1_024 * 1_024 * 1_024),
        _ => Err("Unknown unit"),
    }
}

fn log_progresso(str_log: String) {
    if *LOG_ENABLED.get().unwrap_or(&false) {
        print!("{}", str_log);
    }
}

fn hash_rsha256(
    file_path: &str,
    buffer_size: usize,
    chunk_size: usize,
    chunk_size_str: String,
    check_hash: bool,
    hash_to_check: String,
) -> io::Result<()> {
    let hash_alg = "RSHA256".to_string();

    info!(
        "alg:{} file: {} BlokSize:{} BufferSize:{} ",
        hash_alg, file_path, chunk_size, buffer_size
    );

    let blocos = calcular_blocos(file_path, buffer_size, chunk_size)?; // Função que calcula os blocos

    let (sender_calculo, receiver_calculo): (Sender<ChunkBloco>, Receiver<ChunkBloco>) =
        unbounded();
    let (sender_resultado, receiver_resultado): (Sender<ChunkBloco>, Receiver<ChunkBloco>) =
        unbounded();

    // Enviar os blocos para o canal de cálculo
    for bloco in blocos {
        //  println!("Enviado Bloco {} -> p/ calculo", bloco.n_bloco);
        log_progresso("+".to_string());
        io::stdout().flush().expect("Failed to flush stdout");
        sender_calculo.send(bloco).unwrap();
    }
    drop(sender_calculo); // Dropar após envio

    let num_cpus = num_cpus::get();

    // Criar threads para calcular o hash dos blocos usando BufReader
    let mut handles = Vec::new();
    for n_cpu in 0..num_cpus {
        //let file_path_clone: String = file_path.to_string();
        info!("Cpu {} start running", n_cpu);
        let file_path_clone: String = String::from(file_path);

        let file = File::open(file_path)?;
        let receiver_calculo = receiver_calculo.clone();
        let sender_resultado = sender_resultado.clone();
        let mut reader = BufReader::new(file); // BufReader associado ao descritor clonado

        let handle = thread::spawn(move || {
            while let Ok(bloco) = receiver_calculo.recv() {
                let n_bloco = bloco.n_bloco;
                //println!("Recebido Bloco {} para cálculo de hash", n_bloco);
                //calcular_hash_bloco(&mut reader, &mut bloco).unwrap(); // Passar o BufReader para calcular o hash
                match calcular_hash_bloco(&mut reader, bloco, buffer_size, chunk_size) {
                    Ok(bloco_recebido) => {
                        log_progresso("*".to_string());
                        io::stdout().flush().expect("Failed to flush stdout");
                        sender_resultado.send(bloco_recebido).unwrap();
                        //  println!("Bloco {} calculado e enviado para resultado", n_bloco);
                    }
                    Err(e) => {
                        println!(
                            "Error Rush Block: {}: {:?} File:{} ",
                            n_bloco, e, file_path_clone
                        );
                        process::exit(1);
                    }
                }
            }
        });
        handles.push(handle);
    }

    drop(sender_resultado); // Dropar após o término das threads

    // Coletar os resultados dos hashes
    let mut resultados = Vec::new();
    while let Ok(bloco) = receiver_resultado.recv() {
        //  println!("Recebido Bloco {} no resultado", bloco.n_bloco);
        log_progresso("-".to_string());
        io::stdout().flush().expect("Failed to flush stdout");
        resultados.push(bloco);
    }

    // Esperar todas as threads de cálculo terminarem
    for handle in handles {
        handle.join().unwrap();
    }

    // Ordenar os resultados para garantir a ordem correta
    resultados.sort_by_key(|bloco| bloco.n_bloco);

    // Imprimir os resultados ordenados
    log_progresso("\n".to_string());

    let hash_final: String;
    if resultados.len() == 1 {
        let bloco = &resultados[0];
        hash_final = bloco.hash_bloco.clone();
    } else {
        let mut hasher_cumulativo = Sha256::new();
        info!("Block;bytes_start;byte_end;hash");
        for bloco in resultados {
            info!(
                "{0:10};{1:10};{2:10};{3}",
                bloco.n_bloco, bloco.inicio_bloco, bloco.fim_bloco, bloco.hash_bloco
            );
            hasher_cumulativo.update(bloco.hash_bloco.as_bytes());
        }
        let hash_cumulativo_result = hasher_cumulativo.finalize();
        hash_final = format!("{:x}", hash_cumulativo_result);
    }

    let safe_hash_final = String::from_utf8_lossy(hash_final.as_bytes());
    let safe_hash_alg = String::from_utf8_lossy(hash_alg.as_bytes());
    let safe_chunk_size_str = String::from_utf8_lossy(chunk_size_str.as_bytes());
    let safe_file_path = String::from_utf8_lossy(file_path.as_bytes());
    println!(
        "{} ?{}|{}*{}",
        safe_hash_final, safe_hash_alg, safe_chunk_size_str, safe_file_path
    );

    if check_hash {
        if hash_to_check.trim().to_lowercase() == hash_final.trim().to_lowercase() {
            println!("Sucess. Hashes matched!");
        } else {
            // Erro na checagem
            println!("Error. Does not match!");
            println!("  Hash to check: {}", hash_to_check);
            println!("Hash Calculated: {}", hash_final);
        }
    }

    Ok(())
}

fn hash_sha256(file_path: &str, buffer_size: usize) -> io::Result<String> {
    // Abrindo o arquivo para leitura
    let mut file = File::open(file_path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0; buffer_size];

    // Lendo o arquivo em blocos e alimentando o hasher
    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break; // Fim do arquivo
        }
        hasher.update(&buffer[..bytes_read]);
    }

    // Calculando o hash final e convertendo para string hexadecimal
    let hash_result = hasher.finalize();
    Ok(format!("{:x}", hash_result))
}
// Função que reconstrói o arquivo a partir das partes, usando um buffer menor
fn rebuild(
    dir_destino: &str,
    file: &str,
    buffer_size: usize,
    _chunk_size: usize,
) -> io::Result<()> {
    // Listando todos os arquivos no diretório
    let mut parts: Vec<_> = fs::read_dir(dir_destino)?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            let filename = path.file_name()?.to_str()?.to_string();

            // Verificando se o nome do arquivo termina com "_<número>"
            if filename.contains("_") {
                Some((path, filename))
            } else {
                None
            }
        })
        .collect();

    // Ordenando os arquivos com base no sufixo numérico
    parts.sort_by_key(|(_, filename)| {
        filename
            .rsplit('_')
            .next()
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0)
    });

    // Criando o arquivo final onde o conteúdo será reconstituído
    let mut output_file = File::create(file)?;
    let mut buffer = vec![0; buffer_size];

    // Lendo os arquivos de pedaço na ordem e escrevendo no arquivo final
    for (path, _) in parts {
        let mut part_file = File::open(path)?;

        // Lendo o arquivo de pedaço em blocos pequenos e escrevendo no arquivo final
        loop {
            let bytes_read = part_file.read(&mut buffer)?;
            if bytes_read == 0 {
                break; // Fim do pedaço
            }
            output_file.write_all(&buffer[..bytes_read])?;
        }
    }

    Ok(())
}

// Função que divide o arquivo original em partes de 20 MB, calculando o hash SHA-256 de cada parte
fn split(file: &str, dir_destino: &str, buffer_size: usize, chunk_size: usize) -> io::Result<()> {
    // Abrindo o arquivo original para leitura
    let mut input_file = File::open(file)?;
    let mut buffer = vec![0; buffer_size];
    let mut index = 0;
    let mut bytes_written = 0;

    let path_dir = Path::new(dir_destino);

    if !path_dir.exists() {
        // Cria o diretório, incluindo diretórios intermediários, se necessário
        fs::create_dir_all(path_dir)?;
        println!("Dir created: {}", dir_destino);
    }

    // Nomeando o primeiro arquivo de saída
    let mut part_filename = format!(
        "{}/{}_{}",
        dir_destino,
        Path::new(file).file_name().unwrap().to_str().unwrap(),
        index
    );
    let mut part_file = File::create(&part_filename)?;
    let mut hasher = Sha256::new();

    // Lendo o arquivo em pedaços pequenos e escrevendo nos arquivos menores
    loop {
        let bytes_read = input_file.read(&mut buffer)?;
        if bytes_read == 0 {
            break; // Fim do arquivo
        }

        // Escrevendo o buffer no arquivo de saída atual
        part_file.write_all(&buffer[..bytes_read])?;
        hasher.update(&buffer[..bytes_read]);
        bytes_written += bytes_read;

        // Se o tamanho total escrito atingir o limite do pedaço, calcular o hash e passar para o próximo arquivo
        if bytes_written >= chunk_size as usize {
            // Calcular o hash SHA-256 do bloco
            let hash_result = hasher.finalize();
            let hash_hex = format!("{:x}", hash_result);

            // Escrever o hash em um arquivo com a extensão .sha256
            let hash_filename = format!("{}.sha256.txt", part_filename);
            let mut hash_file = File::create(&hash_filename)?;
            hash_file.write_all(hash_hex.as_bytes())?;

            // Reiniciar para o próximo bloco
            index += 1;
            bytes_written = 0;
            part_filename = format!(
                "{}/{}_{}",
                dir_destino,
                Path::new(file).file_name().unwrap().to_str().unwrap(),
                index
            );
            part_file = File::create(&part_filename)?;
            hasher = Sha256::new(); // Reiniciar o hasher para o próximo bloco
        }
    }

    // Calcular e salvar o hash do último bloco se ele não for vazio
    if bytes_written > 0 {
        let hash_result = hasher.finalize();
        let hash_hex = format!("{:x}", hash_result);
        let hash_filename = format!("{}.sha256.txt", part_filename);
        let mut hash_file = File::create(&hash_filename)?;
        hash_file.write_all(hash_hex.as_bytes())?;
    }

    Ok(())
}

// Função que divide o arquivo original em partes de 20 MB, usando um buffer menor
fn divide_sem_hash(
    file: &str,
    dir_destino: &str,
    buffer_size: usize,
    chunk_size: usize,
) -> io::Result<()> {
    // Abrindo o arquivo original para leitura
    let mut input_file = File::open(file)?;
    let mut buffer = vec![0; buffer_size];
    let mut index = 0;
    let mut bytes_written = 0;

    // Nomeando o primeiro arquivo de saída
    let mut part_filename = format!(
        "{}/{}_{}",
        dir_destino,
        Path::new(file).file_name().unwrap().to_str().unwrap(),
        index
    );
    let mut part_file = File::create(&part_filename)?;

    // Lendo o arquivo em pedaços pequenos e escrevendo nos arquivos menores
    loop {
        let bytes_read = input_file.read(&mut buffer)?;
        if bytes_read == 0 {
            break; // Fim do arquivo
        }

        // Escrevendo o buffer no arquivo de saída atual
        part_file.write_all(&buffer[..bytes_read])?;
        bytes_written += bytes_read;

        // Se o tamanho total escrito atingir o limite do pedaço, passar para o próximo arquivo
        if bytes_written >= chunk_size as usize {
            index += 1;
            bytes_written = 0;
            part_filename = format!(
                "{}/{}_{}",
                dir_destino,
                Path::new(file).file_name().unwrap().to_str().unwrap(),
                index
            );
            part_file = File::create(&part_filename)?;
        }
    }

    Ok(())
}

// Estrutura para armazenar as informações de cada bloco
#[derive(Debug, Clone)]

struct ChunkBloco {
    n_bloco: u64,
    inicio_bloco: u64,
    fim_bloco: u64,
    hash_bloco: String,
}

// Função para calcular os blocos de 50 MB de um arquivo

fn calcular_blocos(
    file_path: &str,
    _buffer_size: usize,
    chunk_size: usize,
) -> io::Result<Vec<ChunkBloco>> {
    let file = File::open(file_path)?;
    let file_size = file.metadata()?.len();

    let mut blocos = Vec::new();
    let mut inicio_bloco = 0;
    let mut n_bloco = 0;

    // Dividindo o arquivo em blocos de 50 MB
    while inicio_bloco < file_size {
        let fim_bloco = std::cmp::min(inicio_bloco + chunk_size as u64, file_size);
        blocos.push(ChunkBloco {
            n_bloco,
            inicio_bloco,
            fim_bloco,
            hash_bloco: String::new(),
        });
        inicio_bloco = fim_bloco;
        n_bloco += 1;
    }

    Ok(blocos)
}

fn calcular_hash_bloco(
    reader: &mut BufReader<File>,
    bloco: ChunkBloco,
    buffer_size: usize,
    _chunk_size: usize,
) -> Result<ChunkBloco, io::Error> {
    let mut hasher = Sha256::new();
    reader.seek(io::SeekFrom::Start(bloco.inicio_bloco))?; // Ir até o offset do bloco

    let mut buffer = vec![0; buffer_size];
    let mut bytes_restantes = bloco.fim_bloco - bloco.inicio_bloco;

    while bytes_restantes > 0 {
        let bytes_a_ler = std::cmp::min(buffer.len() as u64, bytes_restantes) as usize;
        match reader.read(&mut buffer[..bytes_a_ler]) {
            Ok(0) => break, // EOF
            Ok(n) => {
                hasher.update(&buffer[..n]); // Atualiza o hash com os dados lidos
                bytes_restantes -= n as u64;
            }
            Err(e) => return Err(e), // Retorna um erro se a leitura falhar
        }
    }

    // Gerar o hash do bloco
    let result = hasher.finalize();
    Ok(ChunkBloco {
        n_bloco: bloco.n_bloco,
        inicio_bloco: bloco.inicio_bloco,
        fim_bloco: bloco.fim_bloco,
        hash_bloco: format!("{:x}", result),
    })
}

fn parse_line(line: &str) -> Option<(String, String, String, String)> {
    // Split the line by the specific characters ? | *
    let parts: Vec<&str> = line.split(&['?', '|', '*'][..]).collect();

    // Check if we have exactly 4 parts
    if parts.len() == 4 {
        let hash_final = parts[0].to_string();
        let hash_alg = parts[1].to_string();
        let chunk_size_str = parts[2].to_string();
        let file_path = parts[3].to_string();
        Some((hash_final, hash_alg, chunk_size_str, file_path))
    } else {
        None
    }
}

fn read_and_parse_file(file_path: &str) -> io::Result<()> {
    let path = Path::new(file_path);
    let file = File::open(&path)?;
    let reader = BufReader::new(file);

    // Read the file line by line
    for line_result in reader.lines() {
        match line_result {
            Ok(line) => {
                // Parse each line
                if let Some((hash_final, hash_alg, chunk_size_str, file_path)) = parse_line(&line) {
                    // Print or use the variables
                    println!("Hash Final: {}", hash_final);
                    println!("Hash Algorithm: {}", hash_alg);
                    println!("Chunk Size: {}", chunk_size_str);
                    println!("File Path: {}", file_path);

                    let buffer_size_padrao = 8 * 1024 as usize; // 8K
                    let check_hash = true;
                    let hash_to_check = hash_final.trim().to_lowercase();
                    let chunk_size = parse_size(&chunk_size_str).unwrap() as usize;
                    if let Err(e) = hash_rsha256(
                        &file_path,
                        buffer_size_padrao,
                        chunk_size,
                        chunk_size_str,
                        check_hash,
                        hash_to_check,
                    ) {
                        eprintln!("r-sha256 error: {}", e);
                        process::exit(1);
                    }
                } else {
                    println!("Invalid line format: {}", line);
                }
            }
            Err(err) => {
                // Handle the error if reading the line fails
                println!("Error reading line: {}", err);
            }
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Coletando os argumentos da linha de comando
    let start = Instant::now();
    let args: Vec<String> = env::args().collect();

    // Pega

    let mut chunk_size_str = "50MB"; // 50 MB
    let mut chunk_size: usize = parse_size(chunk_size_str)? as usize; // 50 MB

    let mut buffer_size_str = "10KB"; // 50 MB
    let mut buffer_size: usize = parse_size(buffer_size_str)? as usize; // 50 MB

    if args.len() < 3 {
        eprintln!("Use: {} <command> <file_path> <dest_folder_path>?", args[0]);
        eprintln!("Commands: 'rsha256','split','rebuild','sha256' ");
        eprintln!("Option: '--log' to print logs");
        eprintln!(
            "Option: '--blocksize Value' to change size that file block is divided. Default {}. Use KB, MB, GB, TB",
            chunk_size_str
        );
        eprintln!(
            "Option: '--buffersize Value' to change buffersize. Default {}. Use KB, MB, GB, TB",
            buffer_size_str
        );

        process::exit(1);
    }

    if let Some(chunksize_index) = args.iter().position(|x| x == "--blocksize") {
        if let Some(size_str) = args.get(chunksize_index + 1) {
            chunk_size_str = size_str;
            chunk_size = parse_size(size_str)? as usize;
        } else {
            eprintln!("--blocksize provided without a value.  Use KB, MB, GB, TB");
            return Ok(());
        }
    }

    if let Some(buffersize_index) = args.iter().position(|x| x == "--buffersize") {
        if let Some(size_str) = args.get(buffersize_index + 1) {
            buffer_size_str = size_str;
            buffer_size = parse_size(size_str)? as usize;
        } else {
            eprintln!("--buffersize provided without a value.  Use KB, MB, GB, TB");
            return Ok(());
        }
    }

    let enable_logging = args.contains(&"--log".to_string());
    if enable_logging {
        LOG_ENABLED
            .set(true)
            .expect("LOG_ENABLED can only be set once");
        env_logger::Builder::new()
            .filter_level(LevelFilter::Info) // Set logging level (Info, Warn, etc.)
            .init();
        info!("Logging is enabled in the main thread.");
    }

    // Extraindo os argumentos
    let comando = &args[1];
    let file_path = &args[2];

    // Executando a função correspondente com base no comando fornecido
    match comando.as_str() {
        "split" => {
            if args.len() != 4 {
                eprintln!("Use: {} split <file_path> <dest_folder_path>?", args[0]);
                eprintln!("<file_path>: file to split.");
                eprintln!("<file_path>: folder to receive splitted files.");

                process::exit(1);
            }

            let dir_destino = &args[3];

            if let Err(e) = split(file_path, dir_destino, buffer_size, chunk_size) {
                eprintln!("Split Error: {}", e);
                process::exit(1);
            }
        }
        "rebuild" => {
            if args.len() != 4 {
                eprintln!("Use: {} rebuild <file_path> <dest_folder_path>?", args[0]);
                eprintln!("<file_path>: file to rebuild.");
                eprintln!("<file_path>: folder to read splitted files.");
                process::exit(1);
            }
            let dir_destino = &args[3];

            if let Err(e) = rebuild(dir_destino, file_path, buffer_size, chunk_size) {
                eprintln!("Rebuild error: {}", e);
                process::exit(1);
            }
        }
        "rsha256" => {
            let check_hash = false;
            let hash_to_check = "".to_string();
            if let Err(e) = hash_rsha256(
                file_path,
                buffer_size,
                chunk_size,
                chunk_size_str.to_string(),
                check_hash,
                hash_to_check,
            ) {
                eprintln!("rsha256 error: {}", e);
                process::exit(1);
            }
        }
        "check" => {
            if let Err(e) = read_and_parse_file(file_path) {
                eprintln!("check error: {}", e);
                process::exit(1);
            }
        }

        "sha256" => {
            if let Err(e) = hash_sha256(file_path, buffer_size) {
                eprintln!("sha256 error: {}", e);
                process::exit(1);
            }
        }
        _ => {
            eprintln!("Use: {} <command> <file_path> <dest_folder_path>?", args[0]);
            eprintln!("Commands: 'rsha256','split','rebuild','sha256' ");
            process::exit(1);
        }
    }
    let duration = start.elapsed();
    info!("Execution Time: {:?}", duration);
    Ok(())
}
