use run::format::LoadedModel;
use run::tokenizer::build_tokenizer;

fn main() {
    let path = std::path::PathBuf::from(std::env::var("HOME").unwrap()).join("llm/qwen3-0.6b-abl.model");
    let lm = LoadedModel::load(&path).unwrap();
    let tok = build_tokenizer(&lm).unwrap();
    let prompt = "<|im_start|>user\nWhat is 2+2?<|im_end|>\n<|im_start|>assistant\n";
    let ids = tok.encode(prompt);
    println!("prompt = {:?}", prompt);
    println!("ids ({} tokens):", ids.len());
    for id in &ids {
        let s = tok.decode(&[*id], false);
        println!("  {:>6}  {:?}", id, s);
    }
    // Compare: what does plain text "The answer to 2+2 is" give?
    let plain = "The answer to 2+2 is";
    let ids2 = tok.encode(plain);
    println!();
    println!("plain = {:?}", plain);
    println!("ids ({} tokens): {:?}", ids2.len(), ids2);
}
