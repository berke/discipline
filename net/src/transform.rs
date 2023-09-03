use nix::sys::socket as nss;
use nix::fcntl as nfc;
use std::os::unix::io::RawFd;
use std::sync::{Arc,Mutex};

use crate::common::*;

pub trait Transformer {
    fn decode(&mut self,u:&[u8],buf:&mut [u8])->Option<usize>;
    fn encode(&mut self,u:&[u8],buf:&mut [u8])->Option<usize>;
}

mod core {
    const N : usize = 32;

    fn g(xy:(u32,u32),k:[u32;4])->(u32,u32) {
	fn f(r1:u32,r2:u32,x:u32,k0:u32,k1:u32)->u32 {
	    x.rotate_left(r1).wrapping_add(k0).rotate_left(r2) ^ k1
	}
	let mut x = xy.0;
	let mut y = xy.1;
	x ^= f(20,13,y,k[0].wrapping_add(0xea90b2d4),k[1].wrapping_add(0x779e0f9a));
	y ^= f(12, 5,x,k[1].wrapping_add(0x57828eb3),k[2].wrapping_add(0x7ae4d78c));
	x ^= f( 9,24,y,k[2].wrapping_add(0x1d116cad),k[3].wrapping_add(0x9fe73bfe));
	y ^= f(18, 9,x,k[3].wrapping_add(0xbaf13cfd),k[0].wrapping_add(0x8e062e91));
	(x,y)
    }

    pub fn h(xy0:(u32,u32),k:[u32;4])->(u32,u32) {
	let mut xy = xy0;
	let mut kc = k;
	for _r in 0..N-1 {
	    kc[0] = kc[0].wrapping_add(0x992fd18e);
	    kc[1] = kc[1].wrapping_add(0x661a75e3);
	    kc[2] = kc[2].wrapping_add(0xea02e721);
	    kc[3] = kc[3].wrapping_add(0x078a322f);
	    xy = g(xy,kc);
	}
	xy
    }
}

pub struct BlockTransform {
    k:[u32;4],
    ctr:u64
}

fn getiv()->u64 {
    let fd = nfc::open("/dev/urandom",nfc::OFlag::O_RDONLY,nix::sys::stat::Mode::S_IRWXU).unwrap();
    let mut buf = [0;8];
    let mut i = 0;
    loop {
	if i == buf.len() {
	    break
	} else {
	    let p = nix::unistd::read(fd,&mut buf[i..]).unwrap();
	    if p > 0 {
		i += p;
	    }
	}
    }
    u64::from_le_bytes(buf)
}

impl BlockTransform {
    pub fn new(k:[u32;4])->Self {
	BlockTransform{
	    k,
	    ctr:getiv()
	}
    }
}

fn split2x32(p:[u8;8])->(u32,u32) {
    let mut a = [0_u8;4];
    a.copy_from_slice(&p[0..4]);
    let x = u32::from_le_bytes(a);
    a.copy_from_slice(&p[4..8]);
    let y = u32::from_le_bytes(a);
    (x,y)
}

fn join2x32(x:u32,y:u32)->[u8;8] {
    let mut a = [0_u8;8];
    a[0..4].copy_from_slice(&x.to_le_bytes());
    a[4..8].copy_from_slice(&y.to_le_bytes());
    a
}

