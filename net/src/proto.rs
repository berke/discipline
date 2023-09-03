use serde::{Serialize,Deserialize};
use nix::sys::socket as nss;

#[derive(Debug,Serialize,Deserialize,Copy,Clone,PartialEq,PartialOrd,Ord,Eq)]
pub struct SubscriberId {
    pub addr:u32,
    pub port:u16
}

#[derive(Debug,Serialize,Deserialize,Clone,PartialEq,PartialOrd,Ord,Eq)]
pub struct ChannelId {
    id:String
}

#[derive(Debug,Serialize,Deserialize)]
pub struct Message {
    pub source:String,
    pub timestamp:f64,
    pub contents:Vec<u8>
}

#[derive(Debug,Serialize,Deserialize)]
pub enum Control {
    Subscriptions {
	channels:Vec<ChannelId>
    },
    Transmit {
	channel:ChannelId,
	message:Message
    },
    Error {
	msg:String
    }
}

impl From<nss::SockaddrIn> for SubscriberId {
    fn from(a:nss::SockaddrIn)->Self {
	Self {
	    addr:a.ip(),
	    port:a.port()
	}
    }
}
