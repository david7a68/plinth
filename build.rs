use std::path::{Path, PathBuf};

use windows::Win32::Graphics::Direct3D::Dxc::{
    CLSID_DxcCompiler, DxcCreateInstance, IDxcCompiler3,
};

const VS_MAIN: &str = "vs_main";
const PS_MAIN: &str = "ps_main";
const RESOURCE_DIR: &str = "./res/";

const SHADERS: &[&str] = &["rect_shader.hlsl"];

fn main() {
    println!("cargo:rerun-if-changed={}", RESOURCE_DIR);

    let mut shader_path = PathBuf::new();
    for shader in SHADERS {
        shader_path.clear();
        shader_path.push(RESOURCE_DIR);
        shader_path.push(shader);

        compile_dx12_shader(&shader_path);
    }
}

fn compile_dx12_shader(path: &Path) {
    let compiler: IDxcCompiler3 = unsafe { DxcCreateInstance(&CLSID_DxcCompiler) }.unwrap();
    // todo
}
