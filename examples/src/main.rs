use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
struct Response {
    message: String,
    code: i32,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let resp: Response = reqwest::get("http://127.0.0.1:3001/api/v1/play/1")
        .await?
        .json()
        .await?;
    println!("{:#?}", resp);
    Ok(())
}
