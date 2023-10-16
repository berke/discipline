mod ptr;

use ptr::*;

use anyhow::{
    anyhow,
    bail,
    Result
};

use url::Url;

use futures_util::{
    SinkExt,
    StreamExt
};

use tokio::{
    runtime::{
	Builder
    },
    sync::mpsc::{
	self,
	Receiver,
	Sender,
	error::TryRecvError,
    }
};
use tokio_tungstenite::{
    self as tt,
    tungstenite
};
use tungstenite::Message;

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
    Orientation,
    PolicyType,
    ScrolledWindow,
    ScrollablePolicy,
    TextBuffer,
    TextView
};

use std::{
    fmt::Display
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
    pub fn new(config:Config)->Result<(Sender<Command>,
				       Receiver<Response>)> {
	const BUF_SIZE : usize = 8;

	let runtime = Builder::new_current_thread()
	    .enable_all()
	    .build()
	    .expect("Cannot build Tokio runtime");

	let (sender1,receiver1) = mpsc::channel(BUF_SIZE);
	let (sender2,receiver2) = mpsc::channel(BUF_SIZE);

	std::thread::spawn(move || {
	    runtime.block_on(async move {
		let mut this = Self {
		    config,
		    recv:receiver1,
		    send:sender2
		};
		let _ = this.run().await;
	    })
	});
	Ok((sender1,receiver2))
    }

    async fn run(&mut self)->Result<()> {
	let url = Url::parse(&self.config.server_url)?;
	let (mut socket,_response) = tt::connect_async(url).await?;

	loop {
	    let _ = tokio::select! {
		Some(payload) = self.recv.recv() => {
		    let sender = Entity::Administrator(self.config.name.clone());
		    let cmd = Envelope {
			sender,
			signature:"\\_'')_/".to_string(),
			payload
		    };
		    let v = serde_json::to_string(&cmd)?;
		    socket.send(Message::Text(v)).await?;
		},
		Some(msg) = socket.next() => {
		    match msg? {
			Message::Text(u) => {
			    let resp : Result<Envelope<Response>,String> =
				serde_json::from_str(&u)
				.map_err(|e| anyhow!("Invalid JSON: {}",e))?;
			    match resp {
				Ok(env) => {
				    let _ = self.send.send(env.payload).await;
				},
				Err(e) => bail!("Error: {}",e)
			    }
			},
			_ => bail!("Invalid message type")
		    }
		}
	    };
	}
    }
}

fn authorize(message_buf:TextBuffer,
	     send_cmd:Ptr<Sender<Command>>,
	     kid:String,t:f64) {
    message_buf.append(
	&format!("Authorize {} for {}",kid,
		 Seconds::make(t)));

    let cmd = 
	Command::Authorize { subject:kid.clone(),
			     duration:Some(t) };
    send_cmd.yank_mut().blocking_send(cmd)
	.expect("Cannot send");
}

trait TextBufferAppend {
    fn append(&self,u:&str);
}

impl TextBufferAppend for TextBuffer {
    fn append(&self,u:&str) {
	let mut end = self.end_iter();
	self.insert(&mut end,u);
	self.insert(&mut end,"\n");
    }
}
    
