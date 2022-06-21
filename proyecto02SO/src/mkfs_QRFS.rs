use fuse::{Filesystem, Request, ReplyCreate, ReplyEmpty, ReplyAttr, ReplyEntry, ReplyOpen, ReplyStatfs,ReplyData, ReplyDirectory, ReplyWrite, FileType, FileAttr};
use libc::{ENOSYS, ENOENT, EIO, EISDIR, ENOSPC};
use std::ffi::OsStr;
use std::mem;
use crate::mkfs_QRFS;
use serde::{Serialize, Deserialize};
use crate::sesInformation::FileAttrDef;
use qrcode::QrCode;
use image::Luma;

//                                    ---CODIGO DEL ALMACENAJE DE NUESTRO FS                                    ---


//Los Inodes son la unidad que movera nuestro fs
#[derive(Serialize, Deserialize)]
pub struct Inode {
    pub name: String,
    #[serde(with = "FileAttrDef")]
    pub attributes : FileAttr,
    pub references: Vec<usize>
}

impl Inode {
    pub fn changeName(&mut self,value: String) {
        self.name = value;
    }
    //Agrega una referencia a si mismo
    pub fn addReference(&mut self,refValue: usize) {
        self.references.push(refValue);
    }
    //Elimina una referencia a si mismo
    pub fn deleteReference(&mut self,refValue: usize) {
        self.references.retain(|i| *i != refValue);
    }
}

//Se guarda el contenido de cada iNode creado
#[derive(Serialize, Deserialize)]
pub struct memBlock {
    referenceInode : u64,
    data : Vec<u8>
}

impl memBlock {
    //Agrega una referencia a si mismo
    pub fn addData(&mut self,data: u8) {
        self.data.push(data);
    }
    //Elimina una referencia a si mismo
    pub fn deleteData(&mut self,data: u8) {
        self.data.retain(|i| *i != data);
    }
}

//Creamos una estructura para guardar nuestros archivos Inodes
//El super bloque contiene los inodes del sistema
//tambien la memoria de cada inote
#[derive(Serialize, Deserialize)]//Con esto podemos guardar el so
pub struct Disk {
    sigInode: u64,
    pub superBlock : Vec<Inode>,
    pub memoryBlock : Vec<memBlock>,
    pub rootPath: String
}

impl Disk {
    //Crea un nuevo disco y crea el inode raiz
    pub fn new(path:String, diskPath:String) -> Disk{
        
        println!("    Creating Disk...");
        unsafe{
            let mut memBlock = Vec::new();
            let mut blocks = Vec::new(); //Aca guardamos los inodes
            let timespe = time::now().to_timespec();
            let attr = FileAttr {
                ino: 1,
                size: 0,
                blocks: 0,
                atime: timespe,
                mtime: timespe,
                ctime: timespe,
                crtime: timespe,
                kind: FileType::Directory,
                perm: 0o755,
                nlink: 0,
                uid: 0,
                gid: 0,
                rdev: 0,
                flags: 0,
            };
            let name = ".";
            let firstInode = Inode {
                name : name.to_string(),
                attributes : attr,
                references : Vec::new()
            };
            
            blocks.push(firstInode);
            
            let newDisk = Disk {sigInode : 1 as u64, superBlock : blocks, memoryBlock : memBlock,rootPath :  path};
            if pathValidate(diskPath.clone()) {
                println!("existe el disco");
                let loadDisk = loadFS(diskPath);
                match loadDisk {
                    Some(loadDisk) => {
                        return loadDisk;
                    },
                    None => {
                        return newDisk;
                    }
                }
            }
            return newDisk;
        }         
    }

    //Retorna el siguiente inod disponible
    pub fn newInode(&mut self) -> u64{
        unsafe{
            self.sigInode = self.sigInode +1;
            return self.sigInode;
        }
        
    }

    //Agrega el inode al super bloque
    pub fn writeInode(&mut self, inode:Inode) {
        self.superBlock.push(inode);
    }

    //Elimina el inode disponible
    pub fn removeInode(&mut self, inode:u64) {
        self.superBlock.retain(|i| i.attributes.inod != inode);
    }

    //Elimina una referencia de un respectivo inode
    pub fn clearReference(&mut self, inod: u64, refValue: usize) {
        for i in 0..self.superBlock.len() {
            if self.superBlock[i].attributes.inod == inod {
                self.superBlock[i].deleteReference(refValue);
            }
         }
    }

