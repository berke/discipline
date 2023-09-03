mod common;
mod transform;
mod proto;

use nix::sys::socket as nss;
use nix::fcntl as nfc;
use std::os::unix::io::RawFd;
use std::sync::{Arc,Mutex};
use std::net::SocketAddrV4;
use std::os::fd::{OwnedFd,AsRawFd};
use pico_args::Arguments;
use std::collections::{BTreeMap,BTreeSet,VecDeque};

use crate::common::*;
use crate::proto::*;
use transform::{BlockTransform,Transformer};

fn hex_dump(u:&[u8]) {
    let m = u.len();
    for i in 0..m {
	if i & 15 == 0 {
	    if i != 0 {
		println!("");
	    }
	    print!("{:08x}",i);
	}
	print!(" {:02x}",u[i]);
    }
    println!("");
}

struct Subscriber {
    t_last:f64,
    channels:BTreeSet<ChannelId>
}

impl Subscriber {
    pub fn new()->Self {
	Self {
	    t_last:timestamp(),
	    channels:BTreeSet::new()
	}
    }
}

struct Switchboard {
    subscribers:BTreeMap<SubscriberId,Subscriber>,
    channels:BTreeMap<ChannelId,Channel>
}

struct Channel {
    backlog:VecDeque<Message>,
    subscribers:BTreeSet<SubscriberId>
}

impl Channel {
    const BACKLOG_MAX : usize = 128;

    pub fn new()->Self {
	Self {
	    backlog:VecDeque::new(),
	    subscribers:BTreeSet::new()
	}
    }

    pub fn add(&mut self,message:Message) {
	let m = self.backlog.len();
	if m >= Self::BACKLOG_MAX {
	    let _ = self.backlog.pop_front();
	}
	self.backlog.push_back(message);
    }
}

fn timestamp()->f64 {
    std::time::SystemTime::now()
	.duration_since(std::time::SystemTime::UNIX_EPOCH)
	.expect("Cannot get timestamp")
	.as_secs_f64()
}

impl Switchboard {
    pub fn new()->Self {
	Self {
	    subscribers:BTreeMap::new(),
	    channels:BTreeMap::new(),
	}
    }

    pub fn unsubscribe(&mut self,channel:&ChannelId,
		       subscriber:&SubscriberId) {
	let mut e = self.channels.entry(channel.clone())
	    .or_insert_with(|| Channel::new());
	e.subscribers.remove(subscriber);
	let mut f =
	    self.subscribers.entry(subscriber.clone())
	    .or_insert_with(|| Subscriber::new());
	f.channels.remove(channel);
    }

    pub fn channel_mut(&mut self,channel:&ChannelId)->&mut Channel {
	self.channels.entry(channel.clone())
	    .or_insert_with(|| Channel::new())
    }

    pub fn subscriber_mut(&mut self,subscriber:&SubscriberId)->&mut Subscriber {
	self.subscribers.entry(subscriber.clone())
	    .or_insert_with(|| Subscriber::new())
    }

    pub fn subscribe(&mut self,channel:&ChannelId,
		     subscriber:&SubscriberId) {
	self.channel_mut(channel).subscribers.insert(subscriber.clone());
	self.subscriber_mut(subscriber).channels.insert(channel.clone());
    }

    pub fn process(&mut self,
		   subscriber:SubscriberId,
		   buf:&[u8])->Res<()> {
	let ctrl : Control = rmp_serde::decode::from_slice(&buf)?;
	
	match ctrl {
	    Control::Subscriptions{ channels } => {
		let existing = self.subscriber_mut(&subscriber).channels.clone();
		for chan in existing.iter() {
		    self.unsubscribe(chan,&subscriber);
		}
		for chan in channels.iter() {
		    self.subscribe(chan,&subscriber);
		}
	    },
	    Control::Transmit { channel,message } => {
		let mut chan = self.channel_mut(&channel);
		chan.add(message);
	    },
	    Control::Error { msg } => {
		eprintln!("Error from {:?}: {:?}",subscriber,msg);
	    }
	}
	Ok(())
    }
}

fn main()->Res<()> {
    let mut args = Arguments::from_env();

    let src_addr : SocketAddrV4 = args.value_from_str("--addr")?;
    let sock_fd : OwnedFd = nss::socket(nss::AddressFamily::Inet,
			   nss::SockType::Datagram,
			   nss::SockFlag::empty(),
			   nss::SockProtocol::Udp)?;
    let sock_fd_raw = sock_fd.as_raw_fd();
    nss::setsockopt(&sock_fd,nss::sockopt::ReuseAddr,&true)?;
    let src_addr_in : nss::SockaddrIn = src_addr.into();
    nss::bind(sock_fd_raw,&src_addr_in)?;

    let mut xfo = BlockTransform::new([0x12931133,0x94813456,0x19293456,0x9911aacc]);

    let mut switchboard = Switchboard::new();

    let mut buf = [0_u8;4096];
    let mut buf2 = [0_u8;4096];
    loop {
	match nss::recvfrom::<nss::SockaddrIn>(sock_fd_raw,&mut buf) {
	    Err(e) => {
		println!("Error receiving: {:?}",e);
		break;
	    },
	    Ok((m,ao)) => {
		match ao {
		    None => println!("No address!"),
		    Some(a) => {
			let ca : SubscriberId = a.into();
			match xfo.decode(&buf[0..m],&mut buf2) {
			    Some(m2) => {
				match switchboard.process(ca,&buf2[0..m2]) {
				    Ok(()) => (),
				    Err(e) => {
					eprintln!("Switchboard error: {}",e);
				    }
				}
			    },
			    None => {
				eprintln!("Transformation error");
				hex_dump(&buf[0..m]);
			    }
			}
		    }
		}
	    }
	}
    }
    Ok(())
}
