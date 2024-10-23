fn main() {
    let mut main_args = Argumentos {
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
    let matches = Command::new("cli_tool")
        .version("1.0")
        .author("Your Name")
        .about("CLI tool for file operations")
        .arg(
            Arg::new("workers")
                .long("workers")
                .help("Number of workers")
                .num_args(1) // Replaces .takes_value(true)
                .value_parser(clap::value_parser!(u32)),
        )
        .arg(
            Arg::new("numcpus")
                .long("numcpus")
                .help("Number of CPUs")
                .num_args(1) // Replaces .takes_value(true)
                .value_parser(clap::value_parser!(u32)),
        )
        .arg(
            Arg::new("block_size")
                .long("block_size")
                .help("Size of blocks to process")
                .num_args(1), // Replaces .takes_value(true)
        )
        .arg(
            Arg::new("buffer_size")
                .long("buffer_size")
                .help("Size of the buffer")
                .num_args(1), // Replaces .takes_value(true)
        )
        .arg(
            Arg::new("log")
                .long("log")
                .help("Enable or disable logging")
                .action(clap::ArgAction::SetTrue), // Adds a boolean flag
        )
        .subcommand(
            Command::new("gen")
                .about("Generates something")
                .arg(
                    Arg::new("file_path")
                        .help("Path to the input file")
                        .required(true)
                        .num_args(1),
                )
                .arg(
                    Arg::new("dir")
                        .help("Directory path")
                        .required(true)
                        .num_args(1),
                ),
        )
        .subcommand(
            Command::new("split")
                .about("Splits a file")
                .arg(
                    Arg::new("file_path")
                        .help("Path to the input file")
                        .required(true)
                        .num_args(1),
                )
                .arg(
                    Arg::new("dir")
                        .help("Directory path")
                        .required(true)
                        .num_args(1),
                ),
        )
        .subcommand(
            Command::new("rebuild")
                .about("Rebuilds a file")
                .arg(
                    Arg::new("file_path")
                        .help("Path to the input file")
                        .required(true)
                        .num_args(1),
                )
                .arg(
                    Arg::new("dir")
                        .help("Directory path")
                        .required(true)
                        .num_args(1),
                ),
        )
        .get_matches();

    // Retrieve global options
    let workers = matches.get_one::<u32>("workers").unwrap_or(&4);
    let numcpus = matches.get_one::<u32>("numcpus").unwrap_or(&2);
    let binding_a = "64k".to_string();
    let block_size = matches
        .get_one::<String>("block_size")
        .unwrap_or(&binding_a);
    let binding_b = "128k".to_string();
    let buffer_size = matches
        .get_one::<String>("buffer_size")
        .unwrap_or(&binding_b);

    let logging_enabled = matches.get_flag("log"); // Retrieve log flag status

    if logging_enabled {
        println!("Logging is enabled");
    } else {
        println!("Logging is disabled");
    }

    match matches.subcommand() {
        Some(("gen", sub_m)) => {
            let file_path = sub_m.get_one::<String>("file_path").unwrap();
            let dir = sub_m.get_one::<String>("dir").unwrap();

            println!(
                "Running gen with file_path: {}, dir: {}, workers: {}, numcpus: {}, block_size: {}, buffer_size: {}",
                file_path, dir, workers, numcpus, block_size, buffer_size
            );
            let results =
                search_files(&path_str, &file_filter, main_args.recursive_enabled).unwrap();

            process_files(main_args, results);
        }
        Some(("split", sub_m)) => {
            let file_path = sub_m.get_one::<String>("file_path").unwrap();
            let dir = sub_m.get_one::<String>("dir").unwrap();

            println!(
                "Running split with file_path: {}, dir: {}, workers: {}, numcpus: {}, block_size: {}, buffer_size: {}",
                file_path, dir, workers, numcpus, block_size, buffer_size
            );
        }
        Some(("rebuild", sub_m)) => {
            let file_path = sub_m.get_one::<String>("file_path").unwrap();
            let dir = sub_m.get_one::<String>("dir").unwrap();

            println!(
                "Running rebuild with file_path: {}, dir: {}, workers: {}, numcpus: {}, block_size: {}, buffer_size: {}",
                file_path, dir, workers, numcpus, block_size, buffer_size
            );
        }
        _ => println!("No valid subcommand was used"),
    }
}

fn main_new() {
    let mut main_args = Argumentos {
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
    let matches = Command::new("cli_tool")
        .version("1.0")
        .author("Your Name")
        .about("CLI tool for file operations")
        .arg(
            Arg::new("workers")
                .long("workers")
                .help("Number of workers")
                .num_args(1)
                .value_parser(clap::value_parser!(u32)),
        )
        .arg(
            Arg::new("max_concur")
                .long("n_max_concurrency")
                .help("Number maxim of concurrent access to a same file")
                .num_args(1)
                .value_parser(clap::value_parser!(u32)),
        )
        .arg(
            Arg::new("block_size")
                .long("block_size")
                .help("Size of blocks to process")
                .num_args(1),
        )
        .arg(
            Arg::new("buffer_size")
                .long("buffer_size")
                .help("Size of the buffer")
                .num_args(1),
        )
        .arg(
            Arg::new("log")
                .long("log")
                .help("Enable or disable logging")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("recursive")
                .short('R')
                .help("Operate recursively in directories")
                .action(clap::ArgAction::SetTrue),
        )
        .subcommand(
            Command::new("gen")
                .about("Generates something")
                .arg(
                    Arg::new("in_file_path")
                        .help("Input File/Directory path")
                        .required(true)
                        .num_args(1),
                )
                .arg(
                    Arg::new("file_filter")
                        .long("input_file_filter")
                        .help("Filter for files (e.g., *.mp4)"),
                ),
        )
        .get_matches();

    // Retrieve global options
    main_args.n_workers = *matches.get_one::<u32>("workers").unwrap_or(&4);
    main_args.n_max_concur = *matches.get_one::<u32>("max_concur").unwrap_or(&2);
    let binding_bs = "64k".to_string();
    main_args.block_size_str = matches
        .get_one::<String>("block_size")
        .unwrap_or(&binding_bs)
        .clone()
        .to_string();
    let binding_bf = "128k".to_string();
    main_args.buffer_size_str = matches
        .get_one::<String>("buffer_size")
        .unwrap_or(&binding_bf)
        .clone()
        .to_string();
    main_args.log_enabled = matches.get_flag("log");

    main_args.recursive_enabled = matches.get_flag("recursive");

    if main_args.log_enabled {
        LOG_ENABLED
            .set(true)
            .expect("LOG_ENABLED can only be set once");
        env_logger::Builder::new()
            .filter_level(LevelFilter::Info) // Set logging level (Info, Warn, etc.)
            .init();
        info!("Logging is enabled in the main thread.");
    } else {
        println!("Logging is disabled");
    }

    match matches.subcommand() {
        Some(("gen", sub_m)) => {
            main_args.in_file_path = sub_m
                .get_one::<String>("in_file_path")
                .unwrap()
                .clone()
                .to_string();

            let binding_filter = "".to_string();
            main_args.in_file_filter = sub_m
                .get_one::<String>("file_filter")
                .unwrap_or(&binding_filter)
                .clone()
                .to_string();

            let path_str = main_args.in_file_path.clone();
            let file_filter = main_args.in_file_filter.clone();
            let results =
                search_files(&path_str, &file_filter, main_args.recursive_enabled).unwrap();

            process_files(main_args, results);
        }
        _ => println!("No valid subcommand was used"),
    }
}
