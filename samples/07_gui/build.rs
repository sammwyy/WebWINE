fn main() {
    // No-CRT binary: point the PE entry at our own function and pick the
    // console subsystem. The CRT startup object is never pulled in.
    println!("cargo:rustc-link-arg=/ENTRY:mainCRTStartup");
    println!("cargo:rustc-link-arg=/SUBSYSTEM:CONSOLE");
    println!("cargo:rustc-link-arg=/NODEFAULTLIB");
    println!("cargo:rustc-link-arg=/SAFESEH:NO");
    println!("cargo:rustc-link-arg=kernel32.lib");
}
