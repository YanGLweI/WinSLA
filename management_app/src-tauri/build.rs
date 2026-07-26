fn main() {
    // Check that frontend assets exist before embedding
    let dist_dir = std::path::Path::new("frontend/dist");
    if !dist_dir.exists() {
        panic!(
            "Frontend assets not found at 'frontend/dist'. \
             Run 'npm install && npm run build' in management_app/ first."
        );
    }
    println!("cargo:rerun-if-changed=frontend/dist");

    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        let mut res = winres::WindowsResource::new();
        res.set_icon("../../assets/winsla.ico");
        res.set("ProductName", "WinSLA Management");
        res.set("FileDescription", "WinSLA Dual-Account Authentication Management");
        res.set("LegalCopyright", "MIT License - ylw");
        res.set("FileVersion", "2.0.1");
        res.set("ProductVersion", "2.0.1");
        res.compile().expect("Failed to compile Windows resources");
    }
}
