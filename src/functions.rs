use std::error::Error;
use clap::{Arg, Command};

use crossbeam_channel::{unbounded, Receiver, Sender};
use glob::glob;
use log::{error, warn, info, debug, trace, LevelFilter};
use num_cpus;
use once_cell::sync::OnceCell;
use sha2::{Digest, Sha256};
use std::env;
use std::fs::{self, DirEntry, File};
use std::io::{self, BufRead, BufReader, Read, Seek, Write};
use std::path::{Path, PathBuf};
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
    pub name: String,
    pub n_workers: u32,
    pub n_max_concur: u32,
    pub block_size_str: String,
    pub buffer_size_str: String,    
    pub block_size: u64,
    pub buffer_size: u32,
    pub log_enabled: bool,
    pub sub_comando: String,
    pub in_file_path: PathBuf,
    pub in_file_filter: String,
    pub recursive_enabled: bool,
    pub out_file_path: PathBuf,
    pub flag_show_progress:bool,
    pub flag_stop_on_first_error: bool,
}

pub struct TFileHash {
    n_arquivo: u64,
    path: String,
    alg_hash: String,
    valor_hash: String,
}

pub fn process_files(main_args: Argumentos, files: Vec<String>) {
    let n_files_a_processar = files.len();
    info!(
        "Running subcommand:{} over {} files, with: \n in_file_path: {}, workers: {}, max_concur: {}, block_size: {}, buffer_size: {}, recursive_enabled: {}, filter:{:?}, log_enabled:{}, out_file_path:{}, stop_on_first_error{}, show_progress{}",
        main_args.sub_comando,
        files.len(),
        main_args.in_file_path.display(),
        main_args.n_workers,
        main_args.n_max_concur,
        main_args.block_size_str,
        main_args.buffer_size_str,
        main_args.recursive_enabled,
        main_args.in_file_filter,
        main_args.log_enabled,
        main_args.out_file_path.display(),
        main_args.flag_stop_on_first_error,
        main_args.flag_show_progress

    );
    let caminho_pai_full=main_args.in_file_path.to_str().unwrap().replace("\\", "/").to_string();    
     
    let (sender_files, receiver_files): (Sender<String>, Receiver<String>) = unbounded();

    let (sender_files_calculados, receiver_files_calculados): (
        Sender<TFileHash>,
        Receiver<TFileHash>,
    ) = unbounded();

    //Enviar todos arquivos para o canal que sera processado
    for file in files {
        //  println!("Enviado Bloco {} -> p/ calculo", bloco.n_bloco);
        info!("=>{}", file);
        sender_files.send(file).unwrap();
        if main_args.flag_show_progress {
            eprint!("+");
        }
    }

    drop(sender_files);

    // Criar threads para calcular o hash dos blocos usando BufReader
    let mut handles = Vec::new();
    for n_worker in 0..main_args.n_workers {
        let receiver_files_clone = receiver_files.clone();
        let sender_files_calculados_clone = sender_files_calculados.clone();

        let handle = thread::spawn({
            let block_size_str_clone = main_args.block_size_str.clone();
            move || {
                while let Ok(file_input) = receiver_files_clone.recv() {
                    let arquivo_chegada = file_input.clone();
                  

                    info!("<= {}", arquivo_chegada);
                    // Aqui envia pra calculo de hashe!

                    let mut file_size = 0;

                    match fs::metadata(arquivo_chegada.clone()) {
                        Ok(metadata) => {
                            file_size = metadata.len(); // Get the size in bytes
                        }
                        Err(e) => {
                            eprintln!("Failed to get file metadata: {}", e);
                        }
                    }

                    if file_size > main_args.block_size {
                        // ===> hsha

                        let mut algor_hash_tmp_hsha256_type = String::from("hsha256");
                        let mut algor_hash_tmp_hsha256 =
                            format!("{}-{}", algor_hash_tmp_hsha256_type, block_size_str_clone);

                        info!(
                            "{} -> {}",
                            arquivo_chegada.clone(),
                            algor_hash_tmp_hsha256.clone()
                        );

                        match hash_hsha256(
                            &arquivo_chegada.clone(),
                            main_args.buffer_size as usize,
                            main_args.block_size as usize,
                            main_args.n_max_concur,
                            main_args.flag_show_progress

                        ) {
                            Ok(hash_calculado) => {
                                let fileCalculado = TFileHash {
                                    n_arquivo: 0,
                                    path: arquivo_chegada.clone(),
                                    alg_hash: algor_hash_tmp_hsha256.clone(),
                                    valor_hash: hash_calculado.clone(),
                                };
                                sender_files_calculados_clone.send(fileCalculado).unwrap();
                                info!(
                                    "{} {}?{} ",
                                    arquivo_chegada.clone(),
                                    algor_hash_tmp_hsha256.clone(),
                                    hash_calculado.clone()
                                )
                        
                            }
                            Err(e) => {
                                eprintln!("Error hsha256: {} with: {}", e, arquivo_chegada);
                            }
                        }
                    } else {
                        // ===> SHA256 normal

                        let algor_hash_tmp_sha256 = String::from("sha256");
                        info!(
                            "{} -> {}",
                            arquivo_chegada.clone(),
                            algor_hash_tmp_sha256.clone()
                        );

                        match hash_sha256(
                            &arquivo_chegada.clone(),
                            main_args.buffer_size as usize,
                        ) {
                            Ok(hash_calculado) => {
                                let fileCalculado = TFileHash {
                                    n_arquivo: 0,
                                    path: arquivo_chegada.clone(),
                                    alg_hash: algor_hash_tmp_sha256.clone(),
                                    valor_hash: hash_calculado.clone(),
                                };
                                sender_files_calculados_clone.send(fileCalculado).unwrap();

                                //   println!("{0} ?{1}*{2}",filePronto.valor_hash,filePronto.alg_hash,filePronto.path);

                                info!(
                                    "{} ?{}*{} ",
                                    arquivo_chegada.clone(),
                                    algor_hash_tmp_sha256.clone(),
                                    hash_calculado.clone()
                                )
                            }
                            Err(e) => {
                                eprintln!("Error sha256: {} with: {}", e, arquivo_chegada);
                            }
                        }
                    }
                    if main_args.flag_show_progress {
                        eprint!("*");
                    }
                }
            }
        });
        handles.push(handle);
    }

    // Canal Recebendo os hashes prontos
    drop(sender_files_calculados); // Dropar após o término das threads

    let mut resultados = Vec::new();
    while let Ok(filePronto) = receiver_files_calculados.recv() {
        //  println!("Recebido Bloco {} no resultado", bloco.n_bloco);
        info!("+");
        io::stdout().flush().expect("Failed to flush stdout");
        match gera_caminho_relativo(&filePronto.path.clone(), &caminho_pai_full.clone()) {
            Some(caminho_relativo) => {
                println!(
                    "{0} ?{1}*{2}",
                    filePronto.valor_hash,
                    filePronto.alg_hash,
                    caminho_relativo.display()
                );
            }
            None => {
                eprintln!(
                    "Erro: Pai: {} não é um prefixo de Filho: {}",
                    caminho_pai_full, filePronto.path,
                );
            }
        }
        if(main_args.flag_show_progress){
            eprint!("-");
        }

        resultados.push(filePronto);
    }

    for handle in handles {
        handle.join().unwrap();
    }
    /* */
    //resultados.sort_by_key(|filePronto| filePronto.path);
    let n_files_prontos = resultados.len();
    /*
    for filePronto in resultados {
        println!("{0} {1}?{2}",filePronto.valor_hash,filePronto.alg_hash,filePronto.path);
    }
    */
    eprintln!("Total files to process:{}", n_files_a_processar);
    eprintln!("Total files hashed:{}", n_files_prontos);

    if n_files_prontos == n_files_a_processar {
        eprintln!("Sucess. Hashed all files: {}", n_files_a_processar);
    } else {
        eprintln!(
            "Error. Not all files hashed! To hash:{} / Hashed:{}",
            n_files_a_processar, n_files_prontos
        );
        process::exit(1);
    }
}

