//!
//! Simple command line tool to analyse disk usage from the terminal
//!
//! # Usage
//!
//! ```text
//! $ dutree --help
//! Usage: dutree [options] <path> [<path>..]
//!
//! Options:
//!     -d, --depth [DEPTH] show directories up to depth N (def 1)
//!     -a, --aggr [N[KMG]] aggregate smaller than N B/KiB/MiB/GiB (def 1M)
//!     -s, --summary       equivalent to -da, or -d1 -a1M
//!     -u, --usage         report real disk usage instead of file size
//!     -b, --bytes         print sizes in bytes
//!     -x, --exclude NAME  exclude matching files or directories
//!     -H, --no-hidden     exclude hidden files
//!     -A, --ascii         ASCII characters only, no colors
//!     -h, --help          show help
//!     -v, --version       print version number
//! ```
//! # Screenshot
//!
//! ![dutree](https://ownyourbits.com/wp-content/uploads/2018/03/dutree-featured2.png)
//!
//! # More information
//!
//! Copyleft 2018 by Ignacio Nunez Hernanz - nacho _at_ ownyourbits _dot_ com
//!
//! GPL licensed
//!
//! More at [ownyourbits.com](https://ownyourbits.com/2018/03/25/analize-disk-usage-with-dutree)
//!
//! [github](https://github.com/nachoparker/dutree)
//!

extern crate unicode_width;
use unicode_width::UnicodeWidthStr;

extern crate unicode_segmentation;
use unicode_segmentation::UnicodeSegmentation;

extern crate getopts;
use getopts::Options;

extern crate terminal_size;
use terminal_size::{Width, Height, terminal_size};

extern crate regex;
use regex::Regex;

use std::io;
use std::path::{Path, PathBuf};
use std::fs;
#[cfg(target_os = "freebsd")]
use std::os::freebsd::fs::MetadataExt;
#[cfg(target_os = "linux")]
use std::os::linux::fs::MetadataExt;
#[cfg(target_os = "macos")]
use std::os::unix::fs::MetadataExt;
use std::env;
use std::collections::HashMap;

const VERSTR    : &str = "v0.2.17";
const DEF_WIDTH : u16  = 80;

pub enum XResult<T,S> {
    XOk(T),
    XErr(S),
    XExit,
}
use XResult::{XOk, XExit, XErr};

struct Entry<'a> {
    name    : String,
    bytes   : u64,
    color   : Option<&'a str>,
    last    : bool,
    entries : Option<Vec<Entry<'a>>>,
}

pub struct Config {
    paths       : Vec<PathBuf>,
    color_dict  : HashMap<String, String>,
    depth       : u8,
    depth_flag  : bool,
    bytes_flag  : bool,
    usage_flag  : bool,
    hiddn_flag  : bool,
    ascii_flag  : bool,
    no_dir_flg  : bool,
    aggr        : u64,
    exclude     : Vec<String>,
}

fn init_opts() -> Options {
    let mut options = Options::new();

    options.optflagopt( "d", "depth"    , "show directories up to depth N (def 1)", "DEPTH" );
    options.optflagopt( "a", "aggr"     , "aggregate smaller than N B/KiB/MiB/GiB (def 1M)", "N[KMG]");
    options.optflag(    "s", "summary"  , "equivalent to -da, or -d1 -a1M"                );
    options.optflag(    "u", "usage"    , "report real disk usage instead of file size"   );
    options.optflag(    "b", "bytes"    , "print sizes in bytes"                          );
    options.optflag(    "f", "files-only","skip directories for a fast local overview"    );
    options.optmulti(   "x", "exclude"  , "exclude matching files or directories", "NAME" );
    options.optflag(    "H", "no-hidden", "exclude hidden files"                          );
    options.optflag(    "A", "ascii"    , "ASCII characters only, no colors"              );
    options.optflag(    "h", "help"     , "show help"                                     );
    options.optflag(    "v", "version"  , "print version number"                          );
    options
}

