use crate::mkfs_QRFS;
use crate::fsck_qrfs;
use std::env;
use std::ffi::OsStr;
use image;
use quircs;


pub fn mount_qrfs() {

    println!("{:?}", env::args().nth(0).unwrap(););
    let mountPoint = env::args().nth(2).unwrap();
    println!("{:?}", mountPoint);
    let diskDirection = env::args().nth(1).unwrap();
    println!("{:?}", diskDirection);
    let fileSis = my_QRFS::fileSystem::new(mountPoint.clone(), diskDirection.clone());
    fsck_qrfs::checkConsistence(&fileSis);
    let options = ["-o", "nonempty"]
        .iter()
        .map(|o| o.as_ref())
        .collect::<Vec<&OsStr>>();
    
    println!("File System working!");
    fuse::mount(fileSis, &mountPoint, &options).unwrap();
    
}