#![allow(dead_code)]
use std::time::Instant;

pub struct Valve {
    pub mask:usize,
    start:Instant,
    last:Instant,
    threshold:f64,
    counter:usize
}

impl Valve {
    pub fn new(threshold:f64)->Self {
	let t = Instant::now();
        Self{
            mask:1,
	    start:t,
            last:t,
            threshold,
	    counter:0
        }
    }

    pub fn elapsed(&self)->f64 {
	(self.last - self.start).as_secs_f64()
    }

    pub fn tick(&mut self)->Option<f64> {
	let c = self.counter;
	self.counter += 1;
	if c & self.mask == 0 {
	    let now = Instant::now();
	    let dur = now.duration_since(self.last);
	    let dt = dur.as_secs_f64();
	    if dt > 2.0 * self.threshold {
		self.mask >>= 1;
	    } else if dt < self.threshold / 2.0 {
		self.mask = self.mask.wrapping_shl(1) | 1;
	    }
	    if dt >= self.threshold {
		self.last = now;
		Some(now.duration_since(self.start).as_secs_f64())
	    } else {
		None
	    }
	} else {
	    None
	}
    }
}
