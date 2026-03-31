//! MIL (Model Intermediate Language) program builder for ANE
//!
//! Generates MIL text for all ANE kernels used in transformer training/inference.

const MIL_BUILD_INFO: &str = concat!(
    "{{\"coremlc-component-MIL\", \"3510.2.1\"}, ",
    "{\"coremlc-version\", \"3505.4.1\"}, ",
    "{\"coremltools-component-milinternal\", \"\"}, ",
    "{\"coremltools-version\", \"9.0\"}}"
);

/// A MIL program ready for ANE compilation.
pub struct MilProgram {
    pub text: String,
    pub input_channels: usize,
    pub input_spatial: usize,
    pub output_channels: usize,
    pub output_spatial: usize,
}

impl MilProgram {
    pub fn from_text(
        mil: &str,
        input_channels: usize,
        input_spatial: usize,
        output_channels: usize,
        output_spatial: usize,
    ) -> Self {
        MilProgram {
            text: mil.to_string(),
            input_channels,
            input_spatial,
            output_channels,
            output_spatial,
        }
    }

    pub fn as_str(&self) -> &str {
        &self.text
    }
    pub fn input_shape(&self) -> (usize, usize) {
        (self.input_channels, self.input_spatial)
    }
    pub fn output_shape(&self) -> (usize, usize) {
        (self.output_channels, self.output_spatial)
    }
    pub fn input_bytes(&self) -> usize {
        self.input_channels * self.input_spatial * 2
    }
    pub fn output_bytes(&self) -> usize {
        self.output_channels * self.output_spatial * 2
    }
}

/// Start a MIL program with header and function signature.
pub fn mil_header(ic: usize, sp: usize) -> String {
    format!(
        "program(1.3)\n[buildInfo = dict<string, string>({info})]\n{{\n    func main<ios18>(tensor<fp16, [1, {ic}, 1, {sp}]> x) {{\n",
        info=MIL_BUILD_INFO, ic=ic, sp=sp,
    )
}

/// Close a MIL function with output variable.
pub fn mil_footer(output_var: &str) -> String {
    format!("    }} -> ({});\n}}\n", output_var)
}

/// Generate a dynamic matmul block within a MIL function.
/// Slices activations and weights from input, reshapes, transposes, matmuls.
/// Returns the output variable name "{prefix}_y".
pub fn gen_dyn_matmul(
    m: &mut String,
    prefix: &str,
    ic: usize,
    oc: usize,
    seq: usize,
    act_sp_off: usize,
    w_sp_off: usize,
    input_var: &str,
) {
    let p = prefix;
    let iv = input_var;
    *m += &format!("        tensor<int32, [4]> {p}_ba = const()[name=string(\"{p}_ba\"), val=tensor<int32, [4]>([0,0,0,{act_sp_off}])];\n");
    *m += &format!("        tensor<int32, [4]> {p}_sa = const()[name=string(\"{p}_sa\"), val=tensor<int32, [4]>([1,{ic},1,{seq}])];\n");
    *m += &format!("        tensor<fp16, [1,{ic},1,{seq}]> {p}_act = slice_by_size(x={iv},begin={p}_ba,size={p}_sa)[name=string(\"{p}_act\")];\n");
    *m += &format!("        tensor<int32, [4]> {p}_bw = const()[name=string(\"{p}_bw\"), val=tensor<int32, [4]>([0,0,0,{w_sp_off}])];\n");
    *m += &format!("        tensor<int32, [4]> {p}_sw = const()[name=string(\"{p}_sw\"), val=tensor<int32, [4]>([1,{ic},1,{oc}])];\n");
    *m += &format!("        tensor<fp16, [1,{ic},1,{oc}]> {p}_wt = slice_by_size(x={iv},begin={p}_bw,size={p}_sw)[name=string(\"{p}_wt\")];\n");
    *m += &format!("        tensor<int32, [4]> {p}_ra = const()[name=string(\"{p}_ra\"), val=tensor<int32, [4]>([1,1,{ic},{seq}])];\n");
    *m += &format!("        tensor<fp16, [1,1,{ic},{seq}]> {p}_a2 = reshape(shape={p}_ra,x={p}_act)[name=string(\"{p}_a2\")];\n");
    *m += &format!("        tensor<int32, [4]> {p}_pm = const()[name=string(\"{p}_pm\"), val=tensor<int32, [4]>([0,1,3,2])];\n");
    *m += &format!("        tensor<fp16, [1,1,{seq},{ic}]> {p}_a3 = transpose(perm={p}_pm,x={p}_a2)[name=string(\"{p}_a3\")];\n");
    *m += &format!("        tensor<int32, [4]> {p}_rw = const()[name=string(\"{p}_rw\"), val=tensor<int32, [4]>([1,1,{ic},{oc}])];\n");
    *m += &format!("        tensor<fp16, [1,1,{ic},{oc}]> {p}_W = reshape(shape={p}_rw,x={p}_wt)[name=string(\"{p}_W\")];\n");
    *m += &format!("        bool {p}_bF = const()[name=string(\"{p}_bF\"), val=bool(false)];\n");
    *m += &format!("        tensor<fp16, [1,1,{seq},{oc}]> {p}_yh = matmul(transpose_x={p}_bF,transpose_y={p}_bF,x={p}_a3,y={p}_W)[name=string(\"{p}_yh\")];\n");
    *m += &format!("        tensor<fp16, [1,1,{oc},{seq}]> {p}_yt = transpose(perm={p}_pm,x={p}_yh)[name=string(\"{p}_yt\")];\n");
    *m += &format!("        tensor<int32, [4]> {p}_ro = const()[name=string(\"{p}_ro\"), val=tensor<int32, [4]>([1,{oc},1,{seq}])];\n");
    *m += &format!("        tensor<fp16, [1,{oc},1,{seq}]> {p}_y = reshape(shape={p}_ro,x={p}_yt)[name=string(\"{p}_y\")];\n");
}

/// Build a simple dynamic matmul MIL: y = x @ W
pub fn matmul(ic: usize, oc: usize, seq: usize) -> MilProgram {
    let sp = seq + oc;
    let mut m = mil_header(ic, sp);
    gen_dyn_matmul(&mut m, "mm", ic, oc, seq, 0, seq, "x");
    m += &mil_footer("mm_y");
    MilProgram {
        text: m,
        input_channels: ic,
        input_spatial: sp,
        output_channels: oc,
        output_spatial: seq,
    }
}

/// Build an ANE weight blob: 128-byte header + fp16 data.
pub fn build_weight_blob(fp16_data: &[u16]) -> Vec<u8> {
    let weight_bytes = fp16_data.len() * 2;
    let total = 128 + weight_bytes;
    let mut blob = vec![0u8; total];
    blob[0] = 1;
    blob[4] = 2;
    blob[64] = 0xEF;
    blob[65] = 0xBE;
    blob[66] = 0xAD;
    blob[67] = 0xDE;
    blob[68] = 1;
    blob[72..76].copy_from_slice(&(weight_bytes as u32).to_le_bytes());
    blob[80..84].copy_from_slice(&128u32.to_le_bytes());
    for (i, &val) in fp16_data.iter().enumerate() {
        let off = 128 + i * 2;
        blob[off..off + 2].copy_from_slice(&val.to_le_bytes());
    }
    blob
}
