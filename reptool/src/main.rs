use std::env;
use std::fs;
use std::io::{self, Seek, Read, Write, ErrorKind};
use std::path::{Path};
use std::process;

use regex::Regex;
use getopts::Options;

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
    print!("{}", opts.usage(&brief));
}

fn replace_files(extensions: &[&str], option: &RepToolOption, copy_enable: bool) -> io::Result<()> {
    let input_dir = Path::new(&option.input_path);
    let output_dir = Path::new(&option.output_path);

    if copy_enable {
        // Create the output directory if it doesn't exist
        if !output_dir.exists() {
            fs::create_dir_all(output_dir)?;
        }
    }

    // Iterate over the files in the input directory
    let files = fs::read_dir(input_dir)?;
    for file in files {
        let file = file?;
        let file_path = file.path();

        if file_path.is_file() {
            // Check if the file has one of the desired extensions
            if let Some(file_extension) = file_path.extension() {
                // Copy and process in output path for all related extension
                if copy_enable {
                    if extensions.contains(&file_extension.to_str().unwrap()) {
                        let file_name = file_path.file_name().unwrap();
                        let output_file_path = output_dir.join(file_name);
                        let output_path_str = &output_file_path.to_str().unwrap();

                        // Copy the file to the output directory
                        fs::copy(&file_path, &output_file_path)?;
                        if option.verbose_mode {
                            println!("Copied file: {}", output_file_path.to_str().unwrap());
                        }

                        // Replace the file .torrent.rtorrent
                        if file_extension.to_str().unwrap() == "rtorrent" {
                            if let Err(e) = replace_string_in_file(output_path_str, &option.keyword, &option.search_string, &option.replace_string, option.verbose_mode) {
                                println!("Error: Replacing string error in file '{}': {}", output_path_str, e);
                                process::exit(1);
                            }
                        }
                    }
                } else {
                    // Process file in input path by default
                    let input_path_str = file_path.to_str().unwrap();

                    // Replace the file .torrent.rtorrent
                    if file_extension.to_str().unwrap() == "rtorrent" {
                        if let Err(e) = replace_string_in_file(input_path_str, &option.keyword, &option.search_string, &option.replace_string, option.verbose_mode) {
                            println!("Error: Replacing string error in file '{}': {}", input_path_str, e);
                            process::exit(1);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

fn replace_string_in_file(file_path: &str, key: &str, find: &str, replace: &str, verbose: bool) -> io::Result<bool> {
    if verbose {
       println!("Processing file: {}", file_path);
    }

    let mut is_found = false;
    let mut file = fs::OpenOptions::new().read(true).write(true).open(file_path)?;
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

fn main() {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

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

    let extensions = ["rtorrent", "torrent", "libtorrent_resume"];
    match replace_files(&extensions, &option, copy_enable) {
        Ok(()) =>
        {
            if option.verbose_mode {
                println!("All files processed successfully.");
            }
        }
        Err(e) => match e.kind() {
            ErrorKind::NotFound => {
                println!("Error: Input directory not found.");
                process::exit(1);
            }
            ErrorKind::PermissionDenied => {
                println!("Error: Permission denied to access files.");
                process::exit(1);
            }
            _ => {
                println!("Error: Copying or Replacing files error: {}", e);
                process::exit(1);
            }
        },
    }

    process::exit(0); // Return error code 0 on success
}