impl Config {
    pub fn new() -> XResult<Config, String> {

        let args: Vec<String> = env::args().collect();
        let program = args[0].clone();

        let options = init_opts();
        let opt = match options.parse(&args[1..]) {
            Ok(m)    => m,
            Err(err) =>{ print_usage(&program, &options); return XErr( err.to_string() )},
        };

        if opt.opt_present("h") {
            print_usage(&program, &options);
            return XExit;
        }
        if opt.opt_present("v") {
            println!("dutree version {}", VERSTR);
            return XExit;
        };

        let color_dict = create_color_dict();

        let mut paths : Vec<PathBuf> = Vec::new();
        if opt.free.len() == 0 {
            let mut path = std::path::PathBuf::new();
            path.push( ".".to_string() );
            paths.push( path );
        } else {
            for opt in &opt.free {
                let mut path = std::path::PathBuf::new();
                path.push( &opt );
                paths.push( path );
            }
        }

        for p in &paths {
            if !p.exists() {
                return XErr( format!( "path {} doesn't exist", p.display() ) );
            }
        }

        let mut depth_flag = opt.opt_present("d");
        let depth_opt = opt.opt_str("d");
        let mut depth = depth_opt.unwrap_or("1".to_string()).parse().unwrap_or(1);

        let bytes_flag = opt.opt_present("b");
        let usage_flag = opt.opt_present("u");
        let hiddn_flag = opt.opt_present("H");
        let ascii_flag = opt.opt_present("A");
        let no_dir_flg = opt.opt_present("f");

        let mut aggr = if opt.opt_present("a") {
            let aggr_opt = opt.opt_str("a");
            let aggr_val = aggr_opt.unwrap_or("1M".to_string());

            if !Regex::new(r"^\d+\D?$").unwrap().is_match( aggr_val.as_str() ){
                return XErr( format!( "invalid argument '{}'", aggr_val ) );
            }

            let unit = aggr_val.matches(char::is_alphabetic).next().unwrap_or("B");
            let num : Vec<&str> = aggr_val.matches(char::is_numeric).collect();
            let num : u64       = num.concat().parse().unwrap();

            let factor = match unit {
                "b" | "B" => 1024u64.pow(0),
                "k" | "K" => 1024u64.pow(1),
                "m" | "M" => 1024u64.pow(2),
                "g" | "G" => 1024u64.pow(3),
                "t" | "T" => 1024u64.pow(4),
                _         => 1024u64.pow(0),
            };
            num * factor
        } else {
            0
        };

        let exclude = opt.opt_strs("x");

        if opt.opt_present("s") {
            depth_flag = true;
            depth      = 1;
            aggr       = 1024u64.pow(2);
        }

        XOk( Config{ paths, color_dict, depth, depth_flag, bytes_flag,
            usage_flag, hiddn_flag, ascii_flag, no_dir_flg,  aggr, exclude } )
    }
}

fn try_is_symlink( path : &Path ) -> bool {
    let metadata = path.symlink_metadata();
    metadata.is_ok() && metadata.unwrap().file_type().is_symlink()
}

fn file_name_from_path( path : &Path ) -> String {
    let mut abspath = std::env::current_dir().unwrap();
    abspath.push( path );

    // don't resolve links
    if !try_is_symlink( path ) {
        abspath = abspath.canonicalize().unwrap_or( abspath );
    }

    abspath.file_name().unwrap_or( std::ffi::OsStr::new( "/" ) )  // '/' has no filename
           .to_str().unwrap_or( "[invalid name]" ).to_string()
}

fn try_read_dir( path : &Path ) -> Option<fs::ReadDir> {
    if try_is_symlink( path ) { return None } // don't follow symlinks
    match path.read_dir() {
        Ok(dir_list) => Some(dir_list),
        Err(err)     => {
            print_io_error( path, err );
            None
        },
    }
}

fn try_bytes_from_path( path : &Path, usage_flag : bool ) -> u64 {

    match path.symlink_metadata() {
        #[cfg(any(target_os = "freebsd", target_os = "linux"))]
        Ok(metadata) => if usage_flag { metadata.st_blocks()*512 } else { metadata.st_size() },
        #[cfg(target_os = "macos")]
        Ok(metadata) => if usage_flag { metadata.blocks()*512 } else { metadata.size() },
        Err(err)     => {
            print_io_error( path, err );
            0
        },
    }
}

fn path_from_dentry( entry : Result<fs::DirEntry, io::Error> ) -> Option<std::path::PathBuf> {
    match entry {
        Ok(entry) => {
            Some( entry.path() )
        },
        Err(err)  => {
            eprintln!( "Couldn't read entry ({:?})", err.kind() );
            None
        },
    }
}

fn print_io_error( path: &Path, err: io::Error ) {
    eprintln!( "Couldn't read {} ({:?})", file_name_from_path( path ), err.kind() )
}

