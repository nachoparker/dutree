///
///  - dutree -
/// 
/// Simple command to analyse disk usage from the terminal
///
/// Usage:
///  btrfs-sync [options] <src> [<src>...] [[user@]host:]<dir>
///
///  -k|--keep NUM     keep only last <NUM> sync'ed snapshots
///  -d|--delete       delete snapshots in <dst> that don't exist in <src>
///  -z|--xz           use xz     compression. Saves bandwidth, but uses one CPU
///  -Z|--pbzip2       use pbzip2 compression. Saves bandwidth, but uses all CPUs
///  -q|--quiet        don't display progress
///  -v|--verbose      display more information
///  -h|--help         show usage
///
/// <src> can either be a single snapshot, or a folder containing snapshots
/// <user> requires privileged permissions at <host> for the 'btrfs' command
///
/// Cron example: daily synchronization over the internet, keep only last 50
///
/// cat > /etc/cron.daily/btrfs-sync <<EOF
/// #!/bin/bash
/// /usr/local/sbin/btrfs-sync -q -k50 -z /home user@host:/path/to/snaps
/// EOF
/// chmod +x /etc/cron.daily/btrfs-sync
///
/// Copyleft 2018 by Ignacio Nunez Hernanz <nacho _a_t_ ownyourbits _d_o_t_ com>
/// GPL licensed (see end of file) * Use at your own risk!
///
/// More at https://ownyourbits.com
///

extern crate dutree;

use dutree::Config;
use dutree::XResult::XErr;
use dutree::XResult::XOk;
use dutree::XResult::XExit;
use std::process;

fn main() {
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
