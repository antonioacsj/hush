#![allow(non_snake_case)]
use clap::{Arg, Command};
use crossbeam_channel::{unbounded, Receiver, Sender};
use functions::{gera_caminho_relativo, hash_hsha256, hash_sha256};
use glob::glob;
use log::{debug, info, warn, LevelFilter};

use once_cell::sync::OnceCell;

use crate::functions::{process_files, search_files, Argumentos, ParseSize};
use std::env;
use std::fs::{self, DirEntry, File};
use std::io::{self, BufRead, BufReader, Read, Seek, Write};
use std::path::Path;
use std::process;
use std::result::Result;
use std::thread;
use std::time::Instant;
use std::time::SystemTime;

mod functions;

pub static LOG_ENABLED: OnceCell<bool> = OnceCell::new();

fn print_usage(main_args: functions::Argumentos) {
    eprintln!("{} - hash tool for rush situations", main_args.name);
    eprintln!("\nusage: {} <command> <path> <options>?", main_args.name);
    eprintln!("\nCommands:\n\t 'gen': generate hashes from path. If a folder, its recursive. Glob match pattern can be used.\n\t 'check': check hashes from a file using a work_dir as base\n");

    eprintln!(
        "Simple use:\n\t'{} gen <input_path> <options>'",
        main_args.name
    );
    eprintln!(
        "\t'{} check <file_hashes_path> <work_dir_base> <options>'",
        main_args.name
    );
    eprintln!("\nOptions");

    eprintln!("\t'--log' to print lots of boring stuff");
    eprintln!("\t'--progress' to show that something is being done while you drink coffee.");
    eprintln!(
        "\t'--stop' Stop everything if some error. By default, don´t stop. (Make it in your way!) "
    );
    eprintln!(
        "\t'--blocksize Value' to change size that file block is divided. Default {}. Use KB, MB, GB, TB, where B is Byte, ok? :)",
        main_args.block_size_str
    );
    eprintln!(
        "\t'--buffersize Value' to change buffersize to read buffers.. Default {}. Use KB, MB, GB, TB. Hands off if you don't know what it is. ",
        main_args.buffer_size_str
    );
    eprintln!(
        "\t'--n_workers Value' to change how many workers will be used in main pool. Default {}. ",
        main_args.n_workers
    );
    eprintln!("\t'--n_max_concur Value' to change how maximum number of concurrent access to each file, in pool of slaves. Default {}",  main_args.n_max_concur
    );
    eprintln!(
        "\t'--hash_alg Value' to change hash function to use. By default and supported: sha256 "
    );
    eprintln!(
        "\n**IMPORTANT**. In check, the blocksize information necessary to check data. !! SAVE ALL DATA GENERATED !!"
    );
    eprintln!(
        "\nEx: \n\t'{} gen C:/Folder1/Data > C:/Folder1/hash_file.txt'",
        main_args.name
    );
    eprintln!(
        "\t'{} check C:/Folder1/hash_file.txt C:/Folder1/Data'",
        main_args.name
    );
    eprintln!(
        "\nEx with options:\n\t'{} gen C:/Folder1/Data > C:/Folder1/hash_file.txt --progress --stop --log --blocksize 100MB --n_workers 40 --n_max_concur 20'",
        main_args.name
    );
    eprintln!(
        "\t'{} check C:/Folder1/hash_file.txt C:/Folder1/Data --progress --stop --log --n_workers 40 --n_max_concur 20'",
        main_args.name
    );
    eprintln!(
        "\n**IMPORTANT**. In check, the blocksize information necessary to check data. Save all data generated!"
    );

    eprintln!("\n\n\tMore details in: https://github.com/antonioacsj/hush");
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Coletando os argumentos da linha de comando
    let start = Instant::now();
    let args: Vec<String> = env::args().collect();

    let mut main_args = functions::Argumentos {
        name: "hush".to_string(),
        n_workers: 0,
        n_max_concur: 0,
        block_size_str: String::new(),
        buffer_size_str: String::new(),
        block_size: 0,
        buffer_size: 0,
        flag_show_progress: false,
        flag_stop_on_first_error: false,
        log_enabled: false,
        sub_comando: String::new(),
        in_file_path: String::new(),
        in_file_filter: String::new(),
        recursive_enabled: false,
        out_file_path: String::new(),
    };

    // Pega

    main_args.block_size_str = "50MB".to_string(); // 50 MB
    main_args.block_size = ParseSize(&main_args.block_size_str)? as u64; // 50 MB

    main_args.buffer_size_str = "10KB".to_string(); // 50 MB
    main_args.buffer_size = ParseSize(&main_args.buffer_size_str)? as u32; // 50 MB

    main_args.n_workers = 15;
    main_args.n_max_concur = 15;

    if args.len() < 3 {
        print_usage(main_args);
        process::exit(0);
    }

    if let Some(chunksize_index) = args.iter().position(|x| x == "--blocksize") {
        if let Some(size_str) = args.get(chunksize_index + 1) {
            main_args.block_size_str = size_str.to_string();
            main_args.block_size = ParseSize(size_str)? as u64;
        } else {
            eprintln!(
                "--blocksize provided without a value. Use KB, MB, GB, TB. Ex: --blocksize 50MB"
            );
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

    main_args.flag_stop_on_first_error = args.contains(&"--stop".to_string());

    main_args.flag_show_progress = args.contains(&"--progress".to_string());

    let enable_logging = args.contains(&"--log".to_string());
    if enable_logging {
        main_args.log_enabled = true;
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

    main_args.in_file_path = file_path.to_string();

    // Executando a função correspondente com base no comando fornecido
    main_args.sub_comando = comando.to_string();
    info!("Comando {}", comando);
    match comando.as_str() {
        "split" => {
            if args.len() != 4 {
                eprintln!("Use: {} split <file_path> <dest_folder_path>?", args[0]);
                eprintln!("<file_path>: file to split.");
                eprintln!("<file_path>: folder to receive splitted files.");
                process::exit(1);
            }

            let dir_destino = &args[3];

            if let Err(e) = functions::split(
                file_path,
                dir_destino,
                main_args.buffer_size as usize,
                main_args.block_size as usize,
            ) {
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

            if let Err(e) = functions::rebuild(
                dir_destino,
                file_path,
                main_args.buffer_size as usize,
                main_args.block_size as usize,
            ) {
                eprintln!("Rebuild error: {}", e);
                process::exit(1);
            }
        }
        "hsha256" => {
            match functions::hash_hsha256(
                file_path,
                main_args.buffer_size as usize,
                main_args.block_size as usize,
                main_args.n_max_concur,
                main_args.flag_show_progress,
            ) {
                Ok(hash_final) => {
                    let hash_alg = "hsha256";

                    let safe_hash_final = String::from_utf8_lossy(hash_final.as_bytes());
                    let safe_hash_alg = String::from_utf8_lossy(hash_alg.as_bytes());
                    let safe_chunk_size_str =
                        String::from_utf8_lossy(main_args.block_size_str.as_bytes());
                    let safe_file_path = String::from_utf8_lossy(file_path.as_bytes());
                    println!(
                        "{} ?{}|{}*{}",
                        safe_hash_final, safe_hash_alg, safe_chunk_size_str, safe_file_path
                    );
                }
                Err(e) => {
                    eprintln!("hsha256 error: {}", e);
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
            debug!("search_files: {} ", file_path);
            let results = search_files(file_path).unwrap();
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

            if let Err(e) = functions::check_hash(main_args, file_path, work_dir) {
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
            eprintln!("Commands: 'hsha256','split','rebuild','sha256' ");
            process::exit(1);
        }
    }
    let duration = start.elapsed();
    info!("Execution Time: {:?}", duration);
    Ok(())
}
