use fuse::{Filesystem, Request, ReplyCreate, ReplyEmpty, ReplyAttr, ReplyEntry, ReplyOpen, ReplyStatfs,ReplyData, ReplyDirectory, ReplyWrite, FileType, FileAttr};
use libc::{ENOSYS, ENOENT, EIO, EISDIR, ENOSPC};
use std::ffi::OsStr;
use std::mem;
use crate::mountFS;
use serde::{Serialize, Deserialize};
use crate::ses_infor::FileAttrDef;
use qrcode::QrCode;
use image::Luma;

//---------------------------------------CODIGO DEL ALMACENAJE DE NUESTRO FS---------------------------------------


//Creamos la estructura Inode
//Con esta structura nos moveremos por el sistema
#[derive(Serialize, Deserialize)]
pub struct Inode {
    pub name: String,
    #[serde(with = "FileAttrDef")]
    pub attributes : FileAttr,
    pub references: Vec<usize>
}

impl Inode {
    //Agrega una referencia a si mismo
    pub fn add_reference(&mut self,ref_value: usize) {
        self.references.push(ref_value);
    }
    //Elimina una referencia a si mismo
    pub fn delete_reference(&mut self,ref_value: usize) {
        self.references.retain(|i| *i != ref_value);
    }

    pub fn change_name(&mut self,value: String) {
        self.name = value;
    }
}


