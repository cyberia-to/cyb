use std::collections::HashMap;
use crate::onnx_proto::onnx::NodeProto;

use crate::Backend;
use crate::graph::value::Value;

type Device = <Backend as burn::tensor::backend::Backend>::Device;

pub fn binary_op_proto(
    node: &NodeProto,
    values: &mut HashMap<String, Value>,
    _device: &Device,
    op: &str,
) -> Result<(), String> {
    let a = values.get(&node.input[0])
        .ok_or_else(|| format!("{op}: input {} not found", &node.input[0]))?
        .clone();
    let b = values.get(&node.input[1])
        .ok_or_else(|| format!("{op}: input {} not found", &node.input[1]))?
        .clone();

    let result = match op {
        "add" => a.add(b),
        "sub" => a.sub(b),
        "mul" => a.mul(b),
        "div" => a.div(b),
        "pow" => {
            // pow(a, b) — if b is scalar, use powf_scalar for efficiency
            let b_data = b.to_vec_f32();
            if b_data.len() == 1 {
                a.powf_scalar(b_data[0])
            } else {
                // Element-wise power via exp(b * ln(a))
                let ln_a = a.log();
                (b.mul(ln_a)).exp()
            }
        }
        _ => return Err(format!("Unknown binary op: {op}")),
    };

    values.insert(node.output[0].clone(), result);
    Ok(())
}

pub fn unary_op_proto(
    node: &NodeProto,
    values: &mut HashMap<String, Value>,
    _device: &Device,
    op: &str,
) -> Result<(), String> {
    let input = values.get(&node.input[0])
        .ok_or_else(|| format!("{op}: input {} not found", &node.input[0]))?
        .clone();

    let result = match op {
        "sqrt" => input.sqrt(),
        "neg" => input.neg(),
        "exp" => input.exp(),
        "log" => input.log(),
        "recip" => input.recip(),
        "abs" => input.abs(),
        "erf" => input.erf(),
        _ => return Err(format!("Unknown unary op: {op}")),
    };

    values.insert(node.output[0].clone(), result);
    Ok(())
}
