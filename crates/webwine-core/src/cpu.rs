#[derive(Debug, Clone, Default)]
pub struct X86Cpu {
    pub eax: u32,
    pub ebx: u32,
    pub ecx: u32,
    pub edx: u32,
    pub esi: u32,
    pub edi: u32,
    pub ebp: u32,
    pub esp: u32,
    pub eip: u32,
    pub eflags: u32,
    // last_error mirrors GetLastError/SetLastError per-thread state
    pub last_error: u32,
}

impl X86Cpu {
    pub fn new() -> Self {
        X86Cpu {
            eflags: 0x202, // IF=1, reserved bit
            ..Default::default()
        }
    }
}