//Se guarda el contenido de cada iNode creado
#[derive(Serialize, Deserialize)]
pub struct Mem_block {
    ino_ref : u64,
    data : Vec<u8>
}
impl Mem_block {
    //Agrega una referencia a si mismo
    pub fn add_data(&mut self,data: u8) {
        self.data.push(data);
    }
    //Elimina una referencia a si mismo
    pub fn delete_data(&mut self,data: u8) {
        self.data.retain(|i| *i != data);
    }
}
//Creamos una estructura para guardar nuestros archivos Inodes
//El super bloque contiene los inodes del sistema
//tambien la memoria de cada inote
#[derive(Serialize, Deserialize)]//Con esto podemos guardar el so
pub struct Disk {
    NEXT_INO: u64,
    super_block : Vec<Inode>,
    memory_block : Vec<Mem_block>,
    pub root_path: String
}
impl Disk {
    //Crea un nuevo disco y crea el inode raiz
    pub fn new(path:String) -> Disk{
        unsafe{
            let mut mem_block = Vec::new();
            let mut blocks = Vec::new(); //Aca guardamos los inodes
            let ts = time::now().to_timespec();
            let attr = FileAttr {
                ino: 1,
                size: 0,
                blocks: 0,
                atime: ts,
                mtime: ts,
                ctime: ts,
                crtime: ts,
                kind: FileType::Directory,
                perm: 0o755,
                nlink: 0,
                uid: 0,
                gid: 0,
                rdev: 0,
                flags: 0,
            };
            let name = ".";
            let initial_node = Inode {
                name : name.to_string(),
                attributes : attr,
                references : Vec::new()
            };
            
            blocks.push(initial_node);
            Disk {
                NEXT_INO : 1 as u64,
                super_block : blocks,
                memory_block : mem_block,
                root_path :  path
            }
        }
        
    }
    //Retorna el siguiente ino disponible
    pub fn new_ino(&mut self) -> u64{
        unsafe{
            self.NEXT_INO = self.NEXT_INO +1;
            return self.NEXT_INO;
        }
        
    }
    //Agrega el inode al super bloque
    pub fn write_ino(&mut self, inode:Inode) {
        self.super_block.push(inode);
    }
    //Elimina el inode disponible
    pub fn remove_inode(&mut self, inode:u64) {
        self.super_block.retain(|i| i.attributes.ino != inode);
    }
    //Elimina una referencia de un respectivo inode
    pub fn clear_reference(&mut self, ino: u64, ref_value: usize) {
        for i in 0..self.super_block.len() {
            if self.super_block[i].attributes.ino == ino {
                self.super_block[i].delete_reference(ref_value);
            }
         }
    }
    //Agrega una respectiva referencia a un inode
    pub fn add_reference(&mut self, ino: u64, ref_value: usize) {
        for i in 0..self.super_block.len() {
            if self.super_block[i].attributes.ino == ino {
                self.super_block[i].add_reference(ref_value);
            }
         }
    }
     //Obtiene un Inode o nada
    pub fn get_inode(&self, ino: u64) -> Option<&Inode> {
        for i in 0..self.super_block.len() {
            if self.super_block[i].attributes.ino == ino {
                return Some(&self.super_block[i]);
            }

         }
         return None;
    }
    //Obtiene un Inode mutable o nada
    pub fn get_mut_inode(&mut self, ino: u64) -> Option<&mut Inode> {
        for i in 0..self.super_block.len() {
            if self.super_block[i].attributes.ino == ino {
                return Some(&mut self.super_block[i]);
            }

         }
         return None;
    }
    //Busca en base a la carpeta del padre el hijo que tenga el nombre por parametro
    pub fn find_inode_in_references_by_name(&self, parent_inode_ino: u64, name: &str) -> Option<&Inode> {
        for i in 0..self.super_block.len() {
           if self.super_block[i].attributes.ino == parent_inode_ino {
            let parent =  &self.super_block[i];
            for j in 0..parent.references.len() {
                for k in 0..self.super_block.len() {
                    if self.super_block[k].attributes.ino == parent.references[j].try_into().unwrap() {
                        let child =  &self.super_block[k];
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
    //Agrega data al bloque de memoria asociado al ino
    pub fn add_data_to_inode(&mut self, ino:u64,data:u8) {
        for i in 0..self.memory_block.len() {
            if self.memory_block[i].ino_ref == ino {
                self.memory_block[i].add_data(data) ;
            }
        }
    }

    //Elimina la data el bloque de memoria asociado al ino
    pub fn delete_data_to_inode(&mut self, ino:u64,data: u8) {
        for i in 0..self.memory_block.len() {
            if self.memory_block[i].ino_ref == ino {
                self.memory_block[i].delete_data(data);
            }
        }
    }

    //Escribe un arreglo de bites dentro de un inode 
    pub fn write_content(&mut self, ino_ref: u64, content: Vec<u8>) {
        for i in 0..content.len(){
            self.add_data_to_inode(ino_ref, content[i]);

        }
    }

    //Obtiene el contenido de un arreglo 
    pub fn get_bytes_content(&self, ino: u64) -> Option<&[u8]> {
        for i in 0..self.memory_block.len() {
            if self.memory_block[i].ino_ref == ino {
                let bytes = &self.memory_block[i].data[..];
                return Some(bytes);
            }
        }
        return None;
    }
}



//-----------------------------------------ACA INICIA EL CODIGO DEL FILESYSTEM-------------------------------------


//Nuestro fs tiene un disco
pub struct Rb_fs {
    disk : Disk
}
impl Rb_fs {
    pub fn new(root_path:String) -> Self{
        //Falta verificar si hay que agregar crear un nuevo disco o cargarlo, las funciones ya estan
        let new_disk = Disk::new(root_path.to_string());
        Rb_fs {
            disk : new_disk
        }
    }

    pub fn get_disk(&self) -> &Disk {
        return &self.disk;
    }

    pub fn set_disk(&mut self,new_disk:Disk) {
        self.disk = new_disk;
    }

    pub fn save_fs(&self){
        let encode_fs = encode(&self.disk);
        save_to_qr(encode_fs);
    }
    pub fn load_fs(&mut self, path : String) {
        // Carga la base pasada por parametro
        let img = image::open("disk_memories/qrdiskcode.png").unwrap();
        let img_gray = img.into_luma(); //La pasa a grises

        //Crea el decodificador
        let mut decoder = quircs::Quirc::default();

        // Busca todos los codigos qr
        let codes = decoder.identify(img_gray.width() as usize, img_gray.height() as usize, &img_gray);
        let mut vec_decode: Option<Vec<u8>> = None;
        for code in codes {
            let code = code.expect("----RB-FS ERROR AL EXTRAER QR-----------");
            let decoded = code.decode().expect("----RB-FS ERROR AL DECODIFICAR-------");
            vec_decode = Some(decoded.payload);
        }
        match vec_decode {
            Some(vec_decode) => {
                let disk_to_load:mkfs::Disk = mkfs::decode(vec_decode);
                //Aca se carga el disc al fs
                println!("----RB-FS DISCO CARGADO---------");
            },
            None => {
                println!("------- ERROR AL CARGAR EL DISCO --------");
            }
        }
    }
}

impl Drop for Rb_fs {
    fn drop(&mut self) {
        &self.save_fs();
        println!("---RB-FS SAVED---!");
    }
}

impl Filesystem for Rb_fs {

    //Mira dentro de un directorio por su nombre y obtiene sus atributos
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {

        let fila_name = name.to_str().unwrap();
        let inode = self.disk.find_inode_in_references_by_name(parent, fila_name);
        match inode {
            Some(inode) => {
                let ttl = time::now().to_timespec();
                reply.entry(&ttl, &inode.attributes, 0);
                println!("----RB-FS: LOOKUP----");
            },
            None => {
                reply.error(ENOENT);
            }
        }
    }
    //Crea un archivo en la padre pasado poor parametro
    fn create(&mut self, _req: &Request, parent: u64, name: &OsStr, mode: u32, flags: u32, reply: ReplyCreate) {

        let ino_available = self.disk.new_ino();
        let mem_block = Mem_block {
            ino_ref : ino_available,
            data : Vec::new()
        };

        let ts = time::now().to_timespec();

        let attr = FileAttr {
            ino: ino_available,
            size: 0,
            blocks: 1,
            atime: ts,
            mtime: ts,
            ctime: ts,
            crtime: ts,
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

        inode.references.push(mem_block.ino_ref as usize);

        self.disk.write_ino(inode);
        
        self.disk.add_reference(parent, ino_available as usize);
        self.disk.memory_block.push(mem_block);
        println!("----RB-FS: CREATED----");

        reply.created(&ts, &attr, 1, ino_available, flags)
    }

    //Escribe dentro de un archivo en base al ino pasado
    fn write(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64, data: &[u8], _flags: u32, reply: ReplyWrite) {

        let inode = self.disk.get_mut_inode(ino);
        let content: Vec<u8> = data.to_vec();
        
        match inode {
            Some(inode) => {
                inode.attributes.size = data.len() as u64;
                self.disk.write_content(ino, content);
                println!("----RB-FS: WRITE----");

                reply.written(data.len() as u32);
            },
            None => {
                reply.error(ENOENT);
            }
        }    
    }
    //Busca el bloque de memoria asignado al ino y muestra su contenido 
    fn read(&mut self, _req: &Request, ino: u64, fh: u64, offset: i64, size: u32, reply: ReplyData) {
        let memory_block = self.disk.get_bytes_content(ino);
        match memory_block {
            Some(memory_block) => {reply.data(memory_block);
                println!("----RB-FS: READ----");

            },
            None => {reply.error(EIO);}
        }
    }
    //Busca el inode asignado al ino y devuelve sus atributos
    fn getattr(&mut self,_req: &Request, ino: u64, reply: ReplyAttr) {
        let inode = self.disk.get_inode(ino);
        match inode {
            Some(inode) => {
                let ttl = time::now().to_timespec();
                println!("----RB-FS: GETATTR----");

                reply.attr(&ttl, &inode.attributes);
            },
            None => reply.error(ENOENT)
        }
    }
    //Literalmente, lee un directorio
    fn readdir(&mut self, _req: &Request, ino: u64, fh: u64, offset: i64, mut reply: ReplyDirectory) {
        println!("----RB-FS: READDIR----");

        if ino == 1 {
            if offset == 0 {
                reply.add(1, 0, FileType::Directory, ".");
                reply.add(1, 1, FileType::Directory, "..");

            }
        }

        let inode: Option<&Inode> = self.disk.get_inode(ino);
        if mem::size_of_val(&inode) == offset as usize {
            reply.ok();
            return;
        }

        match inode {
            Some(inode) => {
                let references = &inode.references;

                for ino in references {

                    if let ino = ino {
                        let inode = self.disk.get_inode(*ino as u64);

                        if let Some(inode_data) = inode {
                            if inode_data.attributes.ino == 1 {
                                continue;
                            }

                            let name = &inode_data.name;
                            let offset = mem::size_of_val(&inode) as i64;
                            reply.add(inode_data.attributes.ino, offset, inode_data.attributes.kind, name);
                        }
                    }
                }

                reply.ok()
            },
            None => { println!("ERROR ino={:?}", ino); reply.error(ENOENT) }
        }
    }

    //Crea un directorio y asigna un nuevo ino
    fn mkdir(&mut self, _req: &Request, parent: u64, name: &OsStr, _mode: u32, reply: ReplyEntry) {
        println!("----RB-FS: MKDIR----");

        let ino = self.disk.new_ino(); 
        let ts = time::now().to_timespec();
        let attr = FileAttr {
            ino: ino as u64,
            size: 0,
            blocks: 1,
            atime: ts,
            mtime: ts,
            ctime: ts,
            crtime: ts,
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

        self.disk.write_ino(inode);
        self.disk.add_reference(parent,ino as usize);

        reply.entry(&ts, &attr, 0);
    }
    //Elimina un directorio en base al nombre
    fn rmdir(&mut self,_req: &Request, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        println!("----RB-FS: RMDIR----");

        let name = name.to_str().unwrap();
        let inode = self.disk.find_inode_in_references_by_name(parent, name);

        match inode {
            Some(inode) => {
                let ino = inode.attributes.ino;
                self.disk.clear_reference(parent, ino as usize);
                self.disk.remove_inode(ino);

                reply.ok();
            },
            None => reply.error(EIO) 
        }
    }
    //Devuelve las estadistcas del filesystem *no funciona bien XD
    fn statfs(&mut self, _req: &Request, _ino: u64, reply: ReplyStatfs) {
        println!("----RB-FS: STATFS----");

        let mut blocks:u64 = 0;
        let mut files:u64 = self.disk.super_block.len().try_into().unwrap();
        let mut bsize:u32 = 0;
        let mut namelen:u32 = 0;
    
        for i in 0..self.disk.super_block.len() {
            blocks += self.disk.super_block[i].attributes.blocks as u64;
            bsize += self.disk.super_block[i].attributes.size as u32;
            namelen += self.disk.super_block[i].name.len() as u32;
        }
        reply.statfs(blocks,0,0,files,2222 as u64,bsize,namelen,0);
    }

    //Si datasync != 0, solo se deben vaciar los datos del usuario, no los metadatos.
    fn fsync(&mut self, _req: &Request, ino: u64, fh: u64, datasync: bool, reply: ReplyEmpty) { 
        println!("----RB-FS: FSYNC----");

        reply.error(ENOSYS);
    }

    //Revisa el acceso de los permisos
    fn access(&mut self, _req: &Request, _ino: u64, _mask: u32, reply: ReplyEmpty) {
        println!("----RB-FS: ACCESS----");

        reply.ok();
    }
    


}

//-------------------------------ACA EMPIEZA EL CODIGO DE SALVAR EL DISCO Y QR------------------------------

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
pub fn save_to_qr(encode_disk:Vec<u8>) {
    let code = QrCode::new(encode_disk).unwrap();

    // Render the bits into an image.
    let image = code.render::<Luma<u8>>().build();

    // Save the image.
    image.save("disk_memories/qrdiskcode.png").unwrap();
}