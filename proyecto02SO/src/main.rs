mod mount_qrfs;
mod mkfs_QRFS;
mod fsck_qrfs;
mod sesInformation;

use std::env;
use std::ffi::OsStr;
use image;
use quircs;

fn main() {
    mount_qrfs::mount_qrfs();
}