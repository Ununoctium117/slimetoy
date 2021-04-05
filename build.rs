fn main() {
    println!("cargo:rerun-if-changed=shaders/mandelbrot.glsl");
    println!("cargo:rerun-if-changed=shaders/slime.glsl");
    // println!("cargo:rerun-if-changed=shaders/mul12.glsl");
    // println!("cargo:rerun-if-changed=shaders/vertex.glsl");
    // println!("cargo:rerun-if-changed=shaders/fragment.glsl");
}