pub fn search_files(pattern: &str) -> Result<Vec<String>, io::Error> {
    let mut results = Vec::new();

    // Verifica se o caminho é um arquivo
    if Path::new(pattern).is_file() {
        info!("pattern {} is a file", pattern);
        /*
        results.push(pattern.to_string().replace("\\", "/"));
        return Ok(results); // Retorna já que é um arquivo, não precisa continuar
        */
        let full_path = Path::new(pattern).canonicalize()?; // Obtém o caminho absoluto
        results.push(full_path.to_str().unwrap().replace("\\", "/").to_string());
        return Ok(results); // Retorna já que é um arquivo, não precisa continuar
    }

    let mut glob_pattern = pattern.to_string();
    if Path::new(pattern).is_dir() {
        info!("pattern {} is a directory", pattern);
        glob_pattern = format!("{}/**/*", pattern);        
    }

    info!("Glob to use: {}", glob_pattern);
    for entry in glob(&glob_pattern).expect("Failed to read glob pattern") {
        match entry {
            Ok(path) => {
                if path.is_file() {
                    if let Some(path_str) = path.to_str() {
                        eprintln!("Arquivo Encontrado: {}", path_str);
                        let full_path = Path::new(path_str).canonicalize()?; // Obtém o caminho absoluto
                        eprintln!("Canonizado: {}", full_path.display());
                        let arquivo_full=full_path.to_str().unwrap().replace("\\", "/").to_string();
                        eprintln!("Arquivo Full: {}", arquivo_full);
                        results.push(arquivo_full);

                      //  results.push(path_str.to_string().replace("\\", "/"));
                    }
                }
            }
            Err(e) => {
                eprintln!("Erro ao processar o caminho: {}", e);
                return Err(io::Error::new(io::ErrorKind::InvalidInput, e));
            }
        }
    }
    Ok(results)
}