fn main()->glib::ExitCode {
    let app = Application::builder()
	.application_id(APP_ID)
	.build();

    app.connect_activate(|app| {
	let config = Config::open(CONFIG_PATH)
	    .expect("Cannot open configuration file");

	let (send_cmd,receive_resp) =
	    BackendConnection::new(config.clone())
	    .expect("Cannot start backend connection");

	let send_cmd = Ptr::make(send_cmd);
	let receive_resp = Ptr::make(receive_resp);

	let window = ApplicationWindow::builder()
	    .application(app)
	    .default_width(640)
	    .default_height(512)
	    .title("Discipline")
	    .build();

	let message_buf = TextBuffer::builder()
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

	    let duration_h = Entry::builder()
		.input_purpose(InputPurpose::Number)
		.build();
	    box2.append(&duration_h);
	    let authorize_hours = Button::with_label(" hours");
	    box2.append(&authorize_hours);
	    
	    let cancel = Button::with_label("Cancel");
	    box2.append(&cancel);

	    let get_status = Button::with_label("Get status");
	    box2.append(&get_status);
	    get_status.connect_clicked({
		let send_cmd = send_cmd.refer();
		let kid = kid.clone();
		move |_| {
		    let cmd = Command::GetStatus { subject:kid.clone() };
		    send_cmd.yank_mut().blocking_send(cmd)
			.expect("Cannot send");
		}
	    });

	    frame.set_child(Some(&box2));
	    box1.append(&frame);

	    authorize_hours.connect_clicked({
		let duration_h = duration_h.clone();
		let message_buf = message_buf.clone();
		let send_cmd = send_cmd.refer();
		let kid = kid.clone();
		move |_| {
		    let duration_h_text = duration_h.text();
		    if let Ok(d) = duration_h_text.parse::<f64>() {
			authorize(message_buf.clone(),
				  send_cmd.refer(),
				  kid.clone(),
				  d*3600.0);
		    } else {
			message_buf.append("Invalid number");
		    }
		}
	    });

	    authorize_1h.connect_clicked({
		let message_buf = message_buf.clone();
		let send_cmd = send_cmd.refer();
		let kid = kid.clone();
		move |_| {
		    authorize(message_buf.clone(),
			      send_cmd.refer(),
			      kid.clone(),
			      3600.0);
		}
	    });

	    authorize_30min.connect_clicked({
		let message_buf = message_buf.clone();
		let send_cmd = send_cmd.refer();
		let kid = kid.clone();
		move |_| {
		    authorize(message_buf.clone(),
			      send_cmd.refer(),
			      kid.clone(),
			      1800.0);
		}
	    });

	    cancel.connect_clicked({
		let message_buf = message_buf.clone();
		let send_cmd = send_cmd.refer();
		let kid = kid.clone();
		move |_| {
		    authorize(message_buf.clone(),
			      send_cmd.refer(),
			      kid.clone(),
			      0.0);
		}
	    });

	}

	let messages_window = ScrolledWindow::builder()
	    .hexpand(true)
	    .vexpand(true)
	    .vscrollbar_policy(PolicyType::Always)
	    .build();
	
	let messages = TextView::builder()
	    .editable(false)
	    .hexpand(true)
	    .buffer(&message_buf)
	    .vexpand(false)
	    .vscroll_policy(ScrollablePolicy::Natural)
	    .build();

	message_buf.connect_changed({
	    let messages_window = messages_window.clone();
	    move |_| {
		let adj = messages_window.vadjustment();
		adj.set_value(adj.upper() - adj.page_size());
	    }
	});
	messages_window.set_child(Some(&messages));
	box1.append(&messages_window);
	
	window.set_child(Some(&box1));

	const FPS: u32 = 30;
	glib::source::timeout_add_local(
	    std::time::Duration::from_secs_f64(1.0 / FPS as f64),
	    {
		let _messages_window = messages_window.clone();
		let message_buf = message_buf.clone();
		move || {
		    match receive_resp.yank_mut().try_recv() {
			Ok(resp) => {
			    match resp {
				Response::Authorization {
				    subject,
				    time_remaining,
				    last_ping:_
				} => {
				    message_buf.append(
					&format!(
					    "Subject {} time remaining {}",
					    subject,
					    Seconds::make(time_remaining))
				    );
				},
				Response::Ack => {
				    message_buf.append("Server: Acknowledged");
				},
				Response::Error(e) => {
				    message_buf.append(
					&format!("Server: Error {}",e));
				}
			    }
			},
			Err(TryRecvError::Empty) => (),
			Err(TryRecvError::Disconnected) => {
			    println!("Disconnected!?");
			}
		    }
		    true.into()
		}
	    });

	window.present();
    });

    app.run()
}

struct Seconds(f64);

impl Seconds {
    fn make(t:f64)->Self { Self(t) }
}

impl Display for Seconds {
    fn fmt(&self,o:&mut std::fmt::Formatter<'_>)->Result<(),std::fmt::Error> {
	let t = self.0;
	if t < 0.1 {
	    write!(o,"zero")
	} else {
	    if t < 60.0 {
		let sec = t.round() as isize;
		write!(o,"{} second{}",
		       sec,
		       if sec == 1 { "" } else { "s" })
	    } else if t < 3600.0 {
		let min = (t/60.0).round() as isize;
		write!(o,"{} minute{}",
		       min,
		       if min == 1 { "" } else { "s" })
	    } else {
		let hour = (t/3600.0).round() as isize;
		let min = ((t - 3600.0*(hour as f64))/60.0).round() as isize;
		write!(o,"{} hour{}",
		       hour,
		       if hour == 1 { "" } else { "s" })?;
		if min > 0 {
		    write!(o," {} minute{}",
			   min,
			   if min == 1 { "" } else { "s" })?;
		}
		Ok(())
	    }
	}
    }
}
