use std::fs;
use std::io::{self, Seek, Read, Write};
use std::path::{Path};

use regex::Regex;
use clap::Parser;
use anyhow::{Context, Result};
use tracing::{info, span, warn, Level};
use tracing_subscriber::{filter::LevelFilter, fmt};

#[derive(Parser)]
#[command(name = "rtorrent_status_file_modifier")]
#[command(author = "sontran")]
#[command(version = "1.0")]
#[command(about = "Replace string for .torrent.rtorrent", long_about = "This program modifies rtorrent's status file to change the download path for an already loaded torrent.")]
struct RepToolOption {
    /// Input path contains .torrent.rtorrent
    input_path : String,

    /// Search string
    search_string : String,

    /// Replace string
    replace_string : String,

    /// Show all infos
    #[arg(short, long)]
    verbose_mode : bool,

    /// Define output path to copy and modify, untouch input path files
    #[arg(short, long, default_value_t = String::from(""))]
    output_path : String,

    /// Define keyword to search and replace
    #[arg(short, long, default_value_t = String::from("directory"))]
    keyword : String,
}

fn replace_files(extensions: &[&str], option: &RepToolOption) -> Result<()> {
    let input_dir = Path::new(&option.input_path);
    let output_dir = Path::new(&option.output_path);

    if option.output_path != "" {
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
            if extensions.iter().any(|&end| file_path.to_str().expect("Invalid file name").ends_with(end)) {
                // Copy and process in output path for all related extension
                if option.output_path != "" {
                    let file_name = file_path.file_name().expect("Missing file name");
                    let output_file_path = output_dir.join(file_name);
                    let output_path_str = &output_file_path.to_str().expect("Invalid file name");

                    // Copy the file to the output directory
                    fs::copy(&file_path, &output_file_path).with_context(|| format!("Failed to copy file {:?}", file_path))?;
                    if option.verbose_mode {
                        info!("Copied file: {}", output_file_path.to_str().expect("Invalid file name"));
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
                    let input_path_str = file_path.to_str().expect("Missing file name");

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
    let re = Regex::new(format!(r#":({})(\d+):([^:]+)"#, key).as_str()).expect("Failed to construct regex pattern");
    let mat = re.find(&content).expect("Failed to match pattern");

    let find_content = &content[mat.start()..mat.end()];

    for cap in re.captures_iter(&content) {

        // Check whether pattern exist or not

        if cap[3].contains(&find) {
            is_found = true;
            let offset_size: i32 = replace.len() as i32 - find.len() as i32;
            let num: i32 = cap[2].parse().expect("Failed to convert string len");
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

    let span = span!(Level::TRACE, "rtorrent_status_file_modifier span");
    let _enter = span.enter();

    let option: RepToolOption = RepToolOption::parse();

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
    replace_files(&extensions, &option)
        .context("Failed to modify files")
        .map(|_| info!("File modification completed successfully"))
}