pub fn gera_caminho_completo(caminho_relativo_in: &str, caminho_pai_in: &str) -> PathBuf {
/*     info!(
        "Caminho relativo chegada*{}*",
        caminho_relativo_in.to_string()
    );
    info!("Caminho Pai chegada*{}*", caminho_pai_in.to_string());
 */
    let mut caminho_pai = caminho_pai_in
        .to_string()
        .replace("\\", "/")
        .trim()
        .to_string();
    let mut caminho_relativo = caminho_relativo_in
        .to_string()
        .replace("\\", "/")
        .trim()
        .to_string();
  /*   info!(
        "Caminho relativo pos chegada*{}*",
        caminho_relativo.to_string()
    );
    info!("Caminho Pai pos chegada*{}*", caminho_pai.to_string()); */

    // Verificar se caminhoPai termina com "/" e adicionar se necessário
    if !caminho_pai.trim_end().ends_with('/') {
        caminho_pai.push('/');
    }
    /*
    // Verificar se caminhoRelativo começa com "." e adicionar se necessário
    if !caminho_relativo.trim_start().starts_with('.') {
        info!("Adicionando '.' no caminho relativo*{}*", caminho_relativo);
        caminho_relativo = format!("./{}", caminho_relativo);
    }
    */
    /**
    if caminho_pai.starts_with('./') {
        Path::new(&caminho_relativo)
    }else{
        */
    // Combinar os caminhos usando PathBuf
    Path::new(&caminho_pai).join(Path::new(&caminho_relativo))
}
pub fn gera_caminho_relativo(caminho_filho: &str, caminho_pai: &str) -> Option<PathBuf> {
    
    let caminho_filho_pb = PathBuf::from(caminho_filho);
    let caminho_pai_pb = PathBuf::from(caminho_pai);
    
    
   // eprintln!("gera_caminho_relativo => caminho1: {} caminho_pai: {}", caminho_filho_pb.display(), caminho_pai_pb.display());
    // Tentar remover o prefixo (caminho pai)
    
    /*
    let caminhoPai_tmp=Path::new(caminho_pai);
    caminhoPai_tmp.strip_prefix('./')
    if !caminho_pai.trim_end().ends_with('/') {
        caminho_pai.push('/');
    }
    */
    
    
    /*
    Erro: AudioRussiancaso/iped/index/_3.si não é um prefixo ./AudioRussiancaso
Erro: AudioRussiancaso/iped/index/write.lock não é um prefixo ./AudioRussiancaso
Erro: AudioRussiancaso/iped/index/segments_1 não é um prefixo ./AudioRussiancaso
Erro: AudioRussiancaso/iped/index/_1.cfs não é um prefixo ./AudioRussiancaso
Erro: AudioRussiancaso/iped/jre/LICENSE não é um prefixo ./AudioRussiancaso

executando:
hush gen ./AudioRussiancaso
    
     */

    match caminho_filho_pb.strip_prefix(caminho_pai_pb) {
        Ok(caminho_relativo) => {
            // Retornar o caminho relativo com "./" na frente
            let caminho_pronto = PathBuf::from("./").join(caminho_relativo);
           // eprintln!("gera_caminho_relativo => caminho_pronto: {}", caminho_pronto.display());
            Some(caminho_pronto)
        }
        Err(_) => None, // Retorna None se o caminhoPai não for prefixo de caminho1
    }
}
pub fn ParseSize(size_str: &str) ->Result<u64, Box<dyn Error>> {
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
        _ => Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("Unknown unit: {}", unit_part),
        ))),
    }
}

