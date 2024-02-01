use std::env;

const RESOURCE_DIR: &str = "./res/";

fn main() {
    println!("cargo:rerun-if-changed={RESOURCE_DIR}");

    let out_dir = env::var("OUT_DIR").unwrap();

    compile_dx12_shaders(&out_dir);
}

#[derive(Clone, Copy)]
enum Kind {
    Vertex,
    Pixel,
}

fn compile_dx12_shaders(out_dir: &str) {
    compile_dx12_shader(
        "vs_main",
        Kind::Vertex,
        "rect_shader",
        RESOURCE_DIR,
        "rect_vs",
        out_dir,
    );

    compile_dx12_shader(
        "ps_main",
        Kind::Pixel,
        "rect_shader",
        RESOURCE_DIR,
        "rect_ps",
        out_dir,
    );
}

fn compile_dx12_shader(main: &str, kind: Kind, src: &str, src_dir: &str, out: &str, out_dir: &str) {
    let model = match kind {
        Kind::Vertex => "vs_6_0",
        Kind::Pixel => "ps_6_0",
    };

    check_output(
        &std::process::Command::new("./vendor/dxc/bin/x64/dxc.exe")
            .args(["-T", model]) // shader model
            .args(["-E", main]) // entry point
            .args(["-Fo", &format!("{out_dir}/{out}.cso")]) // output
            .args(["-Fd", &format!("{out_dir}/{out}.pdb")]) // debug output
            .arg("-")
            .arg("-WX") // warnings as errors
            .arg("-Zs") // small PDBs
            .arg("-Qstrip_reflect") // strip reflection data
            .arg(&format!("{src_dir}/{src}.hlsl")) // input
            .output()
            .expect("failed to compile rect_vs"),
    );
}

fn check_output(output: &std::process::Output) {
    if output.status.code() != Some(0) {
        println!("cargo:warn {}", String::from_utf8_lossy(&output.stderr));
    }
}
