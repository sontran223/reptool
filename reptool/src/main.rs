use std::env;
use std::fs;
use std::io::{self, Seek, Read, Write};
use std::path::{Path};
use std::process;

use regex::Regex;
use getopts::Options;
use anyhow::{Context, Result};
use tracing::{info, span, warn, Level};
use tracing_subscriber::{filter::LevelFilter, fmt};

struct RepToolOption {
    input_path : String,
    search_string : String,
    replace_string : String,
    verbose_mode : bool,
    output_path : String,
    keyword : String,
}

fn print_usage(program: &str, opts: &Options) {
    let brief = format!("Usage: {} [options] <input_path> <search_string> <replace_string>", program);
    info!("{}", opts.usage(&brief));
}

fn replace_files(extensions: &[&str], option: &RepToolOption, copy_enable: bool) -> Result<()> {
    let input_dir = Path::new(&option.input_path);
    let output_dir = Path::new(&option.output_path);

    if copy_enable {
        // Create the output directory if it doesn't exist
        if !output_dir.exists() {
           fs::create_dir_all(output_dir).with_context(|| format!("Failed to create output directory: {:?}", &option.output_path))?;
        }
    }

    // Iterate over the files in the input directory
    let mut is_found = false;
    let files = fs::read_dir(input_dir).with_context(|| format!("Failed to read input directory: {:?}", &option.input_path))?;
    for file in files {
        let file = file?;
        let file_path = file.path();

        if file_path.is_file() {
            // Check if the file has one of the desired extensions
            if extensions.iter().any(|&end| file_path.to_str().unwrap().ends_with(end)) {
                // Copy and process in output path for all related extension
                if copy_enable {
                    let file_name = file_path.file_name().unwrap();
                    let output_file_path = output_dir.join(file_name);
                    let output_path_str = &output_file_path.to_str().unwrap();

                    // Copy the file to the output directory
                    fs::copy(&file_path, &output_file_path).with_context(|| format!("Failed to copy file {:?}", file_path))?;
                    if option.verbose_mode {
                        info!("Copied file: {}", output_file_path.to_str().unwrap());
                    }

                    // Replace the file .torrent.rtorrent
                    if output_path_str.ends_with(".torrent.rtorrent") {
                        let result: bool = replace_string_in_file(output_path_str, &option.keyword, &option.search_string, &option.replace_string, option.verbose_mode)?;
                        if result {
                            is_found = result;
                        }
                    }
                } else {
                    // Process file in input path by default
                    let input_path_str = file_path.to_str().unwrap();

                    // Replace the file .torrent.rtorrent
                    if input_path_str.ends_with(".torrent.rtorrent") {
                        let result: bool = replace_string_in_file(input_path_str, &option.keyword, &option.search_string, &option.replace_string, option.verbose_mode)?;
                        if result {
                            is_found = result;
                        }
                    }
                }
            }
        }
    }
    if !is_found {
        warn!("No matching found.");
    }

    Ok(())
}

fn replace_string_in_file(file_path: &str, key: &str, find: &str, replace: &str, verbose: bool) -> Result<bool> {
    if verbose {
       info!("Processing file: {}", file_path);
    }

    let mut is_found = false;
    let mut file = fs::OpenOptions::new().read(true).write(true).open(file_path).with_context(|| format!("Failed to open file: {:?}", file_path))?;
    let mut content = String::new();

    file.read_to_string(&mut content)?;

    // Only get directory:path to replace
    let re = Regex::new(format!(r#":({})(\d+):([^:]+)"#, key).as_str()).unwrap();
    let mat = re.find(&content).unwrap();

    let find_content = &content[mat.start()..mat.end()];

    for cap in re.captures_iter(&content) {

        // Check whether pattern exist or not

        if cap[3].contains(&find) {
            is_found = true;
            let offset_size: i32 = replace.len() as i32 - find.len() as i32;
            let num: i32 = cap[2].parse().unwrap();
            let new_size = num + offset_size;
            let mut update_string: String = ":".to_owned();
            update_string.push_str(&cap[1]);
            update_string.push_str(&new_size.to_string());
            update_string.push_str(":");
            let new_path = cap[3].replacen(find, replace, 1);
            update_string.push_str(&new_path) ;
            let modified_content = content.replace(&find_content, &update_string);

            // Update new content to file
            file.seek(io::SeekFrom::Start(0))?;
            file.write_all(modified_content.as_bytes())?;
            file.set_len(modified_content.len() as u64)?;
        }
    }

    Ok(is_found)
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    let span = span!(Level::TRACE, "reptool span");
    let _enter = span.enter();

    // Parse and validate the options
    let mut opts = Options::new();
    opts.optflag("v", "verbose", "Enable verbose output");
    opts.optopt("o", "output", "Set output path", "OUTPUT_PATH");
    opts.optopt("k", "keyword", "Set keyword to parse, \"directoy\" by default", "KEYWORD");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(e) => {
            writeln!(io::stderr(), "Error: {}", e).unwrap();
            print_usage(&program, &opts);
            process::exit(1);
        }
    };

    if matches.free.len() != 3 {
        print_usage(&program, &opts);
        process::exit(1);
    }

    // Construct the options of tool
    let mut option = RepToolOption {
        input_path : String::from(&matches.free[0]),
        search_string : String::from(&matches.free[1]),
        replace_string : String::from(&matches.free[2]),
        verbose_mode : matches.opt_present("v"),
        output_path : String::from(""),
        keyword : String::from("directory"),
    };
 
    let output_path = matches.opt_str("o");
    let keyword = matches.opt_str("k");
 
    let mut copy_enable = false;
    if let Some(output_dir) = &output_path {
        // Copy all neccessary files to new path if defined
        copy_enable = true;
        option.output_path = output_dir.to_string();
    }

    if let Some(search_key) = &keyword {
        option.output_path = search_key.to_string();
    }

    // Create the tracing subscriber with the specified level filter
    let mut level_filter = LevelFilter::WARN;
    if option.verbose_mode {
        level_filter = LevelFilter::TRACE;
    }

    let subscriber = fmt::Subscriber::builder()
        .with_max_level(level_filter)
        .finish();

    // Initialize the tracing subscriber with your custom subscriber
    tracing::subscriber::set_global_default(subscriber).expect("Failed to set the subscriber");

    let extensions = ["rtorrent", "torrent", "libtorrent_resume"];
    if option.verbose_mode {
        info!("Start replacing files ...");
    }
    replace_files(&extensions, &option, copy_enable)
        .context("Failed to modify files")
        .map(|_| info!("File modification completed successfully"))
}
