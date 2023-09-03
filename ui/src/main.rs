mod common;

use gtk4 as gtk;

use gtk::{
    prelude::*,
    glib,
    Application,
    ApplicationWindow
};

const APP_ID : &str = "fr.exhrd.Discipline";

mod config {
    use crate::common::*;

    use std::path::Path;
    use serde::Deserialize;

    #[derive(Debug,Deserialize)]
    pub struct Config {
	family:Family
    }

    #[derive(Debug,Deserialize)]
    pub struct Family {
	pub husband:Endpoint,
	pub wife:Endpoint,
	pub kids:Vec<Endpoint>
    }

    #[derive(Debug,Deserialize)]
    pub struct Endpoint {
	/// Identifier
	pub name:String,

	/// IP:PORT
	pub addr:String
    }

    impl Config {
	pub fn open<P:AsRef<Path>>(path:P)->Res<Self> {
	    let fd = std::fs::File::open(path)?;
	    let this : Self = ron::de::from_reader(fd)?;
	    Ok(this)
	}
    }
}

use config::Config;

const CONFIG_PATH : &str = "etc/discipline.cfg";
    
fn main()->glib::ExitCode {
    let config = Config::open(CONFIG_PATH)
	.expect("Cannot open configuration file");
    let app = Application::builder()
	.application_id(APP_ID)
	.build();

    app.connect_activate(|app| {
	let window = ApplicationWindow::builder()
	    .application(app)
	    .default_width(640)
	    .default_height(512)
	    .title("Discipline")
	    .build();
	
	window.present();
    });

    app.run()
}
