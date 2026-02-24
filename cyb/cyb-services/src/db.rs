use cozo::*;
use serde_json::json;
use std::fs;
use std::sync::Mutex;

fn get_cozo_path() -> Result<String, String> {
    let home_dir = dirs::home_dir().ok_or("Cannot find home directory")?;
    let cyb_dir = home_dir.join(".cyb");

    if !cyb_dir.exists() {
        fs::create_dir_all(&cyb_dir).map_err(|e| e.to_string())?;
    }
    let cozo_dir = cyb_dir.join("cozo");
    if !cozo_dir.exists() {
        fs::create_dir_all(&cozo_dir).map_err(|e| e.to_string())?;
    }

    Ok(cozo_dir.to_string_lossy().into_owned())
}

pub struct DbState {
    pub db: Mutex<DbInstance>,
}

impl DbState {
    pub fn new() -> Self {
        let path = get_cozo_path().unwrap();
        println!("[cyb-services] CozoDB path: {}", &path);

        DbState {
            db: Mutex::new(DbInstance::new("rocksdb", &path, Default::default()).unwrap()),
        }
    }
}

pub fn run_command(db: &mut DbInstance, script: &str, immutable: bool) -> Result<String, String> {
    let mutability = if immutable {
        ScriptMutability::Immutable
    } else {
        ScriptMutability::Mutable
    };

    let result = db
        .run_script(script, Default::default(), mutability)
        .map_err(|e| e.to_string())?;

    let mut result_json = result.into_json();

    if let Some(result_obj) = result_json.as_object_mut() {
        result_obj.insert("ok".to_string(), json!(true));
    }

    let serialized_result = result_json.to_string();

    Ok(serialized_result)
}
