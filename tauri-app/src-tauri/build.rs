fn main() {
    tauri_build::try_build(
        tauri_build::Attributes::default().codegen(tauri_build::CodegenContext::default()),
    )
    .expect("tauri build script failed");
}
