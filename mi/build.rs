fn main() {
    prost_build::compile_protos(&["proto/onnx.proto3"], &["proto/"])
        .expect("Failed to compile ONNX proto");
}
