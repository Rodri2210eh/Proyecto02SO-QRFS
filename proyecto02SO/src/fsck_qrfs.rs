use crate::mkfs_QRFS::fileSystem;
use std::mem;
use sysinfo::{NetworkExt, NetworksExt, ProcessExt, System, SystemExt};

//Checamos cuanto espacio disponible y cuanto usado en general vemos el estado del disco
pub fn checkConsistence(fs:&fileSystem){

    let mut sys = System::new_all();
    sys.refresh_all();
    println!("=> System:");
    let totalMemory = sys.total_memory();
    println!("total memory: {} KB", totalMemory);
    let usedMemory = sys.used_memory();
    println!("used memory : {} KB", usedMemory);
    let usedMemoryFS = mem::size_of_val(fs.getDisk());
    println!("FileSystem SPACE USED : {} KB", usedMemoryFS);
    let memoryBlockUsed = mem::size_of_val(&fs.getDisk().superBlock)*&fs.getDisk().superBlock.len();
    println!("FileSystem::MEMORY BLOCK SPACE USED : {} KB", memoryBlockUsed);
    let superBlockUsed = mem::size_of_val(&fs.getDisk().memoryBlock)*&fs.getDisk().memoryBlock.len();
    println!("FileSystem::SUPER BLOCK SPACE USED : {} KB", superBlockUsed);


    println!("FileSystem SPACE AVAILABLE : {} KB", totalMemory-usedMemory);

}