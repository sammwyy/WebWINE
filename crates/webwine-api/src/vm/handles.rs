use std::collections::HashMap;

pub const STD_INPUT_HANDLE:  u32 = 0xFFFF_FFF6;
pub const STD_OUTPUT_HANDLE: u32 = 0xFFFF_FFF5;
pub const STD_ERROR_HANDLE:  u32 = 0xFFFF_FFF4;
pub const INVALID_HANDLE:    u32 = 0xFFFF_FFFF;
pub const CURRENT_PROCESS:   u32 = 0xFFFF_FFFF;
pub const CURRENT_THREAD:    u32 = 0xFFFF_FFFE;

#[derive(Debug, Clone)]
pub enum KernelObject {
    ConsoleInput(u32),
    ConsoleOutput(u32),
    ConsoleError(u32),
    VfsFile { path: String, cursor: u64, writable: bool },
    /// FindFirstFile/FindNextFile enumeration state: matched (name, is_dir, size)
    /// and a cursor into them.
    FindHandle { matches: Vec<(String, bool, u64)>, cursor: usize },
}

pub struct HandleTable {
    objects: HashMap<u32, KernelObject>,
    next: u32,
}

impl HandleTable {
    pub fn new(pid: u32) -> Self {
        let mut t = HandleTable { objects: HashMap::new(), next: 0x10 };
        t.objects.insert(STD_INPUT_HANDLE,  KernelObject::ConsoleInput(pid));
        t.objects.insert(STD_OUTPUT_HANDLE, KernelObject::ConsoleOutput(pid));
        t.objects.insert(STD_ERROR_HANDLE,  KernelObject::ConsoleError(pid));
        t
    }

    pub fn insert(&mut self, obj: KernelObject) -> u32 {
        let h = self.next; self.next += 4;
        self.objects.insert(h, obj); h
    }

    pub fn get(&self, h: u32) -> Option<&KernelObject>     { self.objects.get(&h) }
    pub fn get_mut(&mut self, h: u32) -> Option<&mut KernelObject> { self.objects.get_mut(&h) }
    pub fn remove(&mut self, h: u32) -> Option<KernelObject> { self.objects.remove(&h) }
}
