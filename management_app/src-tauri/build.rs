fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        let mut res = winres::WindowsResource::new();
        res.set_icon("../../assets/winsla.ico");
        res.set("ProductName", "WinSLA Management");
        res.set("FileDescription", "WinSLA Dual-Account Authentication Management");
        res.set("LegalCopyright", "MIT License - WinSLA Contributors");
        res.compile().expect("Failed to compile Windows resources");
    }
}
