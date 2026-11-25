fn main() {
    // 前端产物由 `npm run build` 生成；不声明依赖时，cargo 认为 crate 没变，
    // 会把上一次的 dist 继续嵌进 exe，出现"改了前端但窗口还是旧版"。
    println!("cargo:rerun-if-changed=../dist");
    println!("cargo:rerun-if-changed=../dist/index.html");
    println!("cargo:rerun-if-changed=../dist/assets");
    tauri_build::build();
}