impl Transformer for BlockTransform {
    fn encode(&mut self,u:&[u8],buf:&mut [u8])->Option<usize> {
	let m = u.len();
	let nblk = m / 8;
	let mut c = self.ctr & !(7 << 61);
	let cc = (c << 3) | (m & 7) as u64;
	let mut j = 0;
	let mut get = |a:&mut [u8]| {
	    a.copy_from_slice(&u[j..j+a.len()]);
	    j += a.len();
	};
	let mut k = 0;
	let mut put = |v:&[u8]| {
	    let n = v.len();
	    buf[k..k+n].copy_from_slice(v);
	    k += n;
	};
	put(&cc.to_le_bytes());
	let mut axy = ((c >> 32) as u32, (c & 0xffffffff) as u32);
	// println!("C={:016X}",c);
	let mut blk = |p:[u8;8]| {
	    let (px,py) = split2x32(p);
	    let cx = (c >> 32) as u32;
	    let cy = (c & 0xffffffff) as u32;
	    let (kx,ky) = core::h((cx,cy),self.k);
	    let ex = kx ^ px;
	    let ey = ky ^ py;
	    // println!("E {:08X} {:08X} | K {:08X} {:08X} | P {:08X} {:08X}",ex,ey,kx,ky,px,py);
	    axy = core::h((axy.0^ex,axy.1^ey),self.k);
	    put(&join2x32(ex,ey));
	    c += 1;
	};
	let mut a = [0;8];
	for _ in 0..nblk {
	    get(&mut a);
	    blk(a);
	}
	if m & 7 != 0 {
	    get(&mut a[0..m&7]);
	    for j in m&7..8 {
		a[j] = 0;
	    }
	    blk(a);
	}
	put(&join2x32(axy.0,axy.1));
	self.ctr = c;
	// println!("ENCODE m={} -> k={}",m,k);
	Some(k)
    }

    fn decode(&mut self,u:&[u8],buf:&mut [u8])->Option<usize> {
	let mut a = [0;8];
	let m0 = u.len();
	// println!("DECODE m0={}",m0);
	if m0 < 24 || m0 & 7 != 0 {
	    return None;
	}
	let mut j = 0;
	let mut get = |a:&mut [u8]| {
	    let n = a.len();
	    a.copy_from_slice(&u[j..j+n]);
	    j += n;
	};
	let mut k = 0;
	let mut put = |v:&[u8]| {
	    let n = v.len();
	    buf[k..k+n].copy_from_slice(v);
	    k += n;
	};
	get(&mut a);
	let cc = u64::from_le_bytes(a);
	let mut c = cc >> 3;
	// println!("C={:016X}",c);
	let m =
	    if cc & 7 == 0 {
		m0 - 16
	    } else {
		m0 - 24 + (cc & 7) as usize
	    };
	let nblk = m / 8;
	let mut axy = ((c >> 32) as u32, (c & 0xffffffff) as u32);
	// println!("m0={} m={} nblk={}",m0,m,nblk);
	let mut blk = |p:[u8;8]| {
	    let (ex,ey) = split2x32(p);
	    axy = core::h((axy.0^ex,axy.1^ey),self.k);
	    let cx = (c >> 32) as u32;
	    let cy = (c & 0xffffffff) as u32;
	    c += 1;
	    let (kx,ky) = core::h((cx,cy),self.k);
	    let px = kx ^ ex;
	    let py = ky ^ ey;
	    // println!("E {:08X} {:08X} | K {:08X} {:08X} | P {:08X} {:08X}",ex,ey,kx,ky,px,py);
	    join2x32(px,py)
	};
	let mut a = [0;8];
	for _ in 0..nblk {
	    get(&mut a);
	    put(&blk(a));
	}
	if m & 7 != 0 {
	    get(&mut a);
	    let b = blk(a);
	    put(&b[0..m&7]);
	}
	get(&mut a);
	let axy2 = split2x32(a);
	if axy != axy2 {
	    return None;
	} else {
	    Some(m)
	}
    }
}

#[test]
fn test_block_transform() {
    let mut xfo = BlockTransform::new();
    let mut u = [0_u8;320];
    for i in 0..u.len() {
	u[i] = (i & 255) as u8;
    }
    println!("u:");
    hex_dump(&u);
    let mut v = [0_u8;1024];
    match xfo.encode(&u,&mut v) {
	Ok(n) =>
	    {
		println!("v:");
		hex_dump(&v[0..n]);
		let mut w = [0_u8;1024];
		match xfo.decode(&v[0..n],&mut w) {
		    Ok(p) => {
			println!("n={} p={}",n,p);
			if u.len() != p {
			    panic!("Length error");
			}
			if &u[0..p] != &w[0..p] {
			    panic!("Mismatch");
			}
			println!("w:");
			hex_dump(&w[0..p]);
		    },
		    Err(_) => panic!("Decoding error")
		}
	    },
	Err(_) => panic!("Decoding error")
    }
}
