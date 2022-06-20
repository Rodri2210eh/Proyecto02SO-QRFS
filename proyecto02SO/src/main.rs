mod mountFS;
mod sesInformation;
use std::env;
use std::ffi::OsStr;
use image;
use quircs;
fn main() {
    
    let mountpoint = match env::args().nth(1) {
        Some(path) => path,
        None => {
            println!("Usage: {} <MOUNTPOINT>", env::args().nth(0).unwrap());
            return;
        }
    };
    println!(mountpoint);
    let fs = mountFS::Rb_fs::new(mountpoint.clone());
    println!("Sistema de archivos !");
    let options = ["-o", "nonempty"]
        .iter()
        .map(|o| o.as_ref())
        .collect::<Vec<&OsStr>>();

    println!("RB-FS started!");
    fuse::mount(fs, &mountpoint, &options).unwrap();
    
}