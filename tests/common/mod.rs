use std::fs;
use std::path::Path;

/// 测试辅助函数：写入文件（自动创建父目录）。
///
/// 设计目的：
/// - 测试中经常需要构造临时 TOML/JSON 文件
/// - 用一个小 helper 统一处理 create_dir_all + write，减少重复代码
pub fn write_file(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}
