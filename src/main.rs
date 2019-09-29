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

extern crate dutree;

use dutree::Config;
use dutree::XResult::XErr;
use dutree::XResult::XOk;
use dutree::XResult::XExit;
use std::process;

fn main() {

    // handle SIGPIPE
    let _signal = unsafe { signal_hook::register(signal_hook::SIGPIPE, || process::exit(0)) };

    // Parse arguments
    let cfg = match Config::new() {
        XOk(cfg)  => cfg,
        XExit     => process::exit(0),
        XErr(err) => {
            eprintln!( "{}", err );
            process::exit(1)
        }
    };

    // Execution
    dutree::run( &cfg );
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