pub fn hash_hush(
    file_path: &str,
    alg_str: &str, // Algoritmo com ou sem bloco!
    buffer_size: usize,    
    n_max_concur: u32,
    flag_show_progress: bool,
) -> Result<String, Box<dyn std::error::Error>> {
    info!(
        "hash_hush: alg:{} file: {} BufferSize:{} ",
        alg_str, file_path, buffer_size
    );

    // Se tem - no algoritmo, é pq é hush
    if let Some((algorithmRash, blocksize_str)) = alg_str.split_once('-') {        
        match ParseSize(blocksize_str) {
            Ok(blocksize_recovered) => {
                return hash_hsha256(
                    file_path,
                    buffer_size,
                    blocksize_recovered as usize,
                    n_max_concur,
                    flag_show_progress
                ); 
            }
            Err(e) => return Err(e),
        }
    } else{
        return hash_sha256(file_path, buffer_size);
    }

}

pub fn hash_hsha256(
    file_path: &str,
    buffer_size: usize,
    chunk_size: usize,
    n_max_concur: u32,
    flag_show_progress:bool
) -> Result<String, Box<dyn std::error::Error>> {
    let hash_alg = "hsha256".to_string();

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
        if flag_show_progress {
            eprint!(">");
        }
    }
    drop(sender_calculo); // Dropar após envio

    let num_cpus = num_cpus::get();

    // Criar threads para calcular o hash dos blocos usando BufReader
    let mut handles = Vec::new();

    for n_cpu in 0..n_max_concur {
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
                            "Error hush Block: {}: {:?} File:{} ",
                            n_bloco, e, file_path_clone
                        );
                        process::exit(1);
                    }
                }
                if flag_show_progress {
                    eprint!("<");
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
        info!("File;Block;bytes_start;byte_end;hash");
        for bloco in resultados {
            info!(
                "{0}{1:10};{2:10};{3:10};{4}",
                file_path,
                bloco.n_bloco, bloco.inicio_bloco, bloco.fim_bloco, bloco.hash_bloco
            );
            hasher_cumulativo.update(bloco.hash_bloco.as_bytes());
        }
        let hash_cumulativo_result = hasher_cumulativo.finalize();
        hash_final = format!("{:x}", hash_cumulativo_result);
        info!("File:{} => Hush:{}",file_path,hash_final);
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
        info!("Dir created: {}", dir_destino);
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


pub fn check_hash(
    main_args: Argumentos,
    file_path: &str,
    work_dir: &str,
) -> io::Result<()> {

    info!(
        "Running subcommand:{} , with: \n in_file_path: {}, work_dir: {}, workers: {}, max_concur: {}, block_size: {}, buffer_size: {}, recursive_enabled: {}, filter:{:?}, log_enabled:{}, out_file_path:{}, stop_on_first_err:{}",
        main_args.sub_comando,        
        main_args.in_file_path.display(),
        work_dir,        
        main_args.n_workers,
        main_args.n_max_concur,
        main_args.block_size_str,
        main_args.buffer_size_str,
        main_args.recursive_enabled,
        main_args.in_file_filter,
        main_args.log_enabled,
        main_args.out_file_path.display(),
        main_args.flag_stop_on_first_error
        
    );
    let mut n_errors=0;
    let mut n_acertos=0;
    let mut n_linhas=0;

    let path = Path::new(file_path);
    let file = File::open(&path)?;
    let reader = BufReader::new(file);

    let mut error_results = Vec::new();

    // Read the file line by line
    for line_result in reader.lines() {
        match line_result {
            Ok(line) => {
                // Parse each line
                n_linhas+=1;
                let split_result = line.split_once('?');
                if split_result.is_none() { // Erro lendo ?
                    n_errors+=1;
                    let error_msg = format!(
                        "Error {}! Line:{} File:{} mot contain '?' char.",
                         n_errors, n_linhas, file_path
                    );
                    error_results.push(error_msg.clone());
                    error!("{}",error_msg);
                    if main_args.flag_stop_on_first_error {
                        eprintln!("{}",error_msg);
                        process::exit(1);                            
                    }                            
                    continue;
                }
                let (hash_lido_tmp, rest) = split_result.unwrap();
                
                    // Split the remaining part by `*` to separate algorithm and path
                info!("Hash Lido raw:*{}*", hash_lido_tmp);
                
                let mut hash_lido = hash_lido_tmp.to_string();
                let mut hash_lido2 = hash_lido.trim().to_string();
                
                info!("Hash Lido Trimmed:*{}*", hash_lido2.trim());
                
                let split_result2 = line.split_once('*');
                if split_result2.is_none() { // Erro lendo ?
                    n_errors+=1;
                    let error_msg = format!(
                        "Error {}! Line:{} File:{} mot contain '*' char.",
                         n_errors, n_linhas, file_path
                    );
                    error_results.push(error_msg.clone());
                    error!("{}",error_msg);
                    if main_args.flag_stop_on_first_error {
                        eprintln!("{}",error_msg);
                        process::exit(1);                            
                    }                            
                    continue;
                }

                 let (algorithm, file_path_relativo) = split_result2.unwrap();
                 /* 
                info!("Algorithm:*{}*", algorithm);
                info!("File Relative Path:*{}*", file_path_relativo); */
                let file_path_completo =
                    gera_caminho_completo(file_path_relativo, work_dir);
                let file_path_completo_dados = file_path_completo.to_str().unwrap();
             /*    info!("File Path Completo:*{}*", file_path_completo_dados); */
                /* Testa se arquivo existe! */
                if ! Path::new(file_path_completo_dados).is_file() {                                                        
                    n_errors+=1;
                    let error_msg = format!(
                        "Error {}! Line:{} File:{} does not exist!",
                            n_errors, n_linhas, file_path_completo_dados
                    );
                    error_results.push(error_msg.clone());
                    error!("{}",error_msg);
                    if main_args.flag_stop_on_first_error {
                        eprintln!("{}",error_msg);
                        process::exit(1);                            
                    }                            
                    continue;
                }
                match hash_hush(
                    file_path_completo_dados,
                    algorithm,
                    main_args.buffer_size as usize,
                    main_args.n_max_concur,
                    main_args.flag_show_progress,

                ) {
                    Ok(hash_calculado) => {
                        /* info!("Hash Calculated:*{}*", hash_calculado); */
                        let hash_calc=hash_calculado.to_lowercase().trim().to_string();
                        let hash_to_check = hash_lido.to_lowercase().trim().to_string();
                        if hash_to_check== hash_calc {
                            info!("Hashes matched!");
                            n_acertos+=1;
                        } else {
                            // Erro na checagem
                            n_errors+=1;
                            let error_msg = format!(
                                "Error {}! Line:{} File:{} Algorithm:{} HashRead:{} HashCalculated:{}: Hash doesnt match!",
                                 n_errors, n_linhas,file_path_completo_dados, algorithm,hash_to_check,hash_calc                                                      
                            );
                            error_results.push(error_msg.clone());
                            error!("{}",error_msg);
                            if main_args.flag_stop_on_first_error {
                                eprintln!("{}",error_msg);
                                process::exit(1);                            
                            }      
                            continue;
                            
                        }
                    }
                    Err(e) => {                        
                        n_errors+=1;
                        let error_msg = format!(
                            "Error {}: Line:{} File:{} Algorithm:{} : error{}:",
                                n_errors, n_linhas,file_path_completo_dados, algorithm,e );
                                error_results.push(error_msg.clone());
                        if main_args.flag_stop_on_first_error {
                            eprintln!("{}",error_msg);
                            process::exit(1);                            
                        }   
                        continue;

                    }
                }

            }
            Err(e) => {
                // Handle the error if reading the line fails
                n_linhas+=1;        
                n_errors+=1;        
                let error_msg = format!("Error {}! Line:{} File:{} => {}",
                     n_errors, n_linhas,file_path, e );
                if main_args.flag_stop_on_first_error {
                    eprintln!("{}",error_msg);
                    process::exit(1);                            
                }   
                continue;
            }
        }
    }
    
    
    if n_linhas == n_acertos {
        let sucess_msg = format!("Success! Total lines:{} matches! No errors.",n_linhas);
        info!("{}",sucess_msg);          
        println!("{}",sucess_msg);          
    } else{
        let sucess_msg = format!("Results: Total lines:{}, matches:{}, errors:{}. ",n_linhas, n_acertos, n_errors);          
        info!("{}",sucess_msg);          
        println!("{}",sucess_msg);                  
        if n_errors > 0 {
            error!("{} errors found", n_errors);                
        }        
        process::exit(1);        
    }        
    
    Ok(())
}

/* 
pub fn check_hash_orignial(
    main_args: Argumentos,
    file_path: &str,
    work_dir: &str,
) -> io::Result<()> {

    info!(
        "Running subcommand:{} , with: \n in_file_path: {}, work_dir: {}, workers: {}, max_concur: {}, block_size: {}, buffer_size: {}, recursive_enabled: {}, filter:{:?}, log_enabled:{}, out_file_path:{}, stop_on_first_err:{}",
        main_args.sub_comando,        
        main_args.in_file_path,
        work_dir,        
        main_args.n_workers,
        main_args.n_max_concur,
        main_args.block_size_str,
        main_args.buffer_size_str,
        main_args.recursive_enabled,
        main_args.in_file_filter,
        main_args.log_enabled,
        main_args.out_file_path,
        main_args.flag_stop_on_first_error
        
    );
    let mut n_errors=0;
    let mut n_acertos=0;
    let mut n_linhas=0;

    let path = Path::new(file_path);
    let file = File::open(&path)?;
    let reader = BufReader::new(file);

    let mut error_results = Vec::new();

    // Read the file line by line
    for line_result in reader.lines() {
        match line_result {
            Ok(line) => {
                // Parse each line
                n_linhas+=1;
                if let Some((hash_lido_tmp, rest)) = line.split_once('?') {
                    // Split the remaining part by `*` to separate algorithm and path
                    info!("Hash Lido raw:*{}*", hash_lido_tmp);
                    
                    let mut hash_lido = hash_lido_tmp.to_string();
                    let mut hash_lido2 = hash_lido.trim().to_string();
                    
                    info!("Hash Lido Trimmed:*{}*", hash_lido2.trim());
                    if let Some((algorithm, file_path_relativo)) = rest.split_once('*') {                        
                        info!("Algorithm:*{}*", algorithm);
                        info!("File Relative Path:*{}*", file_path_relativo);
                        let file_path_completo =
                            gera_caminho_completo(file_path_relativo, work_dir);
                        let file_path_completo_dados = file_path_completo.to_str().unwrap();
                        info!("File Path Completo:*{}*", file_path_completo_dados);
                        /* Testa se arquivo existe! */
                        if ! Path::new(file_path_completo_dados).is_file() {                                                        
                            n_errors+=1;
                            let error_msg = format!(
                                "Error {}! Line:{} File:{} does not exist!",
                                 n_errors, n_linhas, file_path_completo_dados
                            );
                            error_results.push(error_msg);
                            error!("{}",error_msg);
                            if main_args.flag_stop_on_first_error {
                                eprintln!("{}",error_msg);
                                process::exit(1);                            
                            }                            
                            continue;
                        }
                    

                        if let Some((algorithmRash, blocksize_str)) = algorithm.split_once('-') {
                            info!("AlgorithmRash:*{}*", algorithmRash);
                            info!("blocksize:*{}*", blocksize_str);
                            match ParseSize(blocksize_str) {
                                Ok(blocksize_recovered) => {
                                    match hash_hsha256(
                                        file_path_completo_dados,
                                        main_args.buffer_size as usize,
                                        blocksize_recovered as usize,
                                        main_args.n_max_concur,
                                    ) {
                                        Ok(hash_calculado) => {
                                            info!("Hash Calculated:*{}*", hash_calculado);
                                            let hash_calc=hash_calculado.to_lowercase().trim().to_string();
                                            let hash_to_check = hash_lido.to_lowercase().trim().to_string();
                                            if hash_to_check== hash_calc {
                                                info!("Hashes matched!");
                                                n_acertos+=1;
                                            } else {
                                                // Erro na checagem
                                                n_errors+=1;
                                                let error_msg = format!(
                                                    "Error {}! Line:{} File:{} Algorithm:{} HashRead:{} HashCalculated:{}: Hash doesnt match!",
                                                     n_errors, n_linhas,file_path_completo_dados, algorithm,hash_to_check,hash_calc                                                      
                                                );
                                                error_results.push(error_msg);
                                                error!("{}",error_msg);
                                                if main_args.flag_stop_on_first_error {
                                                    eprintln!("{}",error_msg);
                                                    process::exit(1);                            
                                                }      
                                                
                                            }
                                        }
                                        Err(e) => {
                                            eprintln!("hsha256 error: {}. Line:{}", e,n_linhas);
                                              let error_msg = format!(
                                                    "Error {}! Line:{} File:{} Algorithm:{} HashRead:{} HashCalculated:{}: Hash doesnt match!",
                                                     n_errors, n_linhas,file_path_completo_dados, algorithm,hash_to_check,hash_calc                                                      
                                                );
                                            if main_args.flag_stop_on_first_error {
                                                eprintln!("{}",error_msg);
                                                process::exit(1);                            
                                            }   
                                        }
                                    }
                                }
                                Err(e) => {
                                    eprintln!("hsha256 error: {}. Line:{}", e,n_linhas);
                                    if main_args.flag_stop_on_first_error {
                                        eprintln!("{}",error_msg);
                                        process::exit(1);                            
                                    }   
                                }
                            }
                        } else {
                            match hash_sha256(&file_path_completo_dados, main_args.buffer_size as usize) {
                                Ok(hash_calculado) => {
                                    info!("Hash Calculated:*{}*", hash_calculado);
                                    let hash_calc=hash_calculado.to_lowercase().trim().to_string();
                                    let hash_to_check = hash_lido.to_lowercase().trim().to_string();
                                    if hash_to_check== hash_calc {
                                        info!("Hashes matched!");
                                        n_acertos+=1;
                                    } else {
                                        // Erro na checagem
                                        n_errors+=1;
                                        let error_msg = format!(
                                            "Error {}! Line:{} File:{} Algorithm:{} HashRead:{} HashCalculated:{}: Hash doesnt match!",
                                             n_errors, n_linhas,file_path_completo_dados, algorithm,hash_to_check,hash_calc                                                      
                                        );
                                        error_results.push(error_msg);
                                        error!("{}",error_msg);
                                    }
                                }
                                Err(e) => {
                                    eprintln!("sha256 error: {}. Line:{}", e,n_linhas);
                                    if main_args.flag_stop_on_first_error {
                                        eprintln!("{}",error_msg);
                                        process::exit(1);                            
                                    }   
                                }
                            }
                        }
                    } else {
                        eprintln!("Invalid format: missing '*' in {}. Line:{}", line,n_linhas);
                        if main_args.flag_stop_on_first_error {
                            eprintln!("{}",error_msg);
                            process::exit(1);                            
                        }   
                    }
                } else {
                    eprintln!("Invalid format: missing '?' in {}. Line:{}", line,n_linhas);
                    if main_args.flag_stop_on_first_error {
                        eprintln!("{}",error_msg);
                        process::exit(1);                            
                    }   
                }
            }
            Err(err) => {
                // Handle the error if reading the line fails
                eprintln!("Error reading line: {}. Linha{}", err,n_linhas);
                if main_args.flag_stop_on_first_error {
                    eprintln!("{}",error_msg);
                    process::exit(1);                            
                }   
            }
        }
    }
    
    
    if n_linhas == n_acertos {
        println!("Success! Total lines:{} matches! No errors.",n_linhas);          
    } else{
        println!("Results: Total lines:{}, matches:{}, errors:{}. ",n_linhas, n_acertos, n_errors);          
        if n_errors > 0 {
            eprintln!("{} errors found", n_errors);            
        }
        process::exit(1);        
    }
    
    
    
    Ok(())
}
*/