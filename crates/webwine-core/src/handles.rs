use std::collections::HashMap;

// Magic Windows handle constants (as u32 two's-complement)
pub const STD_INPUT_HANDLE:  u32 = 0xFFFF_FFF6; // (DWORD)(-10)
pub const STD_OUTPUT_HANDLE: u32 = 0xFFFF_FFF5; // (DWORD)(-11)
pub const STD_ERROR_HANDLE:  u32 = 0xFFFF_FFF4; // (DWORD)(-12)

pub const INVALID_HANDLE: u32 = 0xFFFF_FFFF;

#[derive(Debug, Clone)]
pub enum KernelObject {
    ConsoleInput(u32),   // pid
    ConsoleOutput(u32),  // pid
    ConsoleError(u32),   // pid
    VfsFile { path: String, cursor: u64, writable: bool },
}

pub struct HandleTable {
    objects: HashMap<u32, KernelObject>,
    next_handle: u32,
}

impl HandleTable {
    pub fn new(pid: u32) -> Self {
        let mut t = HandleTable {
            objects: HashMap::new(),
            next_handle: 0x10,
        };
        t.objects.insert(STD_INPUT_HANDLE,  KernelObject::ConsoleInput(pid));
        t.objects.insert(STD_OUTPUT_HANDLE, KernelObject::ConsoleOutput(pid));
        t.objects.insert(STD_ERROR_HANDLE,  KernelObject::ConsoleError(pid));
        t
    }

    pub fn insert(&mut self, obj: KernelObject) -> u32 {
        let h = self.next_handle;
        self.next_handle += 4;
        self.objects.insert(h, obj);
        h
    }

    pub fn get(&self, handle: u32) -> Option<&KernelObject> {
        self.objects.get(&handle)
    }

    pub fn get_mut(&mut self, handle: u32) -> Option<&mut KernelObject> {
        self.objects.get_mut(&handle)
    }

    pub fn remove(&mut self, handle: u32) -> Option<KernelObject> {
        self.objects.remove(&handle)
    }
}
