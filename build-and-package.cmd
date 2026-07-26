# 一键打包脚本 - 复制并执行以下命令

cd C:\Users\YLW\Documents\PJ\WinSLA

# 1. 编译 DLL（如果还没编译过）
cargo build --release --package cp_provider

# 2. 创建部署包（会自动生成 ZIP）
.\scripts\create-package.ps1 -CreateZip

# 3. 找到 ZIP 文件并拷贝到测试 VM
dir C:\Temp\*.zip

# 完成后，将生成的 ZIP 文件传输到测试 VM 即可使用