    //Agrega una respectiva referencia a un inode
    pub fn addReference(&mut self, inod: u64, refValue: usize) {
        for i in 0..self.superBlock.len() {
            if self.superBlock[i].attributes.inod == inod {
                self.superBlock[i].addReference(refValue);
            }
         }
    }

     //Obtiene un Inode o nada
    pub fn getInode(&self, inod: u64) -> Option<&Inode> {
        for i in 0..self.superBlock.len() {
            if self.superBlock[i].attributes.inod == inod {
                return Some(&self.superBlock[i]);
            }

         }
         return None;
    }

    //Obtiene un Inode mutable o nada
    pub fn getMutInode(&mut self, inod: u64) -> Option<&mut Inode> {
        for i in 0..self.superBlock.len() {
            if self.superBlock[i].attributes.inod == inod {
                return Some(&mut self.superBlock[i]);
            }

         }
         return None;
    }

    //Busca en base a la carpeta del padre el hijo que tenga el nombre por parametro
    pub fn findInodeByName(&self, parentInode: u64, name: &str) -> Option<&Inode> {
        for i in 0..self.superBlock.len() {
           if self.superBlock[i].attributes.inod == parentInode {
            let parent =  &self.superBlock[i];
            for j in 0..parent.references.len() {
                for k in 0..self.superBlock.len() {
                    if self.superBlock[k].attributes.inod == parent.references[j].try_into().unwrap() {
                        let child =  &self.superBlock[k];
                        if child.name == name {
                            return Some(child);
                        }
                    }
                }
            }
           }
        }
        
        return None;
        
    }

    //Agrega data al bloque de memoria asociado al inod
    pub fn addDataInode(&mut self, inod:u64,data:u8) {
        for i in 0..self.memoryBlock.len() {
            if self.memoryBlock[i].referenceInode == inod {
                self.memoryBlock[i].addData(data) ;
            }
        }
    }

    //Escribe un arreglo de bites dentro de un inode 
    pub fn writeContent(&mut self, referenceInode: u64, content: Vec<u8>) {
        for i in 0..content.len(){
            self.addDataInode(referenceInode, content[i]);

        }
    }

    //Elimina la data el bloque de memoria asociado al inod
    pub fn deleteDataInode(&mut self, inod:u64,data: u8) {
        for i in 0..self.memoryBlock.len() {
            if self.memoryBlock[i].referenceInode == inod {
                self.memoryBlock[i].deleteData(data);
            }
        }
    }

    //Obtiene el contenido de un arreglo 
    pub fn getBytesContent(&self, inod: u64) -> Option<&[u8]> {
        for i in 0..self.memoryBlock.len() {
            if self.memoryBlock[i].referenceInode == inod {
                let bytes = &self.memoryBlock[i].data[..];
                return Some(bytes);
            }
        }
        return None;
    }
}



//                                        -ACA INICIA EL CODIGO DEL FILESYSTEM                                    -


//Nuestro fs tiene un disco
pub struct fileSystem {
    disk : Disk
}
impl fileSystem {
    pub fn new(rootPath:String, diskPath:String) -> Self{
        let newDisk = Disk::new(rootPath.to_string(), diskPath);
        fileSystem {
            disk : newDisk
        }
    }

    pub fn getDisk(&self) -> &Disk {
        return &self.disk;
    }

    pub fn setDisk(&mut self,newDisk:Disk) {
        self.disk = newDisk;
    }

    pub fn saveFileSystem(&self){
        let encodeFS = encode(&self.disk);
        saveQR(encodeFS);
    }
    
}

impl Drop for fileSystem {
    fn drop(&mut self) {
        &self.saveFileSystem();
        println!("Save FileSystem");
    }
}

impl Filesystem for fileSystem {

    //Mira dentro de un directorio por su nombre y obtiene sus atributos
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {

        let fileName = name.to_str().unwrap();
        let inode = self.disk.findInodeByName(parent, fileName);
        match inode {
            Some(inode) => {
                let timeInode = time::now().to_timespec();
                reply.entry(&timeInode, &inode.attributes, 0);
                println!("    FileSystem LookUp");
            },
            None => {
                reply.error(ENOENT);
            }
        }
    }

