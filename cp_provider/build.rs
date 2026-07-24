// Build script for Credential Provider
// Sets up proper linking and module definition file

fn main() {
    // Compile any C/C++ code if needed (none for now)
    
    // Set linker flags for DLL
    println!("cargo:rustc-link-lib=user32");
    println!("cargo:rustc-link-lib=ole32");
    println!("cargo:rustc-link-lib=advapi32");
    println!("cargo:rustc-link-lib=secur32");
    
    // Force full symbol resolution
    println!("cargo:rustc-link-arg=/OPT:REF");
    println!("cargo:rustc-link-arg=/OPT:ICF");
}
