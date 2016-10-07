extern crate unix_socket;
use self::unix_socket::UnixStream;
use std::io::prelude::*;
use std::mem;
use netspace::{NetspaceIo,Config};

#[macro_export]
macro_rules! cascade_none_nowrap {
	($opt: expr) => (
		match $opt {
			Some(s) => s,
			_ => return None,
		}
	)
}


mod network;
mod validation;

use self::validation::ValidationZone;
use self::network::NetworkZone;

fn binary_split(msg: &str) -> Vec<&str> {
	msg.splitn(2, " ").collect()
}




pub fn management_handler(mut stream: UnixStream, config: Config) {
	
	let nio = match config.live_test {
		false => {
			NetspaceIo::new("/var/lib/springdvs/gsn.db") 
		},
		true => {
			NetspaceIo::new("live-testing.db")
		}
	};

	let mut szin_buf = [0;4];
	
	stream.read_exact(&mut szin_buf).unwrap();
	
	let szin : u32 = unsafe { mem::transmute(szin_buf) };
	
	let mut bufin : Vec<u8> = Vec::new();
	bufin.resize(szin as usize, b'\0');
	stream.read_exact(bufin.as_mut_slice()).unwrap();
	let command = String::from_utf8(bufin).unwrap();
	
	let mi = ManagementInstance::new();
	
	let out = match mi.run(&command, &nio) {
		Some(s) => s,
		None => "Error: Unrecognised or malformed command".to_string() 
	};
	stream.write_all(out.as_bytes()).unwrap();
}

struct ManagementInstance;

impl ManagementInstance {
	pub fn new() -> Self {
		ManagementInstance
	}
	pub fn run(&self, command: &str, nio: &NetspaceIo) -> Option<String> {
		self.process_request(cascade_none_nowrap!(ManagementZone::from_str(command)), nio)
	}

	pub fn process_request(&self, request: ManagementZone, nio: &NetspaceIo) -> Option<String> {
		match request {
			ManagementZone::Network(nz) => NetworkZone::process(nz, nio),
			ManagementZone::Validation(vz) => ValidationZone::process(vz, nio)
		}
	}
}

#[derive(Clone, PartialEq, Debug)]
pub enum ManagementZone {
	Network(network::NetworkZone), Validation(validation::ValidationZone)
}

impl ManagementZone {
	pub fn from_str(msg: &str) -> Option<ManagementZone> {
		if msg.len() == 0 { return None; }
		
		let atom = binary_split(msg);
		
		Some(match atom[0] {
			"network" => {
				ManagementZone::Network(cascade_none_nowrap!(NetworkZone::from_str(atom[1])))				
			},
			"validation" => {
				ManagementZone::Validation(cascade_none_nowrap!(ValidationZone::from_str(atom[1])))
			}
			_ => return None
		})
		
	}
}