use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Response {
    message: Option<String>,
    code: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let resp: Response = reqwest::get("http://127.0.0.1:3001/api/v1/play/1.wav")
        .await?
        .json()
        .await?;
    println!("{:#?}", resp);
    Ok(())
}
