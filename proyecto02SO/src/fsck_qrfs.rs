use crate::mkfs_QRFS::jr_fs;
use std::mem;
use sysinfo::{NetworkExt, NetworksExt, ProcessExt, System, SystemExt};


/**
 * checkConsistence
 * Recibe una estructura jr_fs (nuestro sistema de archivos)
 * Verifica la consistencia del sistema operativo
 * Imprime el total de memoria disponible, la memoria usada, la memoria libre entre otros datos 
 * del estado del disco
 * no retorna nada
 */
pub fn checkConsistence(fs:&jr_fs){

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