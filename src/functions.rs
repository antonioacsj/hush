use clap::{Arg, Command};

use crossbeam_channel::{unbounded, Receiver, Sender};
use glob::glob;
use log::{info, warn, LevelFilter};
use num_cpus;
use once_cell::sync::OnceCell;
use sha2::{Digest, Sha256};
use std::env;
use std::fs::{self, DirEntry, File};
use std::io::{self, BufRead, BufReader, Read, Seek, Write};
use std::path::Path;
use std::process;
use std::result::Result;
use std::thread;
use std::time::Instant;
use std::time::SystemTime;

#[derive(Debug, Clone)]
struct ChunkBloco {
    n_bloco: u64,
    inicio_bloco: u64,
    fim_bloco: u64,
    hash_bloco: String,
}

pub struct Argumentos {
    pub n_workers: u32,
    pub n_max_concur: u32,
    pub block_size_str: String,
    pub buffer_size_str: String,
    pub block_size: u64,
    pub buffer_size: u32,
    pub log_enabled: bool,
    pub sub_comando: String,
    pub in_file_path: String,
    pub in_file_filter: String,
    pub recursive_enabled: bool,
    pub out_file_path: String,
}

pub fn ParseSize(size_str: &str) -> Result<u64, &'static str> {
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

pub fn hash_rsha256(
    file_path: &str,
    buffer_size: usize,
    chunk_size: usize,
) -> Result<String, Box<dyn std::error::Error>> {
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
        info!("+");
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
        let receiver_calculo_clone = receiver_calculo.clone();
        let sender_resultado_clone = sender_resultado.clone();
        let mut reader = BufReader::new(file); // BufReader associado ao descritor clonado

        let handle = thread::spawn(move || {
            while let Ok(bloco) = receiver_calculo_clone.recv() {
                let n_bloco = bloco.n_bloco;
                //println!("Recebido Bloco {} para cálculo de hash", n_bloco);
                //calcular_hash_bloco(&mut reader, &mut bloco).unwrap(); // Passar o BufReader para calcular o hash
                match calcular_hash_bloco(&mut reader, bloco, buffer_size, chunk_size) {
                    Ok(bloco_recebido) => {
                        info!("*");
                        io::stdout().flush().expect("Failed to flush stdout");
                        sender_resultado_clone.send(bloco_recebido).unwrap();
                        //  println!("Bloco {} calculado e enviado para resultado", n_bloco);
                    }
                    Err(e) => {
                        eprintln!(
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
        info!("-");
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
    info!("\n");

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

    Ok(hash_final)
}

pub fn hash_sha256(
    file_path: &str,
    buffer_size: usize,
) -> Result<String, Box<dyn std::error::Error>> {
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
pub fn rebuild(
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
pub fn split(
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
pub fn divide_sem_hash(
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

pub fn read_and_parse_file(file_path: &str) -> io::Result<()> {
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
                    let chunk_size = ParseSize(&chunk_size_str).unwrap() as usize;
                    if let Err(e) = hash_rsha256(&file_path, buffer_size_padrao, chunk_size) {
                        eprintln!("r-sha256 error: {}", e);
                        process::exit(1);
                    }
                    /*
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
                    */
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
