use clap::Parser;
use poem::{listener::TcpListener, middleware::AddData, web::Data, EndpointExt, Route, Server};
use poem_openapi::{param::Path, payload::Json, Object, Enum, OpenApi, OpenApiService};
use rodio::{OutputStream, Sink};
use std::io::BufReader;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

struct Api;

/// API请求返回操作码
#[derive(Debug, Enum)]
#[oai(rename_all="camelCase")]
pub enum ResponseCode {
    /// 正常
    Success,
    /// 参数错误
    ParameterError,
    /// 内部错误
    ServerError,
}

#[derive(Object)]
struct Response {
    msg: String,
    code: ResponseCode,
}

#[OpenApi]
impl Api {
    #[oai(path = "/play/:name", method = "get")]
    async fn play(
        &self,
        tx: Data<&Arc<Mutex<Sender<String>>>>,
        assets_path: Data<&String>,
        name: Path<String>,
    ) -> Json<Response> {
        let path = std::path::Path::new(assets_path.as_str()).join(name.0);
        if !path.exists() {
            return Json(Response {
                msg: "File not found".to_owned(),
                code: ResponseCode::ParameterError,
            });
        }

        match tx.lock() {
            Ok(tx) => {
                tx.send(String::from(path.to_str().unwrap())).unwrap();
                return Json(Response {
                    msg: "success".to_owned(),
                    code: ResponseCode::Success,
                });
            }
            Err(_) => {
                return Json(Response {
                    msg: "Failed to send message".to_owned(),
                    code: ResponseCode::ServerError,
                });
            }
        }
    }
}

fn paly(rx: Receiver<String>) {
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let mut sink: Option<Sink> = None;
    for path in rx {
        if let Some(s) = &sink {
            s.stop();
        }

        if let Ok(file) = std::fs::File::open(&path) {
            match stream_handle.play_once(BufReader::new(file)) {
                Ok(s) => sink = Some(s),
                Err(error) => {
                    println!("failed to play:{:?}", error);
                }
            }
        }
    }
}
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Path of the assets
    #[clap(short = 'p', long, default_value = "./assets")]
    assets_path: String,
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let args = Args::parse();
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "poem=info");
    }
    tracing_subscriber::fmt::init();

    let (tx, rx) = channel();
    let tx = Arc::new(Mutex::new(tx));
    // 声音播放线程
    thread::spawn(move || {
        paly(rx);
    });

    let api_service =
        OpenApiService::new(Api, "Audio Server", "1.0").server("http://localhost:3001/api/v1");

    let ui = api_service.swagger_ui();

    let route = Route::new()
        .nest("/api/v1", api_service)
        .nest("/", ui)
        .with(AddData::new(tx))
        .with(AddData::new(args.assets_path));

    Server::new(TcpListener::bind("0.0.0.0:3001"))
        .run(route)
        .await?;

    Ok(())
}