impl<'a> Entry<'a> {
    fn new( path: &Path, cfg : &'a Config, depth : u8 ) -> Entry<'a> {
        let name = file_name_from_path( path );

        // recursively create directory tree of entries up to depth
        let depth = if cfg.depth_flag { depth - 1 } else { 1 };

        let entries = if path.is_dir() && ( !cfg.depth_flag || depth > 0 ) {
            let mut aggr_bytes = 0;
            if let Some( dir_list ) = try_read_dir( path ) {
                let mut vec : Vec<Entry> = Vec::with_capacity( dir_list.size_hint().0 );
                for entry in dir_list {
                    if let Some( path ) = path_from_dentry( entry ) {
                        let entry_name = &file_name_from_path(&path);

                        // argument filters
                        if cfg.exclude.iter().any( |p| entry_name == p ){ continue }
                        if cfg.hiddn_flag && entry_name.starts_with("."){ continue }
                        if cfg.no_dir_flg && path.is_dir()              { continue }

                        let entry = Entry::new( &path.as_path(), cfg, depth );
                        if cfg.aggr > 0 && entry.bytes < cfg.aggr {
                            aggr_bytes += entry.bytes;
                        } else {
                            vec.push( entry );
                        }
                    }
                }
                vec.sort_unstable_by( |a, b| b.bytes.cmp( &a.bytes ) );
                if aggr_bytes > 0 {
                    vec.push( Entry {
                        name: "<aggregated>".to_string(),
                        bytes: aggr_bytes,
                        color: None,
                        last : true,
                        entries: None,
                    } );
                }

                let len = vec.len();
                if len > 0 {
                    vec[len-1].last = true;
                }

                Some( vec )
            } else { None }
        } else { None };

        // calculate sizes
        let bytes = if let Some(ref entries) = entries {
            let mut total = try_bytes_from_path( path, cfg.usage_flag );
            for entry in entries {
                total += entry.bytes;
            }
            total
        } else {
            get_bytes( path, cfg.usage_flag )
        };

        // calculate color
        let color = if !cfg.ascii_flag {color_from_path(path, &cfg.color_dict)} else {None};

        Entry { name, bytes, color, last: false, entries }
    }

    fn print_entries( &self, open_parents : Vec<bool>, parent_vals : Vec<u64>,
                      bytes_flag : bool, ascii_flag : bool,
                      max_bytes : u64, bar_width : usize, tree_name_width : usize ) {
        if let Some(ref entries) = self.entries {
            for entry in entries {
                let mut op    = open_parents.clone();
                let mut bytes = parent_vals.clone();
                bytes.push( entry.bytes );

                // make sure the name column has the right length
                let tree_width = (open_parents.len() + 1) * 3; // 3 chars per tree branch
                if tree_name_width >= tree_width {
                    let name_width  = tree_name_width - tree_width;
                    let length = UnicodeWidthStr::width(entry.name.as_str());

                    // truncate Unicode string to name_width
                    let graphemes = UnicodeSegmentation::graphemes( entry.name.as_str(), true );
                    let mut i   = 0;
                    let mut vec = Vec::new();
                    for cluster in graphemes {
                        let w = UnicodeWidthStr::width( cluster );
                        if i + w <= name_width {
                            i += w;
                            vec.push( cluster );
                        }
                    }
                    let mut name = String::new();
                    vec.iter().for_each( |cluster| name.push_str( cluster ) );

                    // surround name by ANSII color escape sequences
                    if let Some( ref col_str ) = entry.color {
                        name.insert( 0, 'm' );
                        name.insert( 0, 0o33 as char );
                        name.insert( 1, '[' );
                        name.insert_str( 2, col_str );
                        name.push( 0o33 as char );
                        name.push_str( "[0m" );
                    }

                    if length < name_width {
                        (length..name_width).for_each( |_| name.push( ' ' ) );
                    }

                    // draw the tree
                    for open in &open_parents {
                        if   *open { print!( "   " ); }
                        else       { print!( "│  " ); }
                    }
                    if   entry.last { print!( "└─ " ); op.push( true  ); }
                    else            { print!( "├─ " ); op.push( false ); }

                    // print it
                    println!( "{} {} {:>13}",
                              name,
                              fmt_bar( &bytes, max_bytes, bar_width, ascii_flag ),
                              fmt_size_str( entry.bytes, bytes_flag ) );
                    if let Some(_) = entry.entries {
                        entry.print_entries( op, bytes, bytes_flag, ascii_flag,
                                             max_bytes, bar_width, tree_name_width );
                    }
                }
            }
        }
    }

    fn print( &self, bytes_flag : bool, ascii_flag : bool ) {

        // calculate plot widths
        let mut twidth = DEF_WIDTH;
        let size = terminal_size();
        if let Some( ( Width(w), Height(_h) ) ) = size {
            twidth = w;
        } else {
            // FIXME: doesn't seem to work when piping
            // eprintln!("Unable to get terminal size");
        }
        let size_width      = 15;
        let var_width       = (twidth - size_width) as usize;
        let tree_name_width = 25.max(var_width * 25 / 100);
        let bar_width = var_width - tree_name_width;

        // initalize
        let     open_parents : Vec<bool> = Vec::new();
        let mut parent_vals  : Vec<u64>  = Vec::new();
        let max_bytes = match self.entries {
            Some(ref entries) => entries.iter().map(|e| e.bytes).max().unwrap_or(self.bytes),
            None => self.bytes,
        };
        parent_vals.push( self.bytes );

        // print
        println!( "[ {} {} ]", self.name, fmt_size_str( self.bytes, bytes_flag ) );
        self.print_entries( open_parents, parent_vals, bytes_flag, ascii_flag,
                            max_bytes, bar_width, tree_name_width );
    }
}

fn fmt_bar( bytes : &Vec<u64>, max_bytes : u64, width : usize, ascii_flag : bool ) -> String {
    let width = width as u64 - 2 - 5; // not including bars and percentage

    let mut str = String::with_capacity( width as usize );
    str.push( '│' );

    let mut bytesi = bytes.iter();
    let _ = bytesi.next();
    let mut total  = &max_bytes;
    let mut part   = bytesi.next().unwrap();
    let mut bars = match total {
        0 => 0,
        _ => (part * width) / total,
    };
    let mut pos    = width - bars;

    let block_char = if ascii_flag { vec![ ' ', '#' ] } else { vec![ ' ', '░', '▒', '▓', '█' ] };
    let mut chr    = 0;
    let levels = bytes.len() - 1;

    for x in 0..width {
        if x > pos {
            total = part;
            part  = bytesi.next().unwrap_or(&0);
            bars = match total { 0 => 0, _ => (part * bars) / total };
            pos = width - bars;
            chr += 1;
            if chr == levels || chr >= block_char.len() {
                chr = block_char.len() - 1;          // last level, solid '█'
            }
        }
        str.push( block_char[chr] );
    }

    let nominator = bytes[bytes.len()-1] * 100;
    let denominator = bytes[bytes.len()-2];
    let result = match denominator {
        0 => 0,
        _ =>  nominator/denominator,
    };
    format!( "{}│ {:3}%", str, result )
}

fn fmt_size_str( bytes : u64, flag : bool ) -> String {
    let b = bytes as f32;
    if      bytes < 1024 || flag   { format!( "{:.2} B"  , bytes                    ) }
    else if bytes < 1024u64.pow(2) { format!( "{:.2} KiB", b/1024.0                 ) }
    else if bytes < 1024u64.pow(3) { format!( "{:.2} MiB", b/(1024u32.pow(2) as f32)) }
    else if bytes < 1024u64.pow(4) { format!( "{:.2} GiB", b/(1024u32.pow(3) as f32)) }
    else                           { format!( "{:.2} TiB", b/(1024u64.pow(4) as f32)) }
}

fn get_bytes( path: &Path, usage_flag : bool ) -> u64 {
    if path.is_dir() {
        let mut bytes : u64 = try_bytes_from_path( path, usage_flag );
        if let Some(dir_list) = try_read_dir( path ) {
            for entry in dir_list {
                if let Some(path) = path_from_dentry( entry ) {
                    bytes += get_bytes( &path, usage_flag );
                }
            }
        }
        bytes
    } else {
        try_bytes_from_path( path, usage_flag )
    }
}

fn color_from_path<'a>( path : &Path, color_dict : &'a HashMap<String, String> ) -> Option<&'a str> {
    if try_is_symlink( path ) {
        let path_link = path.read_link();
        if path_link.is_ok() {
            if path_link.unwrap().exists() {
                if let Some(col) = color_dict.get(&"ln".to_string()) {
                    return Some(&col);
                }
            }
        }
        if let Some( col ) = color_dict.get( &"or".to_string() )  {
            return Some( &col );
        }
    }
    let metadata = path.symlink_metadata();
    if metadata.is_ok() {
        #[cfg(any(target_os = "freebsd", target_os = "linux"))]
        let mode = metadata.unwrap().st_mode();
        #[cfg(target_os = "macos")]
        let mode = metadata.unwrap().mode();
        if path.is_dir() {
            if mode & 0o002 != 0 {  // dir other writable
                if let Some( col ) = color_dict.get( &"ow".to_string() ) {
                    return Some( &col );
                }
            }
            if let Some( col ) = color_dict.get( &"di".to_string() ) {
                return Some( &col );
            }
        }
        if mode & 0o111 != 0 {  // executable
            if let Some( col ) = color_dict.get( &"ex".to_string() ) {
                return Some( &col );
            }
        }
    }
    if let Some( ext_str ) = path.extension() {
        for ( key , _ ) in color_dict {
            if &key[..2] != "*." { continue }
            let k = key.trim_start_matches( "*." );
            if ext_str == k {
                return Some( &color_dict.get( key ).unwrap() );
            }
        }
    }
    if path.is_file() {
        if let Some( col ) = color_dict.get( &"fi".to_string() ) {
            return Some( &col );
        }
        else { return None }
    }
    // we are assuming it can only be a 'bd','cd'. can also be 'pi','so' or 'no'
    if let Some( col ) = color_dict.get( &"bd".to_string() ) {
        return Some( &col );
    }
    None
}

