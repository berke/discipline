mod ptr;

use ptr::*;

use anyhow::{
    anyhow,
    bail,
    Result
};

use url::Url;
use tungstenite::{
    connect,
    stream::{
	MaybeTlsStream
    },
    client::IntoClientRequest,
    WebSocket,
    Message
};

use gtk4 as gtk;

use gtk::{
    prelude::*,
    glib,
    Application,
    ApplicationWindow,
    Box,
    Button,
    Entry,
    Frame,
    InputPurpose,
    Label,
    Orientation
};

use std::{
    thread::{
	self,
	JoinHandle
    },
    collections::VecDeque,
    time::Duration,
    net::TcpStream,
    sync::{
	Arc,
	Mutex,
	mpsc::{
	    self,
	    Receiver,
	    Sender,
	    TryRecvError
	}
    }
};

use discipline_net::*;

const APP_ID : &str = "fr.exhrd.Discipline";

mod config {
    use anyhow::Result;
    use std::path::Path;
    use serde::Deserialize;

    #[derive(Debug,Clone,Deserialize)]
    pub struct Config {
	pub server_url:String,
	pub retry_delay:f64,
	pub loop_delay:f64,
	pub name:String,
	pub kids:Vec<String>
    }

    impl Config {
	pub fn open<P:AsRef<Path>>(path:P)->Result<Self> {
	    let fd = std::fs::File::open(path)?;
	    let this : Self = ron::de::from_reader(fd)?;
	    Ok(this)
	}
    }
}

use config::Config;

const CONFIG_PATH : &str = "ui/etc/discipline.cfg";

struct BackendConnection {
    config:Config,
    recv:Receiver<Command>,
    send:Sender<Response>
}

impl BackendConnection {
    pub fn new(config:Config)->Result<(JoinHandle<()>,Sender<Command>,
				       Receiver<Response>)> {
	let (sender1,receiver1) = mpsc::channel();
	let (sender2,receiver2) = mpsc::channel();

	let mut this = Self { config,
			      recv:receiver1,
			      send:sender2 };
	
	let jh = thread::Builder::new()
	    .spawn(move || {
		loop {
		    match this.run() {
			Ok(()) => (),
			Err(e) => eprintln!("Backend thread exited abnormally: {}",e)
		    }
		    std::thread::sleep(
			std::time::Duration::from_secs_f64(
			    this.config.retry_delay));
		}
	    })?;
	Ok((jh,sender1,receiver2))
    }

    fn reader<Req:IntoClientRequest>(
	mut socket:WebSocket<MaybeTlsStream<TcpStream>>,
	send:Sender<Response>)->Result<()> {
	loop {
	    let msg = socket.read()?;
	    match msg {
		Message::Text(u) => {
		    let resp : Result<Envelope<Response>,String> = serde_json::from_str(&u)
			.map_err(|e| anyhow!("Invalid JSON: {}",e))?;
		    match resp {
			Ok(env) => {
			    send.send(env.payload)?;
			},
			Err(e) => bail!("Error: {}",e)
		    }
		},
		_ => bail!("Invalid message type")
	    }
	}
    }

    fn run(&mut self)->Result<()> {
	let url = Url::parse(&self.config.server_url)?;
	let (mut socket,_response) = connect(url)?;

	// let jh = thread::Builder::new()
	//     .spawn({
	// 	move || {
	// 	    Self::reader(socket.clone(),
	// 			 self.send)
	// 		.expect("Reader failed")
	// 	}
	//     })?;

	loop {
	    let payload = self.recv.recv()?;
	    println!("<<< Payload {:?}",payload);
	    let sender = Entity::Administrator(self.config.name.clone());
	    let cmd = Envelope {
		sender,
		signature:"\\_'')_/".to_string(),
		payload
	    };
	    let v = serde_json::to_string(&cmd)?;
	    socket.send(Message::Text(v))?;
	}
	Ok(())
    }
}
    
fn main()->glib::ExitCode {
    let app = Application::builder()
	.application_id(APP_ID)
	.build();

    app.connect_activate(|app| {
	let config = Config::open(CONFIG_PATH)
	    .expect("Cannot open configuration file");

	let (jh,send_cmd,receive_resp) = BackendConnection::new(config.clone())
	    .expect("Cannot start backend connection");

	let send_cmd = Ptr::make(send_cmd);
	let receive_resp = Ptr::make(receive_resp);

	let window = ApplicationWindow::builder()
	    .application(app)
	    .default_width(640)
	    .default_height(512)
	    .title("Discipline")
	    .build();

	let box1 = Box::new(Orientation::Vertical,8);
	for kid in config.kids.iter() {
	    let frame = Frame::builder()
		.label(kid)
		.hexpand(true)
		.build();
	    let box2 = Box::new(Orientation::Horizontal,8);

	    let authorize_label = Label::new(Some(" Authorize:"));
	    box2.append(&authorize_label);
	    let authorize_30min = Button::with_label("30min");
	    box2.append(&authorize_30min);
	    let authorize_1h = Button::with_label("1h");
	    box2.append(&authorize_1h);

	    authorize_1h.connect_clicked({
		let send_cmd = send_cmd.refer();
		let kid = kid.clone();
		move |_| {
		    println!("Authorize {} for 1h",kid);

		    let cmd = 
			Command::Authorize { subject:kid.clone(),
					     duration:Some(3600.0) };
		    send_cmd.yank_mut().send(cmd).expect("Cannot send");
		}
	    });

	    let duration_h = Entry::builder()
		.input_purpose(InputPurpose::Number)
		.build();
	    box2.append(&duration_h);
	    let authorize_30min = Button::with_label(" hours");
	    box2.append(&authorize_30min);
	    
	    let cancel = Button::with_label("Cancel");
	    box2.append(&cancel);

	    frame.set_child(Some(&box2));
	    box1.append(&frame);
	}
	window.set_child(Some(&box1));
	window.present();
    });

    app.run()
}
