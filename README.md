This program modifies rtorrent's status file to change the download path for an already loaded torrent

Usage: rtorrent_status_file_modifier [OPTIONS] <INPUT_PATH> <SEARCH_STRING> <REPLACE_STRING>

Arguments:
  <INPUT_PATH>
          Input path contains .torrent.rtorrent

  <SEARCH_STRING>
          Search string

  <REPLACE_STRING>
          Replace string

Options:
  -v, --verbose-mode
          Show all infos

  -o, --output-path <OUTPUT_PATH>
          Define output path to copy and modify, untouch input path files

          [default: ]

  -k, --keyword <KEYWORD>
          Define keyword to search and replace

          [default: directory]

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