fn print_usage( program: &str, opts: &Options ) {
    let brief = format!( "Usage: {} [options] <path> [<path>..]", program );
    print!( "{}", opts.usage( &brief ) );
}

fn create_color_dict() -> HashMap<String, String> {
    let env_str = env::var("LS_COLORS").unwrap_or( "".to_string() );
    let colors  = env_str.split(':');
    let mut color_dict = HashMap::with_capacity( colors.size_hint().0 );
    for entry in colors {
        if entry.len() == 0 { break; }

        let     line = entry.replace( "\"", "" );
        let mut line = line.split('=');
        let key      = line.next().unwrap();
        let val      = line.next().unwrap();

        color_dict.insert( key.to_string(), val.to_string() );
    }
    color_dict
}

pub fn run( cfg: &Config ) {
    let entry = if cfg.paths.len() == 1 {
        Entry::new( cfg.paths[0].as_path(), &cfg, cfg.depth + 1 )
    } else {
        let mut bytes = 0;
        let mut entries : Vec<Entry> = Vec::with_capacity( cfg.paths.len() );

        for path in &cfg.paths {
            let e = Entry::new( path.as_path(), &cfg, cfg.depth + 1 );
            bytes += e.bytes;
            entries.push( e );
        }
        entries.sort_unstable_by( |a, b| b.bytes.cmp( &a.bytes ) );
        let len = entries.len();
        if len > 0 {
            entries[len-1].last = true;
        }
        Entry {
            name    : "<collection>".to_string(),
            bytes,
            color   : None,
            last    : false,
            entries : Some(entries)
        }
    };

    entry.print( cfg.bytes_flag, cfg.ascii_flag );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ls_colors() {
        let mut dict = HashMap::<String, String>::new();
        dict.insert( "di".to_string(), "dircode".to_string() );
        dict.insert( "li".to_string(), "linkcod".to_string() );
        dict.insert( "*.mp3".to_string(), "mp3code".to_string() );
        dict.insert( "*.tar".to_string(), "tarcode".to_string() );
        assert_eq!( "dircode", color_from_path( Path::new(".")       , &dict ).unwrap() );
        assert_eq!( "mp3code", color_from_path( Path::new("test.mp3"), &dict ).unwrap() );
        assert_eq!( "tarcode", color_from_path( Path::new("test.tar"), &dict ).unwrap() );
    }

    /*
    #[test]
    fn plot_bar() {
        println!("{}", fmt_bar(100, 100, 40 ) );
        println!("{}", fmt_bar( 90, 100, 40 ) );
        println!("{}", fmt_bar( 40, 100, 40 ) );
        println!("{}", fmt_bar( 30, 100, 40 ) );
        println!("{}", fmt_bar( 20, 100, 40 ) );
        println!("{}", fmt_bar( 10, 100, 40 ) );
        println!("{}", fmt_bar(  0, 100, 40 ) );
    }

    #[test]
    fn path_flavours() {
    // different paths, like . .. ../x /home/ dir / /usr/etc/../bin
    }

    #[test]
    fn get_bytes_test() {
        println!( "calculated bytes {}",
                  get_bytes( Path::new( "." ) ) );
        println!( "calculated bytes {}",
                  get_bytes( Path::new( "Cargo.toml" ) ) );
    }
    */
}

// License
//
// This script is free software; you can redistribute it and/or modify it
// under the terms of the GNU General Public License as published by
// the Free Software Foundation; either version 2 of the License, or
// (at your option) any later version.
//
// This script is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this script; if not, write to the
// Free Software Foundation, Inc., 59 Temple Place, Suite 330,
