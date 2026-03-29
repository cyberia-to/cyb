pub mod math;
pub mod matmul;
pub mod activation;
pub mod tensor_manip;
pub mod norm;
pub mod control;

use std::collections::HashMap;
use crate::onnx_proto::onnx::NodeProto;

use crate::Backend;
use crate::graph::value::Value;

type Device = <Backend as burn::tensor::backend::Backend>::Device;

/// Dispatch an ONNX NodeProto to the appropriate burn tensor operation
pub fn dispatch_proto(
    node: &NodeProto,
    values: &mut HashMap<String, Value>,
    device: &Device,
) -> Result<(), String> {
    match node.op_type.as_str() {
        // Math ops
        "Add" => math::binary_op_proto(node, values, device, "add"),
        "Sub" => math::binary_op_proto(node, values, device, "sub"),
        "Mul" => math::binary_op_proto(node, values, device, "mul"),
        "Div" => math::binary_op_proto(node, values, device, "div"),
        "Sqrt" => math::unary_op_proto(node, values, device, "sqrt"),
        "Neg" => math::unary_op_proto(node, values, device, "neg"),
        "Exp" => math::unary_op_proto(node, values, device, "exp"),
        "Log" => math::unary_op_proto(node, values, device, "log"),
        "Pow" => math::binary_op_proto(node, values, device, "pow"),
        "Reciprocal" => math::unary_op_proto(node, values, device, "recip"),
        "Abs" => math::unary_op_proto(node, values, device, "abs"),
        "Erf" => math::unary_op_proto(node, values, device, "erf"),

        // MatMul
        "MatMul" => matmul::matmul_proto(node, values, device),
        "Gemm" => matmul::matmul_proto(node, values, device),

        // Activations
        "Relu" => activation::activation_proto(node, values, device, "relu"),
        "Gelu" => activation::activation_proto(node, values, device, "gelu"),
        "Sigmoid" => activation::activation_proto(node, values, device, "sigmoid"),
        "Softmax" => activation::activation_proto(node, values, device, "softmax"),
        "Tanh" => activation::activation_proto(node, values, device, "tanh"),

        // Tensor manipulation
        "Reshape" => tensor_manip::reshape_proto(node, values, device),
        "Transpose" => tensor_manip::transpose_proto(node, values, device),
        "Concat" => tensor_manip::concat_proto(node, values, device),
        "Squeeze" => tensor_manip::squeeze_proto(node, values, device),
        "Unsqueeze" => tensor_manip::unsqueeze_proto(node, values, device),
        "Gather" => tensor_manip::gather_proto(node, values, device),
        "Shape" => tensor_manip::shape_proto(node, values, device),
        "Slice" => tensor_manip::slice_proto(node, values, device),
        "Cast" => tensor_manip::pass_through(node, values),
        "Identity" => tensor_manip::pass_through(node, values),
        "Constant" => tensor_manip::constant_proto(node, values, device),
        "ConstantOfShape" => tensor_manip::constant_of_shape_proto(node, values, device),
        "Where" => tensor_manip::where_proto(node, values, device),
        "Flatten" => tensor_manip::flatten_proto(node, values, device),
        "Expand" => tensor_manip::pass_through(node, values), // stub

        // Normalization
        "LayerNormalization" => norm::layer_norm_proto(node, values, device),
        "ReduceMean" => norm::reduce_mean_proto(node, values, device),

        // Control flow
        "If" => control::if_proto(node, values, device),
        "Range" => control::range_proto(node, values, device),
        "Split" => control::split_proto(node, values, device),

        // Quantized ops
        "MatMulNBits" => matmul::matmul_nbits_proto(node, values, device),
        "DequantizeLinear" => matmul::dequantize_linear_proto(node, values, device),

        // Specialized fusion ops (Llama ONNX)
        "GroupQueryAttention" => norm::group_query_attention_proto(node, values, device),
        "SkipSimplifiedLayerNormalization" | "SimplifiedLayerNormalization" =>
            norm::simplified_layer_norm_proto(node, values, device),
        "ReduceSum" => norm::reduce_sum_proto(node, values, device),

        // Skip known harmless ops
        "Dropout" => tensor_manip::pass_through(node, values),

        other => {
            log::warn!("Unsupported op: {other} (node: {})", node.name);
            Err(format!("Unsupported ONNX operator: {other}"))
        }
    }
}
