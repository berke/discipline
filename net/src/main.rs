use serde::{
    Deserialize,
    Serialize
};

use std::{
    sync::{Arc,Mutex},
    net::{TcpListener,TcpStream},
    thread::spawn
};
use tungstenite::{
    accept,
    Message
};
use pico_args::Arguments;
use anyhow::{
    anyhow,
    bail,
    Result
};

#[derive(Debug,Serialize,Deserialize,Clone,Copy)]
enum Command {
    Query
}

#[derive(Debug,Serialize,Deserialize,Clone,Copy)]
enum Response {
    Authorization
}

struct Config {
}

struct Controller {
    config:Config
}

impl Controller {
    pub fn new(config:Config)->Result<Self> {
	Ok(Self { config })
    }

    pub fn command(&mut self,cmd:Command)->Result<Response> {
	Ok(Response::Authorization)
    }
}

struct ApiServer {
    ctl:Arc<Mutex<Controller>>,
    server:TcpListener
}

impl ApiServer {
    pub fn new(listen_addr:&str,config:Config)->Result<Self> {
	let ctl = Arc::new(Mutex::new(Controller::new(config)?));
	let server = TcpListener::bind(listen_addr)?;
	Ok(Self {
	    ctl,
	    server
	})
    }

    fn handle_message(ctl:&Arc<Mutex<Controller>>,
		      msg:&Message)->Result<Response> {
	match msg {
	    Message::Text(u) => {
		let msg = serde_json::from_str(&u)
		    .map_err(|e| anyhow!("Invalid JSON: {}",e))?;
		let cmd = Command::from(msg);
		println!("Command received: {:?}",cmd);
		ctl.lock().unwrap().command(cmd)
	    },
	    _ => bail!("Invalid message type")
	}
    }
    
    fn handle(ctl:Arc<Mutex<Controller>>,stream:TcpStream)->Result<()> {
	let mut websocket = accept(stream)?;
	loop {
	    let msg = websocket.read_message()?;
	    if msg.is_close() {
		break;
	    }
	    let response =
		Self::handle_message(&ctl,&msg)
		.map_err(|e| format!("{}",e));
	    let v = serde_json::to_string(&response)?;
	    websocket.write_message(Message::Text(v))?;
	}
	Ok(())
    }

    pub fn run(&mut self)->Result<()> {
	for stream in self.server.incoming() {
	    let stream = stream?;
	    let ctl = Arc::clone(&self.ctl);
	    spawn (|| {
		Self::handle(ctl,stream).unwrap();
	    });
	}
	Ok(())
    }
}

fn main()->Result<()> {
    let progname : String = std::env::args().nth(0).unwrap();
    
    let mut args = Arguments::from_env();

    if args.contains("-h") || args.contains("--help") {
	eprintln!("Usage: {} [--listen ADDR:PORT]",
		  progname);
	return Ok(())
    }

    let listen_addr = args.opt_value_from_str("--listen")?
	.unwrap_or_else(|| "127.0.0.1:9001".to_string());

    let rest = args.finish();
    if !rest.is_empty() {
	bail!("Invalid arguments: {:?}",rest);
    }

    let config = Config { };

    let mut api_srv = ApiServer::new(&listen_addr,config)?;

    api_srv.run()
}
