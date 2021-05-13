use futures::prelude::*;
use gotham::hyper::{upgrade::OnUpgrade, Body, HeaderMap, Response, StatusCode};
use gotham::state::{request_id, FromState, State};
use winput::{Vk, Button};
use std::{fs::File, io};
use std::io::{Write, BufReader, BufRead};
use std::sync::Mutex;
use anyhow::Error;

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate anyhow;

mod ws;

lazy_static! {
    static ref global_lines: Mutex<Vec<String>>=Mutex::new(Vec::new());
}

fn main()-> Result<(), Error>  {
    pretty_env_logger::init();

    let input_file = File::open("./layout.txt")?;
    let buffered = BufReader::new(input_file);

    let mut lines= buffered
        .lines()
        .collect::<io::Result<Vec<String>>>()?
        .iter()
        .enumerate()
        .map(|(i, line)| format!("{}:{}",i, line))
        .collect::<Vec<String>>();

    global_lines.lock().map_err(|_| anyhow!("aliens attacked"))?.append(&mut lines);

    println!("{:#?}", global_lines.lock().map_err(|_| anyhow!("aliens attacked") )?);

    let addr = "192.168.1.94:5000";

    println!("Listening on http://{}/", addr);

    gotham::start(addr, || Ok(handler));
    Ok(())
}

//handeler checks if everything is fine or not
fn handler(mut state: State) -> (State, Response<Body>) {
    let headers = HeaderMap::take_from(&mut state);
    let on_upgrade = OnUpgrade::try_take_from(&mut state);

    match on_upgrade {
        Some(on_upgrade) if ws::requested(&headers) => {
            let (response, ws) = match ws::accept(&headers, on_upgrade) {
                Ok(res) => res,
                Err(_) => return (state, bad_request()),
            };

            let req_id = request_id(&state).to_owned();

            tokio::spawn(async move {
                match ws.await {
                    Ok(ws) => connected(req_id, ws).await,
                    Err(err) => {
                        eprintln!("websocket init error: {}", err);
                        Err(())
                    }
                }
            });

            (state, response)
        }
        _ => (state, Response::new(Body::from(INDEX_HTML))),
    }
}

//the connection, it's the main function I guess
async fn connected<S>(req_id: String, stream: S) -> Result<(), ()>
where
    S: Stream<Item = Result<ws::Message, ws::Error>> + Sink<ws::Message, Error = ws::Error>,
{
    let (mut _sink, mut stream) = stream.split();
    //if a client enters say so
    println!("Client {} connected", req_id);

    while let Some(message) = stream
        .next()
        .await
        .transpose()
        .map_err(|error| println!("Websocket receive error: {}", error))?
    {
        println!("{}: {}", req_id, message);


        winput::send(&global_lines.lock().map_err(|_| anyhow!("aliens attacked") )[0]); 

        //echo "message" back 
        /*match sink.send(message).await {
            Ok(()) => (),
             this error indicates a successfully closed connection
            Err(ws::Error::ConnectionClosed) => break,
            Err(error) => {
                println!("Websocket send error: {}", error);
                return Err(());
            }
        }*/
       
    }

    println!("Client {} disconnected", req_id);
    Ok(())
}

//lol idk
fn bad_request() -> Response<Body> {
    Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .body(Body::empty())
        .unwrap()
}

const INDEX_HTML: &str = include_str!("index.html");
//there were tests here but they made me confused