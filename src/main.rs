#![allow(non_snake_case)]
use clap::{Arg, Command};
use crossbeam_channel::{unbounded, Receiver, Sender};
use functions::{hash_rsha256, hash_sha256,gera_caminho_relativo};
use glob::glob;
use log::{info, warn,debug, LevelFilter};

use once_cell::sync::OnceCell;

use std::env;
use std::fs::{self, DirEntry, File};
use std::io::{self, BufRead, BufReader, Read, Seek, Write};
use std::path::Path;
use std::process;
use std::result::Result;
use std::thread;
use std::time::Instant;
use std::time::SystemTime;
use crate::functions::{ParseSize};


mod functions;

pub static LOG_ENABLED: OnceCell<bool> = OnceCell::new();


fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Coletando os argumentos da linha de comando
    let start = Instant::now();
    let args: Vec<String> = env::args().collect();

    let mut main_args = functions::Argumentos {
        n_workers: 0,
        n_max_concur: 0,
        block_size_str: String::new(),
        buffer_size_str: String::new(),
        block_size: 0,
        buffer_size: 0,
        log_enabled: false,
        sub_comando: String::new(),
        in_file_path: String::new(),
        in_file_filter: String::new(),
        recursive_enabled: false,
        out_file_path: String::new(),
    };

    // Pega

    main_args.block_size_str= "50MB".to_string(); // 50 MB
    main_args.block_size = ParseSize(&main_args.block_size_str)? as u64; // 50 MB

    main_args.buffer_size_str = "10KB".to_string(); // 50 MB
    main_args.buffer_size = ParseSize(&main_args.buffer_size_str)? as u32; // 50 MB

    main_args.n_workers= 15;
    main_args.n_max_concur= 15;

    if args.len() < 3 {
        eprintln!("Use: {} <command> <file_path> <dest_folder_path>?", args[0]);
        eprintln!("Commands: 'gen', 'check', 'rsha256','split','rebuild','sha256' ");
        eprintln!("Option: '--log' to print logs");
        eprintln!(
            "Option: '--blocksize Value' to change size that file block is divided. Default {}. Use KB, MB, GB, TB",
            main_args.block_size_str
        );
        eprintln!(
            "Option: '--buffersize Value' to change buffersize. Default {}. Use KB, MB, GB, TB",
            main_args.buffer_size_str
        );
        eprintln!(
            "Option: '--n_workers Value' to change how many workers will be used. Default {}. ",         main_args.n_workers
        );
        eprintln!(
            "Option: '--n_max_concur Value' to change how maximum number of concurrent access to each file . Default {}",  main_args.n_max_concur
        );

        process::exit(1);
    }

    if let Some(chunksize_index) = args.iter().position(|x| x == "--blocksize") {
        if let Some(size_str) = args.get(chunksize_index + 1) {
            main_args.block_size_str= size_str.to_string();
            main_args.block_size = ParseSize(size_str)? as u64;
        } else {
            eprintln!("--blocksize provided without a value. Use KB, MB, GB, TB. Ex: --blocksize 50MB");
            return Ok(());
        }
    }


    if let Some(buffersize_index) = args.iter().position(|x| x == "--buffersize") {
        if let Some(size_str) = args.get(buffersize_index + 1) {
            main_args.buffer_size_str = size_str.to_string();
            main_args.buffer_size = ParseSize(size_str)? as u32;
        } else {
            eprintln!("--buffersize provided without a value. Use KB, MB. Ex: --buffersize 8KB ");
            return Ok(());
        }
    }
    
    
    if let Some(chunksize_index) = args.iter().position(|x| x == "--n_workers") {
        if let Some(size_str) = args.get(chunksize_index + 1) { 
            match size_str.parse::<u32>() {
                Ok(parsed_value) => {
                    main_args.n_workers = parsed_value;
                }
                Err(e) => {
                    eprintln!("Failed to parse size_str as u32: {}", e);
                }    
            }            
        } else {
            eprintln!("--n_workers provided without a value. Don´t use it, or use a number.Ex: --n_workers 30");
            return Ok(());
        }
    }

    
    if let Some(chunksize_index) = args.iter().position(|x| x == "--n_max_concur") {
        if let Some(size_str) = args.get(chunksize_index + 1) {        
            match size_str.parse::<u32>() {
                Ok(parsed_value) => {
                    main_args.n_max_concur = parsed_value;
                }
                Err(e) => {
                    eprintln!("Failed to parse size_str as u32: {}", e);
                }    
            }
                    
        } else {
           
            eprintln!("--n_max_concur provided without a value. Don´t use it, or use a number. Ex: --n_max_concur 20");
            return Ok(());
        }
    }

    let enable_logging = args.contains(&"--log".to_string());
    if enable_logging {
        main_args.log_enabled=true;
        LOG_ENABLED
            .set(true)
            .expect("LOG_ENABLED can only be set once");
        env_logger::Builder::new()
            .filter_level(LevelFilter::Debug) // Set logging level (Info, Warn, etc.)
            .init();
        info!("Logging is enabled in the main thread.");
    }
   
    // Extraindo os argumentos
    let comando = &args[1];
    let file_path = &args[2];
    
    main_args.in_file_path=file_path.to_string();
  

    // Executando a função correspondente com base no comando fornecido
    main_args.sub_comando=comando.to_string();
    info!("Comando {}",comando);
    match comando.as_str() {
        "split" => {
            
            if args.len() != 4 {
                eprintln!("Use: {} split <file_path> <dest_folder_path>?", args[0]);
                eprintln!("<file_path>: file to split.");
                eprintln!("<file_path>: folder to receive splitted files.");
                process::exit(1);
            }

            let dir_destino = &args[3];

            if let Err(e) = functions::split(file_path, dir_destino, main_args.buffer_size as usize,  main_args.block_size as usize) {
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

            if let Err(e) = functions::rebuild(dir_destino, file_path, main_args.buffer_size as usize, main_args.block_size as usize) {
                eprintln!("Rebuild error: {}", e);
                process::exit(1);
            }
        }
        "rsha256" => {
            
            match functions::hash_rsha256(
                file_path,
                main_args.buffer_size as usize,
                main_args.block_size as usize,         
                main_args.n_max_concur
                         
            ){
                Ok(hash_final) => {
                            let hash_alg= "rsha256";
                            
                            let safe_hash_final = String::from_utf8_lossy(hash_final.as_bytes());
                            let safe_hash_alg = String::from_utf8_lossy(hash_alg.as_bytes());
                            let safe_chunk_size_str = String::from_utf8_lossy(main_args.block_size_str.as_bytes());
                            let safe_file_path = String::from_utf8_lossy(file_path.as_bytes());
                            println!(
                                "{} ?{}|{}*{}",
                                safe_hash_final, safe_hash_alg, safe_chunk_size_str, safe_file_path
                            );
                           
                }
                Err(e) =>
                {
                    eprintln!("rsha256 error: {}", e);
                    process::exit(1);
                }
                 

            }
            
           
            
        }
        "gen" => {

            
            if args.len() < 3 {
                eprintln!("Use: {} gen <file_path> ", args[0]);
                eprintln!("<file_path>: file_path to gen(glob pattern!).");                

                process::exit(1);
            }            
            
                                                
            debug!("search_files: {} ",file_path)    ;
            let results =         
            search_files(file_path ).unwrap();

            process_files(main_args, results);


        }

        "check" => {
            if args.len() < 3 {
                eprintln!("Use: {} check <file_path> <dir_work>?", args[0]);
                eprintln!("<file_path>: file_path to check.");
                eprintln!("<dir_work>: set work dir where files are.");

                process::exit(1);
            }
            let mut work_dir = ".";
            if args.len() > 3 {
                 work_dir = &args[3];
            }

            if let Err(e) = functions::read_and_parse_file(main_args,file_path,work_dir) {
                eprintln!("check error: {}", e);
                process::exit(1);
            }
        }

        "sha256" => {
            if let Err(e) = functions::hash_sha256(file_path, main_args.buffer_size as usize) {
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

pub struct TFileHash {
    n_arquivo: u64,
    path:String,
    alg_hash:String,
    valor_hash:String
    
}

fn process_files(main_args: functions::Argumentos,files :Vec<String>){

    let n_files_a_processar=files.len();
    info!(
        "Running subcommand:{} over {} files, with: \n in_file_path: {}, workers: {}, max_concur: {}, block_size: {}, buffer_size: {}, recursive_enabled: {}, filter:{:?}, log_enabled:{}, out_file_path:{}",
        main_args.sub_comando,
        files.len(),
        main_args.in_file_path,
        main_args.n_workers,
        main_args.n_max_concur,
        main_args.block_size_str,
        main_args.buffer_size_str,
        main_args.recursive_enabled,
        main_args.in_file_filter,
        main_args.log_enabled,
        main_args.out_file_path
        
    );

    let (sender_files, receiver_files): (Sender<String>, Receiver<String>) =
        unbounded();
    
    
    let (sender_files_calculados, receiver_files_calculados): (Sender<TFileHash>, Receiver<TFileHash>) =
        unbounded();

    //Enviar todos arquivos para o canal que sera processado
        for file in files {
            //  println!("Enviado Bloco {} -> p/ calculo", bloco.n_bloco);
            info!("=>{}",file);            
            sender_files.send(file).unwrap();
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
                let arquivo_chegada=file_input.clone();
                info!("<= {}",arquivo_chegada);            
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
                              
                if file_size>main_args.block_size{                    
                    // ===> RSHA 

                    let mut algor_hash_tmp_rsha256_type=String::from("rsha256");                                                                                
                    let mut algor_hash_tmp_rsha256= format!("{}-{}",algor_hash_tmp_rsha256_type, block_size_str_clone);
                    
                    info!("{} -> {}",arquivo_chegada.clone(),algor_hash_tmp_rsha256.clone());                    

                    match functions::hash_rsha256(
                        &arquivo_chegada.clone(),
                        main_args.buffer_size as usize,
                        main_args.block_size as usize,                        
                        main_args.n_max_concur
                    ){                      
                        Ok(hash_calculado) => {
                            
                            let fileCalculado= TFileHash{
                                n_arquivo: 0,
                                path:arquivo_chegada.clone(),
                                alg_hash:algor_hash_tmp_rsha256.clone(),
                                valor_hash: hash_calculado.clone()
                            };
                            sender_files_calculados_clone.send(fileCalculado).unwrap();
                            info!("{} {}?{} ",arquivo_chegada.clone(),algor_hash_tmp_rsha256.clone(),hash_calculado.clone())
                        }
                        Err(e) =>{
                            eprintln!("Error rsha256: {} with: {}", e,arquivo_chegada); 
                        } 
                    }
                       

                } else{
                    // ===> SHA256 normal
                    
                    let algor_hash_tmp_sha256=String::from("sha256");
                    info!("{} -> {}",arquivo_chegada.clone(),algor_hash_tmp_sha256.clone());                                       
                    
                    match functions::hash_sha256(&arquivo_chegada.clone(), main_args.buffer_size as usize){
                        Ok(hash_calculado) => {                            
                            
                            let fileCalculado= TFileHash{
                                n_arquivo: 0,
                                path:arquivo_chegada.clone(),
                                alg_hash:algor_hash_tmp_sha256.clone(),
                                valor_hash: hash_calculado.clone()
                            };
                            sender_files_calculados_clone.send(fileCalculado).unwrap();
                            
                         //   println!("{0} ?{1}*{2}",filePronto.valor_hash,filePronto.alg_hash,filePronto.path);

                            info!("{} ?{}*{} ",arquivo_chegada.clone(),algor_hash_tmp_sha256.clone(),hash_calculado.clone())
                        }
                        Err(e) =>{
                            eprintln!("Error sha256: {} with: {}", e,arquivo_chegada); 
                        } 
                     }
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
        match gera_caminho_relativo(&filePronto.path.clone(),&main_args.in_file_path.clone()) {
            Some(caminho_relativo) =>{
                info!("Caminho relativo: {}", caminho_relativo.display());
                println!("{0} ?{1}*{2}",filePronto.valor_hash,filePronto.alg_hash,caminho_relativo.display());
            } 
            None => { 
                eprintln!("Erro: {} não é um prefixo {}",filePronto.path,main_args.in_file_path);
            }
            
        }

        
        resultados.push(filePronto);
    }

    
    for handle in handles {
        handle.join().unwrap();
    }
    /* */
    //resultados.sort_by_key(|filePronto| filePronto.path);
    let n_files_prontos=resultados.len();
    /* 
    for filePronto in resultados {
        println!("{0} {1}?{2}",filePronto.valor_hash,filePronto.alg_hash,filePronto.path);            
    }
    */
    info!("Total files a processar:{}",n_files_a_processar);
    info!("Total files processados:{}",n_files_prontos);

}



fn search_files(pattern: &str) -> Result<Vec<String>, io::Error> {
    let mut results = Vec::new();

    // Verifica se o caminho é um arquivo
    if Path::new(pattern).is_file() {
        results.push(pattern.to_string().replace("\\", "/"));
        return Ok(results) // Retorna já que é um arquivo, não precisa continuar
    }

    let mut glob_pattern= pattern.to_string();
    if Path::new(pattern).is_dir() {
        glob_pattern =  format!("{}/**/*", pattern); 
    }

    info!("Glob to use: {}", glob_pattern);
    for entry in glob(&glob_pattern).expect("Failed to read glob pattern") {
        match entry {
            Ok(path) => {
                if path.is_file() {
                    if let Some(path_str) = path.to_str() {
                        results.push(path_str.to_string().replace("\\", "/"));
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