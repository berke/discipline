mod ptr;

use anyhow::{
    bail,
    Result
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

const APP_ID : &str = "fr.exhrd.Discipline";

mod config {
    use anyhow::Result;
    use std::path::Path;
    use serde::Deserialize;

    #[derive(Debug,Deserialize)]
    pub struct Config {
	pub server_url:String,
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
    
fn main()->glib::ExitCode {
    let app = Application::builder()
	.application_id(APP_ID)
	.build();

    app.connect_activate(|app| {
	let config = Config::open(CONFIG_PATH)
	    .expect("Cannot open configuration file");
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
