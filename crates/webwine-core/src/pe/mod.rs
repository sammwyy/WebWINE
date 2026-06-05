pub mod inspector;
pub mod loader;

pub use inspector::{inspect_bytes, PeImportModule, PeInfo, PeSection};
pub use loader::load_pe;

/// Parse a PE the way WebWINE needs it: like `PE::parse`, but skip the attribute
/// certificates table (the Authenticode signature appended past the logical end
/// of the file). goblin rejects an out-of-bounds cert table — common for signed
/// games like Undertale — even though the signature is irrelevant to execution.
pub fn parse_pe(bytes: &[u8]) -> goblin::error::Result<goblin::pe::PE<'_>> {
    let mut opts = goblin::pe::options::ParseOptions::default();
    opts.parse_attribute_certificates = false;
    goblin::pe::PE::parse_with_opts(bytes, &opts)
}
