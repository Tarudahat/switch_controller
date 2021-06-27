use futures::prelude::*;
use gotham::{hyper::{upgrade::OnUpgrade, Body, HeaderMap, Response, StatusCode}};
use gotham::state::{request_id, FromState, State};
use std::{fs::File, io};
use std::io::{BufReader, BufRead};
use std::sync::Mutex;
use anyhow::Error;
use enigo::{Enigo, MouseControllable, MouseButton, KeyboardControllable, Key};
use serde::{Deserialize};

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate anyhow;

mod ws;

lazy_static! {
    static ref GLOBAL_LINES: Mutex<Vec<String>>=Mutex::new(Vec::new());
}

#[derive(Deserialize, Debug)]
struct Controller
{
    buttons:Vec<bool>,
    axes:Vec<f32>,
    left_stick:Vec<bool>,
}

fn handle_input(input_str:&String){
    match input_str.as_str() {
        "MS_L"=>Enigo.mouse_click(MouseButton::Left),
        "MS_R"=>Enigo.mouse_click(MouseButton::Right),
        "MS_M"=>Enigo.mouse_click(MouseButton::Middle),
        "ESC"=>Enigo.key_click(Key::Escape),
        "ENTER"=>Enigo.key_click(Key::Return),
        "SHIFT"=>Enigo.key_click(Key::Shift),
        "DELETE"=>Enigo.key_click(Key::Delete),
        "ALT"=>Enigo.key_click(Key::Alt),
        "UP"=>Enigo.key_click(Key::UpArrow),
        "DOWN"=>Enigo.key_click(Key::DownArrow),
        "LEFT"=>Enigo.key_click(Key::LeftArrow),
        "RIGHT"=>Enigo.key_click(Key::RightArrow),
        "_"=>{},
        _=>{
            //using expect here isn't that big brain
            let char=input_str.chars().last().expect("ERROR: input was not a char nor an accepted string");
            Enigo.key_click(Key::Layout(char))
        },
    }
}

fn main()-> Result<(), Error>  {
    pretty_env_logger::init();

    let input_file = File::open("./layout.txt")?;
    let buffered = BufReader::new(input_file);

    let mut lines= buffered
        .lines()
        .collect::<io::Result<Vec<String>>>()?;

    GLOBAL_LINES.lock().map_err(|_| anyhow!("aliens attacked"))?.append(&mut lines);

    println!("{:#?}", GLOBAL_LINES.lock().map_err(|_| anyhow!("aliens attacked") )?);

    let addr= "192.168.1.10:5000";// = GLOBAL_LINES.lock().map_err(|_| anyhow!("aliens attacked"))?.last().expect("q");

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
    let mut enigo = Enigo::new();
    println!("Client {} connected", req_id);
    while let Some(message) = stream
        .next()
        .await
        .transpose()
        .map_err(|error| println!("Websocket receive error: {}", error))?
    {
        println!("{}: {}", req_id, message);

        if let ws::Message::Text(text)=message{
            let output_str = &text;
            
            //controller data
            let cd: Controller= serde_json::from_str(output_str).map_err(|_| println!("JSON object wack"))?;

            //cursed rust code
            for i in 0..16 {
                if cd.buttons[i]
                {
                    let temp_str=GLOBAL_LINES.lock().map_err(|_| println!("aliens attacked"))?;
                    handle_input(temp_str.get(i).expect("some thing is wrong, idk I'm to tired"));
                }
            }
        
            enigo.mouse_move_relative((cd.axes[2]*15.0) as i32, (cd.axes[3]*15.0) as i32);

        }

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