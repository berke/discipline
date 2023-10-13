mod valve;

use serde::{
    Deserialize,
    Serialize
};
use std::{
    time::{
	SystemTime,
	UNIX_EPOCH
    },
    fs::File,
    io::{
	BufReader,
	BufWriter
    },
    path::{
	Path,
	PathBuf
    },
    collections::BTreeMap,
    sync::{
	Arc,
	Mutex
    },
    net::{
	TcpListener,
	TcpStream
    },
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
use rand::Rng;
use discipline_net::*;
use valve::Valve;

struct Config {
    state_path:String
}

struct Controller {
    config:Config,
    state:ControllerState,
    serial:u64,
    valve:Valve
}

impl Controller {
    const SAVE_INTERVAL : f64 = 1.0;
    
    pub fn create_state(config:&Config)->Result<()> {
	let state = ControllerState::new();
	state.atomic_replace(&config.state_path)?;
	Ok(())
    }
    
    pub fn new(config:Config)->Result<Self> {
	let state = ControllerState::load(&config.state_path)?;
	let serial = state.serial();
	let valve = Valve::new(Self::SAVE_INTERVAL);
	Ok(Self { config,state,serial,valve })
    }

    pub fn command(&mut self,env:Envelope<Command>)->Result<Envelope<Response>> {
	let payload = self.state.handle(&env)?;
	if let Some(_) = self.valve.tick() {
	    let new_serial = self.state.serial();
	    if new_serial != self.serial {
		self.serial = new_serial;
		self.state.atomic_replace(&self.config.state_path)?;
	    }
	}
	Ok(Envelope {
	    sender:Entity::Controller,
	    payload,
	    signature:"\\_'')_/".to_string()
	})
    }
}

#[derive(Clone,Debug,Serialize,Deserialize,)]
struct SubjectInfo {
    last_ping:Option<f64>,
    authorized_until:Option<f64>
}

#[derive(Clone,Debug,Serialize,Deserialize,)]
struct ControllerState {
    serial:u64,
    administrators:Vec<String>,
    subjects:BTreeMap<String,SubjectInfo>
}

fn now()->f64 {
    SystemTime::now()
	.duration_since(UNIX_EPOCH)
	.expect("Cannot get timestamp")
	.as_secs_f64()
}

impl ControllerState {
    fn handle(&mut self,
	      env:&Envelope<Command>)->Result<Response> {
	let t_now = now();
	let mut updated = false;

	if let Entity::Subject(subject) = &env.sender {
	    if let Some(subject_info) =
		self.subjects.get_mut(subject) {
		    subject_info.last_ping = Some(t_now);
		    updated = true;
		}
	}

	let err = |u:&str| Ok(Response::Error(u.to_string()));

	let resp =
	    match &env.payload {
		Command::GetStatus { subject } => {
		    if let Some(subject_info) =
			self.subjects.get(subject) {
			    let time_remaining =
				subject_info.authorized_until.map(
				    |t| (t - t_now).max(0.0)
				).unwrap_or(0.0);
			    Ok(Response::Authorization {
				subject:subject.to_string(),
				last_ping:subject_info.last_ping
				    .map(|t| t_now - t),
				time_remaining
			    })
			} else {
			    err("Unknown subject")
			}
		},
		Command::Authorize { subject,duration } => {
		    if let Entity::Administrator(adm) = &env.sender {
			if self.administrators.contains(adm) {
			    if let Some(subject_info) =
				self.subjects.get_mut(subject) {
				    subject_info.authorized_until =
					duration.map(|t| t_now + t);
				    updated = true;
				    Ok(Response::Ack)
				} else {
				    err("Unknown subject")
				}
			} else {
			    err("Unknown administrator")
			}
		    } else {
			err("Only administrators can authorize")
		    }
		}
	    };

	if updated {
	    self.updated();
	}

	resp
    }
}

pub trait Updateable where Self:Sized {
    fn new()->Self;

    fn load<P:AsRef<Path>>(path:P)->Result<Self>;

    fn save<P:AsRef<Path>>(&self,path:P)->Result<()>;

    fn updated(&mut self);

    fn serial(&self)->u64;

    fn atomic_replace<P:AsRef<Path>>(&self,path:P)->Result<()> {
	let mut tmp_path : PathBuf = path.as_ref().into();
	let id = random_id();
	tmp_path.set_extension(&id);
	self.save(&tmp_path)?;
	std::fs::rename(tmp_path,path)?;
	Ok(())
    }
}

fn random_id()->String {
    // XXX
    let mut rng = rand::thread_rng();
    let x = rng.gen::<u64>();
    format!("{:016X}",x)
}

impl Updateable for ControllerState {
    fn new()->Self {
	Self {
	    serial:0,
	    administrators:Vec::new(),
	    subjects:BTreeMap::new()
	}
    }

    fn load<P:AsRef<Path>>(path:P)->Result<Self> {
	let fd = File::open(path)?;
	let buf = BufReader::new(fd);
	Ok(ron::de::from_reader(buf)?)
    }
    
    fn save<P:AsRef<Path>>(&self,path:P)->Result<()> {
	let fd = File::create(path)?;
	let buf = BufWriter::new(fd);
	Ok(ron::ser::to_writer(buf,&self)?)
    }

    fn updated(&mut self) {
	self.serial += 1;
    }

    fn serial(&self)->u64 {
	self.serial
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
		      msg:&Message)->Result<Envelope<Response>> {
	match msg {
	    Message::Text(u) => {
		let cmd : Envelope<Command> = serde_json::from_str(&u)
		    .map_err(|e| anyhow!("Invalid JSON: {}",e))?;
		ctl.lock().unwrap().command(cmd)
	    },
	    _ => bail!("Invalid message type")
	}
    }
    
    fn handle(ctl:Arc<Mutex<Controller>>,stream:TcpStream)->Result<()> {
	let mut websocket = accept(stream)?;
	loop {
	    let msg = websocket.read()?;
	    if msg.is_close() {
		break;
	    }
	    let response =
		Self::handle_message(&ctl,&msg)
		.map_err(|e| format!("{}",e));
	    let v = serde_json::to_string(&response)?;
	    websocket.send(Message::Text(v))?;
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

    let state_path : String = args.opt_value_from_str("--state-path")?
	.unwrap_or_else(|| "state.dat".to_string());

    let create_state = args.contains("--create-state");

    let rest = args.finish();
    if !rest.is_empty() {
	bail!("Invalid arguments: {:?}",rest);
    }

    let config = Config { state_path };

    if create_state {
	Controller::create_state(&config)?;
    }
    
    let mut api_srv = ApiServer::new(&listen_addr,config)?;

    api_srv.run()
}
