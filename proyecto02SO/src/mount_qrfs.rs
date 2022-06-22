use crate::mkfs_QRFS;
use crate::fsck_qrfs;
use std::env;
use std::ffi::OsStr;
use image;
use quircs;


pub fn mount_qrfs() {
    //Se define el backstrace para evitar errores de ejecucion
    env::set_var("RUST_BACKTRACE", "full"); 

    println!("{:?}", env::args().nth(0).unwrap());
    let mountPoint = match env::args().nth(2) {
        Some(path) => path,
        None => {
            println!("Usage: {} <MOUNTPOINT>", env::args().nth(0).unwrap());
            return;
        }
    };

    let diskDirection = env::args().nth(1).unwrap();
    println!("{:?}", diskDirection);
    let fileSis = mkfs_QRFS::jr_fs::new(mountPoint.clone(), diskDirection.clone());
    fsck_qrfs::checkConsistence(&fileSis);
    let options = ["-o", "nonempty"]
        .iter()
        .map(|o| o.as_ref())
        .collect::<Vec<&OsStr>>();
    
    println!("File System working!");
    println!("{:?}", options);
    fuse::mount(fileSis, &mountPoint, &options).unwrap();
    
}