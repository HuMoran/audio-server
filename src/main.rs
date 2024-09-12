use clap::Parser;
use poem::Endpoint;
use poem::{listener::TcpListener, middleware::AddData, web::Data, EndpointExt, Route, Server};
use poem_openapi::{param::Path, payload::Json, Enum, Object, OpenApi, OpenApiService};
use rodio::{Decoder, OutputStream, Sink, Source};
use std::io::BufReader;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

struct Api;

/// API 请求返回操作码
#[derive(Debug, Enum)]
#[oai(rename_all = "camelCase")]
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

#[derive(Debug)]
pub enum Message {
    Play(String),
    PlayLoop(String),
    Pause,
    Resume,
    Stop,
}

#[OpenApi]
impl Api {
    #[oai(path = "/play/:name", method = "get")]
    async fn play(
        &self,
        tx: Data<&Arc<Mutex<Sender<Message>>>>,
        assets_path: Data<&String>,
        name: Path<String>,
    ) -> Json<Response> {
        let path = std::path::Path::new(assets_path.as_str()).join(name.0);
        if !path.exists() {
            return Json(Response {
                msg: "File not found".to_string(),
                code: ResponseCode::ParameterError,
            });
        }

        let path = path.to_str().unwrap().to_string();
        tx.lock().map(|t| t.send(Message::Play(path))).ok();
        Json(Response {
            msg: "success".to_string(),
            code: ResponseCode::Success,
        })
    }

    #[oai(path = "/play-loop/:name", method = "get")]
    async fn play_loop(
        &self,
        tx: Data<&Arc<Mutex<Sender<Message>>>>,
        assets_path: Data<&String>,
        name: Path<String>,
    ) -> Json<Response> {
        let path = std::path::Path::new(assets_path.as_str()).join(name.0);
        if !path.exists() {
            return Json(Response {
                msg: "File not found".to_string(),
                code: ResponseCode::ParameterError,
            });
        }

        let path = path.to_str().unwrap().to_string();
        tx.lock().map(|t| t.send(Message::PlayLoop(path))).ok();
        Json(Response {
            msg: "success".to_string(),
            code: ResponseCode::Success,
        })
    }

    #[oai(path = "/pause", method = "get")]
    async fn pause(&self, tx: Data<&Arc<Mutex<Sender<Message>>>>) -> Json<Response> {
        tx.lock().map(|t| t.send(Message::Pause)).ok();
        Json(Response {
            msg: "success".to_string(),
            code: ResponseCode::Success,
        })
    }

    #[oai(path = "/resume", method = "get")]
    async fn resume(&self, tx: Data<&Arc<Mutex<Sender<Message>>>>) -> Json<Response> {
        tx.lock().map(|t| t.send(Message::Resume)).ok();
        Json(Response {
            msg: "success".to_string(),
            code: ResponseCode::Success,
        })
    }

    #[oai(path = "/stop", method = "get")]
    async fn stop(&self, tx: Data<&Arc<Mutex<Sender<Message>>>>) -> Json<Response> {
        tx.lock().map(|t| t.send(Message::Stop)).ok();
        Json(Response {
            msg: "success".to_string(),
            code: ResponseCode::Success,
        })
    }
}

fn paly(rx: Receiver<Message>) {
    let Ok((_stream, stream_handle)) = OutputStream::try_default() else {
        eprintln!("try open default output stream error");
        std::process::exit(-1);
    };
    let Ok(sink) = Sink::try_new(&stream_handle) else {
        eprintln!("try new sink byt default output stream handle error");
        std::process::exit(-1);
    };

    for msg in rx {
        match msg {
            Message::Play(path) => {
                let Ok(file) = std::fs::File::open(&path) else {
                    eprintln!("open file {path} error");
                    continue;
                };

                sink.stop();
                if let Err(e) = Decoder::new(BufReader::new(file)).map(|s| {
                    sink.append(s);
                    sink.play();
                }) {
                    eprintln!("decode file {path} error: {e}");
                }
            }
            Message::PlayLoop(path) => {
                let Ok(file) = std::fs::File::open(&path) else {
                    eprintln!("open file {path} error");
                    continue;
                };

                sink.stop();
                if let Err(e) = Decoder::new(BufReader::new(file)).map(|s| {
                    sink.append(s.repeat_infinite());
                    sink.play();
                }) {
                    eprintln!("decode file {path} error: {e}");
                }
            }
            Message::Pause => {
                sink.pause();
            }
            Message::Resume => {
                sink.play();
            }
            Message::Stop => {
                sink.stop();
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
        .around(|ep, req| async move {
            let uri = req.uri().clone();
            let method = req.method().clone();
            let resp = ep.get_response(req).await;
            println!("[{}] {}", method, uri,);

            Ok(resp)
        })
        .with(AddData::new(tx))
        .with(AddData::new(args.assets_path));

    Server::new(TcpListener::bind("127.0.0.1:3001"))
        .run(route)
        .await?;

    Ok(())
}
