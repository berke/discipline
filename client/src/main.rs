use serde::{
    Deserialize,
    Serialize
};
use url::Url;
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
    connect,
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

fn main()->Result<()> {
    let progname : String = std::env::args().nth(0).unwrap();
    
    let mut args = Arguments::from_env();

    if args.contains("-h") || args.contains("--help") {
	eprintln!("Usage: {} [--url ws://ADDR:PORT/]",
		  progname);
	return Ok(())
    }

    let sender =
	if let Some(u) = args.opt_value_from_str("--sender-subject")? {
	    Entity::Subject(u)
	} else {
	    if let Some(u) = args.opt_value_from_str("--sender-admin")? {
		Entity::Administrator(u)
	    } else {
		bail!("Specify --sender-subject or --sender-admin")
	    }
	};

    let subject : String = args.value_from_str("--subject")?;

    let retry_delay : Option<f64> = args.opt_value_from_str("--retry-delay")?;
    let loop_delay : Option<f64> = args.opt_value_from_str("--loop-delay")?;

    let authorize_for : Option<f64> =
	args.opt_value_from_str("--authorize-for")?;
    let get_status = args.contains("--get-status");
    let show_time_remaining = args.contains("--show-time-remaining");

    let url = args.opt_value_from_str("--url")?
	.unwrap_or_else(|| "ws://127.0.0.1:9001".to_string());

    let url = Url::parse(&url)?;

    let rest = args.finish();
    if !rest.is_empty() {
	bail!("Invalid arguments: {:?}",rest);
    }

    let process = move ||->Result<()> {
	let (mut socket,response) = connect(url.clone())?;
	// eprintln!("Response: {:#?}",response);

	let mut transact = |payload:Command|->Result<Envelope<Response>> {
	    let sender = sender.clone();
	    let cmd = Envelope {
		sender,
		signature:"\\_'')_/".to_string(),
		payload
	    };
	    let v = serde_json::to_string(&cmd)?;
	    socket.send(Message::Text(v))?;
	    let msg = socket.read()?;
	    match msg {
		Message::Text(u) => {
		    let resp : Result<Envelope<Response>,String> = serde_json::from_str(&u)
			.map_err(|e| anyhow!("Invalid JSON: {}",e))?;
		    match resp {
			Ok(env) => Ok(env),
			Err(e) => bail!("Error: {}",e)
		    }
		},
		_ => bail!("Invalid message type")
	    }
	};

	loop {
	    let subject = subject.clone();
	    if let Some(d) = authorize_for {
		let env = transact(Command::Authorize { subject,duration:Some(d) })?;
		match &env.payload {
		    Response::Ack => (),
		    Response::Error(e) => bail!("Remote error: {}",e),
		    _ => bail!("Unexpected response")
		}
	    } else if get_status || show_time_remaining {
		let env = transact(Command::GetStatus { subject })?;
		match &env.payload {
		    Response::Authorization {
			subject,
			time_remaining,
			last_ping
		    } => {
			if show_time_remaining {
			    println!("{}",time_remaining.round() as isize);
			} else {
			    println!("{:#?}",env.payload);
			}
		    },
		    Response::Error(e) => bail!("Remote error: {}",e),
		    _ => bail!("Unexpected response")
		}
	    } else {
		bail!("Specify --get-status, --show-time-remaining or --authorize-for")
	    }

	    if let Some(d) = loop_delay {
		std::thread::sleep(std::time::Duration::from_secs_f64(d))
	    } else {
		break;
	    }
	}

	socket.close(None)?;

	Ok(())
    };

    loop {
	match process() {
	    Err(e) => eprintln!("Error: {}",e),
	    Ok(()) => ()
	}

	if let Some(d) = retry_delay {
	    eprintln!("Waiting for retry...");
	    std::thread::sleep(std::time::Duration::from_secs_f64(d))
	} else {
	    break;
	}
    }

    Ok(())
}