    //Crea un archivo en la padre pasado poor parametro
    fn create(&mut self, _req: &Request, parent: u64, name: &OsStr, mode: u32, flags: u32, reply: ReplyCreate) {

        let availableInode = self.disk.newInode();
        let memBlock = memBlock {
            referenceInode : availableInode,
            data : Vec::new()
        };

        let timespe = time::now().to_timespec();

        let attr = FileAttr {
            ino: availableInode,
            size: 0,
            blocks: 1,
            atime: timespe,
            mtime: timespe,
            ctime: timespe,
            crtime: timespe,
            kind: FileType::RegularFile,
            perm: 0o755,
            nlink: 0,
            uid: 0,
            gid: 0,
            rdev: 0,
            flags,
        };
        
        let name = name.to_str().unwrap();

        let mut inode = Inode {
            name: name.to_string(),
            attributes: attr,
            references: Vec::new()
        };

        inode.references.push(memBlock.referenceInode as usize);

        self.disk.writeInode(inode);
        
        self.disk.addReference(parent, availableInode as usize);
        self.disk.memoryBlock.push(memBlock);
        println!("    FileSystem Created...");

        reply.created(&timespe, &attr, 1, availableInode, flags)
    }

    //Escribe dentro de un archivo en base al inod pasado
    fn write(&mut self, _req: &Request, inod: u64, _fh: u64, offset: i64, data: &[u8], _flags: u32, reply: ReplyWrite) {

        let inode = self.disk.getMutInode(inod);
        let content: Vec<u8> = data.to_vec();
        
        match inode {
            Some(inode) => {
                inode.attributes.size = data.len() as u64;
                self.disk.writeContent(inod, content);
                println!("    FileSystem Write");

                reply.written(data.len() as u32);
            },
            None => {
                reply.error(ENOENT);
            }
        }    
    }

    //Busca el bloque de memoria asignado al inod y muestra su contenido 
    fn read(&mut self, _req: &Request, inod: u64, fh: u64, offset: i64, size: u32, reply: ReplyData) {
        let memoryBlock = self.disk.getBytesContent(inod);
        match memoryBlock {
            Some(memoryBlock) => {reply.data(memoryBlock);
                println!("    FileSystem Read");

            },
            None => {reply.error(EIO);}
        }
    }

    //Funcion para cambiar de nombre un archivo mediante el padre
    fn rename(&mut self, _req:&Request, parent:u64, name:&OsStr, _newparent: u64, newname:&OsStr, reply:ReplyEmpty) {
        let name = name.to_str().unwrap();
        let inode :Option<&Inode> = self.disk.findInodeByName(parent, name);
        match inode {
            Some(inode) => {
                let inod = inode.attributes.ino;
                let child = self.disk.get_mut_inode(inod);
                match child {
                    Some(child) => {
                        println!("    FileSystem Rename");
                        child.name = newname.to_str().unwrap().to_string();
                        reply.ok()
                    },
                    None => {println!("Error en Rename");}
                }
            },
            None => {
                reply.error(ENOENT);
            }
        }
    }

    //Busca el inode asignado al inod y devuelve sus atributos
    fn getattr(&mut self,_req: &Request, inod: u64, reply: ReplyAttr) {
        let inode = self.disk.getInode(inod);
        match inode {
            Some(inode) => {
                let timeInode = time::now().to_timespec();
                println!("    FileSystem GetATTR");

                reply.attr(&timeInode, &inode.attributes);
            },
            None => {
                reply.error(ENOENT);
            }
        }
    }

    //Literalmente, lee un directorio
    fn readdir(&mut self, _req: &Request, inod: u64, fh: u64, offset: i64, mut reply: ReplyDirectory) {
        println!("    FileSystem ReadDir");

        if inod == 1 {
            if offset == 0 {
                reply.add(1, 0, FileType::Directory, ".");
                reply.add(1, 1, FileType::Directory, "..");

            }
        }

        let inode: Option<&Inode> = self.disk.getInode(inod);
        if mem::size_of_val(&inode) == offset as usize {
            reply.ok();
            return;
        }

        match inode {
            Some(inode) => {
                let references = &inode.references;

                for inod in references {

                    if let inod = inod {
                        let inode = self.disk.getInode(*inod as u64);

                        if let Some(inodeData) = inode {
                            if inodeData.attributes.inod == 1 {
                                continue;
                            }

                            let name = &inodeData.name;
                            let offset = mem::size_of_val(&inode) as i64;
                            reply.add(inodeData.attributes.inod, offset, inodeData.attributes.kind, name);
                        }
                    }
                }

                reply.ok()
            },
            None => {
                reply.error(ENOENT);
            }
        }
    }

