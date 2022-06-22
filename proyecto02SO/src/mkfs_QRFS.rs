use fuse::{Filesystem, Request, ReplyCreate, ReplyEmpty, ReplyAttr, ReplyEntry, ReplyOpen, ReplyStatfs,ReplyData, ReplyDirectory, ReplyWrite, FileType, FileAttr};
use libc::{ENOSYS, ENOENT, EIO, EISDIR, ENOSPC};
use std::ffi::OsStr;
use std::mem;
use crate::mkfs_QRFS;
use serde::{Serialize, Deserialize};
use crate::sesInformation::FileAttrDef;
use qrcode::QrCode;
use image::Luma;

//Los Inodes son la unidad que movera nuestro fs
/**
 * Estructura Inode
 * Posee el nombre, atributos y referencias a otros Inode
 * Lo utilizamos para movernos por nuestro FS
 */
#[derive(Serialize, Deserialize)]
pub struct Inode {
    pub name: String,
    #[serde(with = "FileAttrDef")]
    pub attributes : FileAttr,
    pub references: Vec<usize>
}

impl Inode {
    /**
     * changeNamme
     * Recibe el nuevo nombre
     * reemplaza el nombre del Inode por el nuevo
     * No retorna nada
     */
    pub fn changeName(&mut self,value: String) {
        self.name = value;
    }

    /**
     * addReference
     * Recibe una referencia
     * Añade una referencia de el mismo
     * No retorna nada
     */
    pub fn addReference(&mut self,refValue: usize) {
        self.references.push(refValue);
    }
    /**
     * deleteReference
     * Recibe una referencia
     * Elimina dicha referencia
     * No retorna nada
     */
    pub fn deleteReference(&mut self,refValue: usize) {
        self.references.retain(|i| *i != refValue);
    }
}

/**
 * estructura memBlock
 * Guarda los Inode y sus datos
 * No retorna nada
 */
#[derive(Serialize, Deserialize)]
pub struct memBlock {
    referenceInode : u64,
    data : Vec<u8>
}

impl memBlock {
    //Agrega una referencia a si mismo
    /**
     * addData
     * Recibe datos
     * Añade estos mismos datos (Una referencia de si mismo)
     * No retorna nada
     */
    pub fn addData(&mut self,data: u8) {
        self.data.push(data);
    }
    //Elimina una referencia a si mismo
    /**
     * deleteData
     * Recibe un dato
     * Elimina el dato
     * No retorna nada
     */
    pub fn deleteData(&mut self,data: u8) {
        self.data.retain(|i| *i != data);
    }
}

//Creamos una estructura para guardar nuestros archivos Inodes
//El super bloque contiene los inodes del sistema
//tambien la memoria de cada inote
/**
 * Estructura Disk
 * Posee el siguiente Inode, el superBlock, el block de memoria, y su ruta
 * Se utiliza para guardar los datos de nuevo FS
 * No retorna nada
 */
#[derive(Serialize, Deserialize)]
pub struct Disk {
    sigInode: u64,
    pub superBlock : Vec<Inode>,
    pub memoryBlock : Vec<memBlock>,
    pub rootPath: String
}

impl Disk {
    /**
     * new
     * Recibe un path de montaje y un path del disco
     * Crea un disco y su Inode raiz o en caso de existir ya un disco este lo carga
     * Retorna el nuevo disco o si ya existe uno entonces este lo carga
     */
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

    /**
     * returnNextInode
     * No recibe nada
     * Regresa el siguiente Inode
     */
    pub fn returnNextInode(&mut self) -> u64{
        unsafe{
            self.sigInode = self.sigInode +1;
            return self.sigInode;
        }
        
    }

    /**
     * writeInode
     * Recibbe un Inode
     * Agrega el inode al super bloque
     * No retorna nada
     */
    pub fn writeInode(&mut self, inode:Inode) {
        self.superBlock.push(inode);
    }

    /**
     * removeInode
     * Recibe Inode
     * Elimina el Inode del super bloque
     * No retorna nada
     */
    pub fn removeInode(&mut self, inode:u64) {
        self.superBlock.retain(|i| i.attributes.ino != inode);
    }

    /**
     * clearReference
     * Recibe un Inode y una referencia
     * Elimina la referencia con respecto al Inode
     * No regresa nada
     */
    pub fn clearReference(&mut self, ino: u64, refValue: usize) {
        for i in 0..self.superBlock.len() {
            if self.superBlock[i].attributes.ino == ino {
                self.superBlock[i].deleteReference(refValue);
            }
         }
    }

    /**
     * clearReference
     * Recibe un Inode y una referencia
     * Añade la referencia con respecto al Inode
     * No regresa nada
     */
    pub fn addReference(&mut self, ino: u64, refValue: usize) {
        for i in 0..self.superBlock.len() {
            if self.superBlock[i].attributes.ino == ino {
                self.superBlock[i].addReference(refValue);
            }
         }
    }

