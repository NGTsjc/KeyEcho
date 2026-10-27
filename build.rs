#[cfg(windows)]
fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=asset/logo.ico");
    let mut res = winres::WindowsResource::new();
    // 资源 ID 1 同时供 EXE 默认图标和系统托盘使用。
    res.set_icon_with_id("asset/logo.ico", "1");
    res.compile().unwrap();
}

#[cfg(not(windows))]
fn main() {}