    //Crea un directorio y asigna un nuevo inod
    fn mkdir(&mut self, _req: &Request, parent: u64, name: &OsStr, _mode: u32, reply: ReplyEntry) {
        println!("    FileSystem mkdir");

        let inod = self.disk.newInode(); 
        let timespe = time::now().to_timespec();
        let attr = FileAttr {
            ino: ino as u64,
            size: 0,
            blocks: 1,
            atime: timespe,
            mtime: timespe,
            ctime: timespe,
            crtime: timespe,
            kind: FileType::Directory,
            perm: 0o755,
            nlink: 0,
            uid: 0,
            gid: 0,
            rdev: 0,
            flags: 0,
        };

        let name = name.to_str().unwrap().to_string();


        let inode = Inode {
            name: name,
            attributes: attr,
            references: Vec::new()
        };

        self.disk.writeInode(inode);
        self.disk.addReference(parent,inod as usize);

        reply.entry(&timespe, &attr, 0);
    }

    //Elimina un directorio en base al nombre
    fn rmdir(&mut self,_req: &Request, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        println!("    FileSystem rmdir");

        let name = name.to_str().unwrap();
        let inode = self.disk.findInodeByName(parent, name);

        match inode {
            Some(inode) => {
                let inod = inode.attributes.inod;
                self.disk.clearReference(parent, inod as usize);
                self.disk.removeInode(inod);

                reply.ok();
            },
            None => reply.error(EIO) 
        }
    }

    //Devuelve las estadistcas del filesystem *no funciona bien XD
    fn statfs(&mut self, _req: &Request, _ino: u64, reply: ReplyStatfs) {
        println!("    FileSystem STATFS");

        let mut blocks:u64 = 0;
        let mut files:u64 = self.disk.superBlock.len().try_into().unwrap();
        let mut bsize:u32 = 0;
        let mut namelen:u32 = 0;
    
        for i in 0..self.disk.superBlock.len() {
            blocks += self.disk.superBlock[i].attributes.blocks as u64;
            bsize += self.disk.superBlock[i].attributes.size as u32;
            namelen += self.disk.superBlock[i].name.len() as u32;
        }
        reply.statfs(blocks,0,0,files,2222 as u64,bsize,namelen,0);
    }

    //Si datasync != 0, solo se deben vaciar los datos del usuario, no los metadatos.
    fn fsync(&mut self, _req: &Request, inod: u64, fh: u64, datasync: bool, reply: ReplyEmpty) { 
        println!("    FileSystem fsync");

        reply.error(ENOSYS);
    }

    //Revisa el acceso de los permisos
    fn access(&mut self, _req: &Request, _ino: u64, _mask: u32, reply: ReplyEmpty) {
        println!("    FileSystem access ok");

        reply.ok();
    }
}

//                            ---ACA EMPIEZA EL CODIGO DE SALVAR EL DISCO Y QR                            --

//Transforma el disco a bits
pub fn encode(object: &Disk) -> Vec<u8> {
    let enc = bincode::serialize(object).unwrap();
    return enc;
}

//Decodifica un arreglo de bits y devuelve un Disk
pub fn decode(object: Vec<u8>) -> Disk {
    let decoded: Disk = bincode::deserialize(&object[..]).unwrap();
    return decoded;
}

//Guarda un arreglo de bits a una imagen de codigo QR
pub fn saveQR(encodeDisk:Vec<u8>) {
    let code = QrCode::new(encodeDisk).unwrap();

    // Render the bits into an image.
    let image = code.render::<Luma<u8>>().build();

    // Save the image.
    image.save("Storage/discoQR.png").unwrap();
}

pub fn pathValidate(path:String) -> bool{
    let imagen = image::open(path);
    match imagen {
        Ok(imagen) => {
            return true;
        },
        Err(imagen) => {
            return false;
        }
    }
}

pub fn loadFS(path : String) -> Option<Disk>{
    // Carga la base pasada por parametro
    let imagen = image::open("Storage/discoQR.png").unwrap();
    let grayImage = imagen.into_luma(); //La pasa a grises

    //Crea el decodificador
    let mut decoder = quircs::Quirc::default();

    // Busca todos los codigos qr
    let codes = decoder.identify(grayImage.width() as usize, grayImage.height() as usize, &grayImage);
    let mut vectorDecode: Option<Vec<u8>> = None;
    for code in codes {
        let code = code.expect("    FileSystem Error extrayendo QR");
        let decoded = code.decode().expect("    FileSystem Error Decodicar");
        vectorDecode = Some(decoded.payload);
    }
    match vectorDecode {
        Some(vectorDecode) => {
            let loadDisk:Disk = decode(vectorDecode);
            //Aca se carga el disc al fs
            println!("    FileSystem Disco Cargado");
            return Some(loadDisk);
        },
        None => {
            println!("    Error cargando Disk");
            return None;
        }
    }
}