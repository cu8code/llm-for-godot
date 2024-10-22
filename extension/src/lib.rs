use rusqlite::{ffi::sqlite3_auto_extension, Connection};
use sqlite_vec::sqlite3_vec_init;

use godot::{classes::*, prelude::*};
use zerocopy::IntoBytes;

use serde::{Deserialize, Serialize};
use serde_json::json;
use ureq;

// Define the structure of the request body and response for the embedding API
#[derive(Serialize, Deserialize, Debug)]
struct EmbeddingRequest {
    input: String,
    model: String,
}

#[derive(Deserialize, Debug)]
struct EmbeddingData {
    // Assuming each data item contains an embedding array (replace with actual structure)
    embedding: Vec<f32>,
}

#[derive(Deserialize, Debug)]
struct EmbeddingResponse {
    object: String,
    data: Vec<EmbeddingData>, // The `data` field contains an array of embedding data
    model: String,
    usage: Usage,
}

#[derive(Deserialize, Debug)]
struct Usage {
    prompt_tokens: i32,
    total_tokens: i32,
}

#[derive(GodotClass)]
#[class(base=Node)]
struct VectorDB {
    connection: rusqlite::Connection,
    base: Base<Node>,
}

#[godot_api]
impl INode for VectorDB {
    fn init(base: Base<Node>) -> Self {
        unsafe {
            sqlite3_auto_extension(Some(std::mem::transmute(sqlite3_vec_init as *const ())));
        }

        let connection = Connection::open_in_memory().expect("Failed to create sqlite instance");

        // Create the item table with id and content
        connection.execute(
            "CREATE TABLE item (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                content TEXT NOT NULL
            )",
            [],
        ).expect("Failed to create item table");

        // Create the vec_items table with item_id and embedding
        connection.execute(
            "CREATE VIRTUAL TABLE vec_items USING vec0(embedding float[4096])",
            [],
        ).expect("Failed to create vec_items table");

        godot_print!("SQLite tables created successfully.");
        Self { connection, base }
    }
}

#[godot_api]
impl VectorDB {
    #[func]
    fn create_embeddings(&self, input_text: String) {
        // Step 1: Create the request payload
        let embedding_request = EmbeddingRequest {
            input: input_text.clone(),
            model: "zephyr-7b-beta-Q5_K_M".to_string(), // Replace with the actual model name
        };
        let embedding_request_json = serde_json::to_string(&embedding_request).expect("Failed to serialize");

        // Step 2: Make the HTTP POST request to the embedding API
        let response = ureq::post("https://api.vultrinference.com/v1/embeddings") // Replace with actual API URL
            .set("Authorization", "SXLJCOFRWCV7KAC6XUR5WPXR5NZ5R33MRRWQ") // Replace with actual API key
            .send_json(json!(embedding_request));

        // Step 3: Check if the response is successful
        if let Ok(response) = response {
            // Parse the response as JSON
            let embedding_response: EmbeddingResponse = response.into_json().expect("Failed to parse response");

            // Step 4: Extract the embeddings from the `data` array
            for embedding_data in embedding_response.data {
                let embedding = embedding_data.embedding; // Extract embedding vector

                assert_eq!(embedding.len(), 4096);

                // Convert the embedding vector to bytes for storage in SQLite
                let bytes = embedding.as_bytes();

                // Step 5: Insert the item into the item table
                let mut item_stmt = self
                    .connection
                    .prepare("INSERT INTO item(content) VALUES (?)")
                    .expect("Failed to prepare statement for item");

                // Execute the insert to the item table
                item_stmt.execute(rusqlite::params![input_text]).expect("Failed to insert item");

                // Get the last inserted row ID from the item table
                let item_id: i64 = self.connection.last_insert_rowid();

                // Step 6: Insert the embedding into the vec_items table
                let mut vec_stmt = self
                    .connection
                    .prepare("INSERT INTO vec_items(rowid, embedding) VALUES (?, ?)")
                    .expect("Failed to prepare statement for vec_items");

                vec_stmt.execute(rusqlite::params![item_id, bytes]).expect("Failed to insert embedding");

                godot_print!("Embedding created and stored for input: {}", input_text);
            }
        } else {
            godot_print!("Failed to create embeddings: {}", embedding_request_json);
        }
    }

    #[func]
    fn match_item(&self, item: String) {
        // Step 1: Create the request payload
        let embedding_request = EmbeddingRequest {
            input: item.clone(),
            model: "zephyr-7b-beta-Q5_K_M".to_string(), // Replace with the actual model name
        };
        let embedding_request_json = serde_json::to_string(&embedding_request).expect("Failed to serialize");

        // Step 2: Make the HTTP POST request to the embedding API
        let response = ureq::post("https://api.vultrinference.com/v1/embeddings") // Replace with actual API URL
            .set("Authorization", "SXLJCOFRWCV7KAC6XUR5WPXR5NZ5R33MRRWQ") // Replace with actual API key
            .send_json(json!(embedding_request));

        // Step 3: Check if the response is successful
        if let Ok(response) = response {
            // Parse the response as JSON
            let embedding_response: EmbeddingResponse =
                response.into_json().expect("Failed to parse response");

            // Step 4: Extract the embeddings from the `data` array
            for embedding_data in embedding_response.data {
                let embedding = embedding_data.embedding; // Extract embedding vector

                assert_eq!(embedding.len(), 4096);

                // Convert the embedding vector to bytes for storage in SQLite
                let bytes = embedding.as_bytes();

                // Step 5: Perform the distance-based matching against the vec_items table
                let results: Vec<(i64, f64)> = self
                    .connection
                    .prepare(
                        r"
                        SELECT
                            rowid,
                            distance
                        FROM vec_items
                        WHERE embedding MATCH ?1
                        ORDER BY distance
                        LIMIT 3
                        ",
                    ).expect("Failed to prepare query")
                    .query_map([bytes], |row| Ok((row.get(0)?, row.get(1)?))).expect("msg")
                    .collect::<Result<Vec<_>, _>>().expect("msg");

                // Step 6: Fetch the content of matched items from the items table
                for (item_id, distance) in results {
                    let content: String = self.connection.query_row(
                        "SELECT content FROM item WHERE id = ?",
                        [item_id],
                        |row| row.get(0),
                    ).expect("Failed to fetch content for item");

                    godot_print!("Matched Item ID: {}, Distance: {}, Content: {}", item_id, distance, content);
                }
            }
        } else {
            godot_print!("Failed to create embeddings: {}", embedding_request_json);
        }
    }

}

struct Extension;

#[gdextension]
unsafe impl ExtensionLibrary for Extension {}