     /**
     * getInode
     * Recibe un numero de identificacion del Inode
     * Regresa el Inode, en caso de no encontrarlo regresa None
     */
    pub fn getInode(&self, ino: u64) -> Option<&Inode> {
        for i in 0..self.superBlock.len() {
            if self.superBlock[i].attributes.ino == ino {
                return Some(&self.superBlock[i]);
            }

         }
         return None;
    }

    /**
     * getInode
     * Recibe un numero de identificacion del Inode
     * Regresa el Inode Mutable, en caso de no encontrarlo regresa None
     */
    pub fn getMutInode(&mut self, ino: u64) -> Option<&mut Inode> {
        for i in 0..self.superBlock.len() {
            if self.superBlock[i].attributes.ino == ino {
                return Some(&mut self.superBlock[i]);
            }

         }
         return None;
    }

    /**
     * findeInodeByName
     * Recibe un numero de identificacion del padre y un nombre
     * Regresa el Inode al que corresponde el nombre y el padre
     */
    pub fn findInodeByName(&self, parentInode: u64, name: &str) -> Option<&Inode> {
        for i in 0..self.superBlock.len() {
           if self.superBlock[i].attributes.ino == parentInode {
            let parent =  &self.superBlock[i];
            for j in 0..parent.references.len() {
                for k in 0..self.superBlock.len() {
                    if self.superBlock[k].attributes.ino == parent.references[j].try_into().unwrap() {
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

    /**
     * addDataInode
     * Recibe el numero de asocie del Inode y los datos
     * Añade los datos al bloque de memoria asociado al Inode
     * No retorna nada
     */
    pub fn addDataInode(&mut self, ino:u64,data:u8) {
        for i in 0..self.memoryBlock.len() {
            if self.memoryBlock[i].referenceInode == ino {
                self.memoryBlock[i].addData(data) ;
            }
        }
    }

    /**
     * writeContent
     * Recibe una referencia y contenido
     * Añade el contenido y la referencia a los datos del Inode
     * No retorna nada
     */
    pub fn writeContent(&mut self, referenceInode: u64, content: Vec<u8>) {
        for i in 0..content.len(){
            self.addDataInode(referenceInode, content[i]);

        }
    }

     /**
     * writeContent
     * Recibe el numero del Inode
     * Elimina los datos del bloque de memoria asociados al Inode
     * No retorna nada
     */
    pub fn deleteDataInode(&mut self, ino:u64,data: u8) {
        for i in 0..self.memoryBlock.len() {
            if self.memoryBlock[i].referenceInode == ino {
                self.memoryBlock[i].deleteData(data);
            }
        }
    }

    /**
     * getBytesContent
     * Recibe el numero del Inode
     * Retorna el contenido en un arreglo binario
     */
    pub fn getBytesContent(&self, ino: u64) -> Option<&[u8]> {
        for i in 0..self.memoryBlock.len() {
            if self.memoryBlock[i].referenceInode == ino {
                let bytes = &self.memoryBlock[i].data[..];
                return Some(bytes);
            }
        }
        return None;
    }
}


/**
  * FILESYSTEM
 * Estructura jr_fs
 * Es nuestro FS, solo posee un disco
 */
pub struct jr_fs {
    disk : Disk
}

impl jr_fs {
     /**
     * new
     * Recibe el path del montaje y del disco
     * Crea una estructura de jr_fs creando un disco con los parametros
     * No retorna nada
     */
    pub fn new(rootPath:String, diskPath:String) -> Self{
        let newDisk = Disk::new(rootPath.to_string(), diskPath);
        jr_fs {
            disk : newDisk
        }
    }

     /**
     * getDisk
     * No recibe nada
     * Retorna el disco
     */
    pub fn getDisk(&self) -> &Disk {
        return &self.disk;
    }

     /**
     * setDisk
     * Recibe un disco
     * Modifica el disco actual por el del patrametro
     * No retorna nada
     */
    pub fn setDisk(&mut self,newDisk:Disk) {
        self.disk = newDisk;
    }

     /**
     * writeContent
     * Recibe el numero del Inode
     * Guarda como un QR el disco
     * No retorna nada
     */
    pub fn saveFileSystem(&self){
        let encodeFS = encode(&self.disk);
        saveQR(encodeFS);
    }
}

 /**
     * Drop
     * No recibe nada
     * Se encarga de guardar el sistema de archivos
     * No retorna nada
     */
impl Drop for jr_fs{
    fn drop(&mut self) {
        &self.saveFileSystem();
    }
}

impl Filesystem for jr_fs {

    //Mira dentro de un directorio por su nombre y obtiene sus atributos
     /**
     * lookup
     * Recibe el request, el padre, el nombre, y la respuesta
     * Se encarga de revisar el directorio con el nombre y obtener sus atributos
     * No retorna nada, pero si modifica la respuesta para que fuse puede utilizarla
     */
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

     /**
     * create
     * Recibe el request, el padre, el nombre, el modo, las banderas y la respuesta
     * Se encarga de crear un nuevo Inode, agregarlo a la memoria y al disco el nuevo Inode y todos sus datos
     * No retorna nada
     */
    fn create(&mut self, _req: &Request, parent: u64, name: &OsStr, mode: u32, flags: u32, reply: ReplyCreate) {

        let availableInode = self.disk.returnNextInode();
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

     /**
     * write
     * Recibe el request, el Ino, fh, offset, data, banderas y la respuesta
     * Escribe dentro de un archivo basandose en el ino anterior
     * No retorna nada
     */
    fn write(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64, data: &[u8], _flags: u32, reply: ReplyWrite) {

        let inode = self.disk.getMutInode(ino);
        let content: Vec<u8> = data.to_vec();
        
        match inode {
            Some(inode) => {
                inode.attributes.size = data.len() as u64;
                self.disk.writeContent(ino, content);
                println!("    FileSystem Write");

                reply.written(data.len() as u32);
            },
            None => {
                reply.error(ENOENT);
            }
        }    
    }

    /**
     * write
     * Recibe el request, el Ino, fh, offset, size y la respuesta
     * Busca en el bloque de memoria y envia su contenido por el reply
     * No retorna nada
     */
    fn read(&mut self, _req: &Request, ino: u64, fh: u64, offset: i64, size: u32, reply: ReplyData) {
        let memoryBlock = self.disk.getBytesContent(ino);
        match memoryBlock {
            Some(memoryBlock) => {reply.data(memoryBlock);
                println!("    FileSystem Read");

            },
            None => {reply.error(EIO);}
        }
    }

    /**
     * rename
     * Recibe el request, el padre, el nombre, nuevopadre, nuevo nombre y la respuesta
     * Obtiene el Ino por medio del nombre y su padre y modifica en este el nombre
     * No retorna nada
     */
    fn rename(&mut self, _req:&Request, parent:u64, name:&OsStr, _newparent: u64, newname:&OsStr, reply:ReplyEmpty) {
        let name = name.to_str().unwrap();
        let inode :Option<&Inode> = self.disk.findInodeByName(parent, name);
        match inode {
            Some(inode) => {
                let ino = inode.attributes.ino;
                let child = self.disk.getMutInode(ino);
                match child {
                    Some(child) => {
                        println!("    FileSystem Rename");
                        child.name = newname.to_str().unwrap().to_string();
                        reply.ok()
                    }
                    None => {
                        println!("No se pudo renombrar el FileSystem");
                    }
                }
            },
            None => {
                reply.error(ENOENT);
            }
        }
    }

    /**
     * getattr
     * Recibe el request, el Ino y la respuesta
     * BBusca al inode asignado al ino y envia como respuesta sus atributos
     * No retorna nada
     */
    fn getattr(&mut self,_req: &Request, ino: u64, reply: ReplyAttr) {
        let inode = self.disk.getInode(ino);
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

    //lee un directorio
    /**
     * readdir
     * Recibe el request, el Ino, fh, offset y la respuesta
     * Se encarga de leer un directorio
     * No retorna nada
     */
    fn readdir(&mut self, _req: &Request, ino: u64, fh: u64, offset: i64, mut reply: ReplyDirectory) {
        println!("    FileSystem ReadDir");

        if ino == 1 {
            if offset == 0 {
                reply.add(1, 0, FileType::Directory, ".");
                reply.add(1, 1, FileType::Directory, "..");

            }
        }

        let inode: Option<&Inode> = self.disk.getInode(ino);
        if mem::size_of_val(&inode) == offset as usize {
            reply.ok();
            return;
        }

        match inode {
            Some(inode) => {
                let references = &inode.references;

                for ino in references {

                    if let ino = ino {
                        let inode = self.disk.getInode(*ino as u64);

                        if let Some(inodeData) = inode {
                            if inodeData.attributes.ino == 1 {
                                continue;
                            }

                            let name = &inodeData.name;
                            let offset = mem::size_of_val(&inode) as i64;
                            reply.add(inodeData.attributes.ino, offset, inodeData.attributes.kind, name);
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

    /**
     * opendir
     * Recibe el request, el Ino, banderas y la respuesta
     * Abre un directorio
     * No retorna nada
     */
    fn opendir(&mut self, _req: &Request, _inod: u64, _flags: u32, reply: ReplyOpen) { 
        let dir = self.disk.getInode(_inod);
        match dir {
            Some(dir) => {
                println!("    FileSystem Opendir");
                reply.opened(dir.attributes.ino, 1 as u32);
            },
            None => {
                println!("No se pudo abrir")
            }
        }

    }

    //Crea un directorio y asigna un nuevo ino
    /**
     * write
     * Recibe el request, el parent, name, mode y la respuesta
     * Crea un directorio y asigna el nuevo ino
     * No retorna nada
     */
    fn mkdir(&mut self, _req: &Request, parent: u64, name: &OsStr, _mode: u32, reply: ReplyEntry) {
        println!("    FileSystem mkdir");

        let ino = self.disk.returnNextInode(); 
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
        self.disk.addReference(parent,ino as usize);

        reply.entry(&timespe, &attr, 0);
    }

    /**
     * rmdir
     * Recibe el request, el parent, name y la respuesta
     * Elimina un directorio basado en su nombre y padre
     * No retorna nada
     */
    fn rmdir(&mut self,_req: &Request, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        println!("    FileSystem rmdir");

        let name = name.to_str().unwrap();
        let inode = self.disk.findInodeByName(parent, name);

        match inode {
            Some(inode) => {
                let ino = inode.attributes.ino;
                self.disk.clearReference(parent, ino as usize);
                self.disk.removeInode(ino);

                reply.ok();
            },
            None => reply.error(EIO) 
        }
    }

    /**
     * statfs
     * Recibe el request, el Ino y la respuesta
     * Devuelve la estructura del FS
     * No retorna nada
     */
    fn statfs(&mut self, _req: &Request, _ino: u64, reply: ReplyStatfs) {
        println!("    FileSystem STATFS");

        let mut blocks:u64 =  (self.disk.superBlock.len() +self.disk.memoryBlock.len()) as u64;
        let mut blockfree:u64 = blocks - self.disk.memoryBlock.len() as u64;
        let mut bavail:u64 = blockfree;
        let mut files:u64 = self.disk.memoryBlock.len().try_into().unwrap();
        let mut filefree:u64 = 1024 as u64;
        let mut blocksize:u32 = (mem::size_of::<Vec<Inode>>() as u32 +mem::size_of::<Inode>() as u32)*1024;
        let mut namelen:u32 = 77;
        let mut freesize:u32 = 1;

        reply.statfs(blocks, blockfree, bavail, files, filefree, blocksize, namelen, freesize);
    }

    /**
     * fsync
     * Recibe el request, el Ino, fh, datasync y la respuesta
     * No esta implementado solo regresa error
     * No retorna nada
     */
    fn fsync(&mut self, _req: &Request, ino: u64, fh: u64, datasync: bool, reply: ReplyEmpty) {
        reply.error(ENOSYS);
    }

    /**
     * access
     * Recibe el request, el Ino, mask y la respuesta
     * Regresa un mensaje de ok al acceso
     */
    fn access(&mut self, _req: &Request, _ino: u64, _mask: u32, reply: ReplyEmpty) {
        reply.ok();
    }
}

/** QRcode
 * encode
 * Recibe un disco
 * Se encarga de serializar (codifica) el disco
 * Regresa la serialización
 */
pub fn encode(object: &Disk) -> Vec<u8> {
    let enc = bincode::serialize(object).unwrap();
    return enc;
}

/**
 * decode
 * Recibe un vector de numeros
 * Se encarga de descodificar el disco
 * Regresa el disco
 */
pub fn decode(object: Vec<u8>) -> Disk {
    let decoded: Disk = bincode::deserialize(&object[..]).unwrap();
    return decoded;
}

/**
 * saveQR
 * Recibe al disco serializado
 * Se encarga de guardar los datos en una imagen QR en el path por defecto
 * No regresa nada
 */
pub fn saveQR(encodeDisk:Vec<u8>) {
    let code = QrCode::new(encodeDisk).unwrap();

    // Render the bits into an image.
    let image = code.render::<Luma<u8>>().build();

    // Save the image.
    image.save("/home/tinky-winky/Documents/Proyecto02SO-QRFS/proyecto02SO/src/Storage/discoQR.png").unwrap();
}

/**
 * pathValidate
 * Recibe un path
 * Verifica que el path sea una imagen y que exista
 * Regresa true o false
 */
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

/**
 * loadFS
 * Recibe un path
 * Se encarga de cargar el sistema de archivos apartir del path carga la imagen y la decodifica
 * Regresa el disco en caso que sea un disco, sino regresar None
 */
pub fn loadFS(path : String) -> Option<Disk>{
    // Carga la base pasada por parametro
    let imagen = image::open("/home/tinky-winky/Documents/Proyecto02SO-QRFS/proyecto02SO/src/Storage/discoQR.png").unwrap();
    let grayImage = imagen.to_luma(); //La pasa a grises

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