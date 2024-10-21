use sqlite_vec::sqlite3_vec_init;
use rusqlite::{ffi::sqlite3_auto_extension, Connection};

use godot::{classes::*, prelude::*};
use zerocopy::IntoBytes;

#[derive(GodotClass)]
#[class(base=Node)]
struct VectorDB {
    connection: rusqlite::Connection,
    base: Base<Node>
}

#[godot_api]
impl INode for VectorDB {
    fn init(base: Base<Node>) -> Self {
        unsafe {
            sqlite3_auto_extension(Some(std::mem::transmute(sqlite3_vec_init as *const ())));
        }
        let connection = Connection::open_in_memory().expect("Failed to create sqlite instance");
        let v: Vec<f32> = vec![0.1, 0.2, 0.3];
        let (sqlite_version, vec_version, x): (String, String, String) = connection.query_row(
                "select sqlite_version(), vec_version(), vec_to_json(?)",
                &[v.as_bytes()],
                |x| Ok((x.get(0)?, x.get(1)?, x.get(2)?)),
            ).expect("could not get the version");
        connection.execute(
                "CREATE VIRTUAL TABLE vec_items USING vec0(embedding float[4])",
                [],
            ).expect("Failed to create table");
        godot_print!("sqlite_version={sqlite_version} vec_version={vec_version}");
        Self {
            connection,
            base,
        }
    }
}

#[godot_api]
impl VectorDB {
    fn insert_embeding(self, data:PackedInt32Array){
        let arr = data.to_vec();
        let bytes = arr.as_bytes();
        let mut smt = self.connection.prepare("INSERT INTO vec_items(rowid, embedding) VALUES (?, ?)").expect("failed to create prepare statement");
        smt.execute(rusqlite::params![0, bytes.as_bytes()]).expect("failed to execute statement");
    }
}

struct Extension;

#[gdextension]
unsafe impl ExtensionLibrary for Extension {}
