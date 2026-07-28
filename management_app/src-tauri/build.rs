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
        // 使用 embed-resource 编译 winsla.rc（图标 + UAC manifest + 版本信息）
        // .res 文件作为链接器直接输入，资源段无条件包含
        // （winres 生成的纯资源 .lib 会被 MSVC 链接器丢弃，不可用）
        embed_resource::compile("winsla.rc", embed_resource::NONE);
    }
}
