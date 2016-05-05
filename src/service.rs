#![allow(dead_code)]
extern crate epoll;

use std::io::prelude::*;
use std::os::unix::io::{AsRawFd, RawFd};
use std::net::{UdpSocket,TcpListener,TcpStream, SocketAddr};
use std::thread;

use spring_dvs::enums::{DvspMsgType, DvspRcode};
use spring_dvs::protocol::{Packet, Ipv4, http_from_bin, http_to_bin};
use spring_dvs::protocol::{FrameResponse, HttpWrapper};
use spring_dvs::serialise::{NetSerial};
use spring_dvs::formats::ipv4_to_str_address;



use netspace::*;

use self::epoll::*;
use self::epoll::util::*;
use ::config::Config;
use ::unit_test_env::setup_live_test_env;

//pub use spring_dvs::enums::{Success, Failure};

use ::protocol::process_packet;


pub struct Tcp;
pub struct Dvsp;
/*
 * ToDo:
 * There is not timeout handling going on
 * The server could potentially hang until
 * fossil fuel runs out.
 */

impl Dvsp {
	pub fn start(config: &Config) -> Result<Success,Failure> {
		
		let socket = match UdpSocket::bind("0.0.0.0:55301") {
				Ok(s) => s,
				Err(_) => return Err(Failure::InvalidArgument)
		};
		
		let epfd = epoll::create1(0).unwrap();
		let sfd = socket.as_raw_fd();
	
		let mut event = EpollEvent {
			data: sfd as u64,
			//events: (event_type::EPOLLIN | event_type::EPOLLET | event_type::EPOLLRDHUP)
			events: (event_type::EPOLLIN | event_type::EPOLLRDHUP)
		};
		
		match epoll::ctl(epfd, ctl_op::ADD, sfd, &mut event) {
			Ok(()) => { },
			Err(e) => println!("[Error] CtlError on add: {}", e)
		};
		
		let cfg_clone = config.clone();
		
		let _ = thread::spawn(move|| {
			
			Dvsp::epoll_wait(epfd, socket, cfg_clone);	    
		});
	
		
		
		Ok(Success::Ok)
	}
	
	
	
	
	fn epoll_wait(epfd: RawFd, socket: UdpSocket, config: Config) {
	
		let mut bytes = [0;768];
	
		let mut events = Vec::<EpollEvent>::with_capacity(100);
	  
	    unsafe { events.set_len(100); }
	    
	    let nio = match config.live_test {
			false => {
				println!("[Alert] Live System");
				NetspaceIo::new("gsn.db") 
			},
			true => {
				println!("[Alert] Warning: Live testing enabled; using testing database");
				let nio = NetspaceIo::new("live-testing.db");
				
				setup_live_test_env(&nio, &config);
				nio
			
			}
		};
		
	    println!("[Service] UDP Service Online");
	    loop {
		    match epoll::wait(epfd, &mut events[..], -1) {
		
		        Ok(num_events) => {
		            
		            
		            for _ in 0..num_events {
		
		       			let (sz, from) = match socket.recv_from(&mut bytes) {
							Err(_) => return,
							Ok(s) => s
						};
		
	
		
		            	let bytes = process_packet(&bytes[0..sz], &from, config, &nio);
		            	match socket.send_to(bytes.as_slice(), from) {
		            		Err(_) => return,
							_ => { }
		            	};
		
		            }
		        }
		
		        Err(e) => println!("[Error] Error on epoll::wait(): {}", e)
			}
	    }
	}

}


impl Tcp {

	pub fn start(cfg: &Config) -> Result<Success,Failure> {
		
		let listener = TcpListener::bind("0.0.0.0:55300").unwrap();

		let config = cfg.clone();
		

		let s = thread::spawn(move|| {
				
			let nio = match config.live_test {
				false => {
					NetspaceIo::new("gsn.db") 
				},
				true => {
					
					let nio = NetspaceIo::new("live-testing.db");
					nio
				}
			};			


			println!("[Service] TCP Service Online");
			for stream in listener.incoming() {
				
				match stream {
					Ok(mut stream) => {
	
						let mut buf = [0;4096];
						
						let mut address = match stream.peer_addr() {
							Ok(a) => a,
							Err(_) => continue
						};
						
						let size = match stream.read(&mut buf) {
							Ok(s) => s,
							Err(_) => 0
						};
						
						
						if size > 4 {
							
							
							let out : Vec<u8> = Tcp::handle_request(&buf[0..size], &mut address, &config, &nio);
	
							stream.write(out.as_slice()).unwrap();
	
						}
	
					},
					Err(_) => { }
				}
			}	    
		});
		
		match s.join() {
			Ok(_) => { },
			_ => println!("[Error] Error on TCP thread join"),
		}	
		Ok(Success::Ok)
		
	}
	
	pub fn handle_request(bytes: &[u8], address: &mut SocketAddr, config: &Config, nio: &NetspaceIo) -> Vec<u8> {
		let check = &bytes[0..4];
		
		if &check == &"POST".as_bytes() {
			// Here sort it as an HTTP service layer
			match HttpWrapper::deserialise_request(Vec::from(bytes), address) {
				Ok(bytes_in) => {
					let bytes = process_packet(&bytes_in, &address, config.clone(), &nio);
					return HttpWrapper::serialise_response_bytes(&bytes)
				},
				Err(_) => return HttpWrapper::serialise_response(
										&Packet::from_serialisable(
											DvspMsgType::GsnResponse, 
											&FrameResponse::new(DvspRcode::MalformedContent)
										).unwrap())
			};
		}
		
		// Here we handle a straight DVSP TCP stream
		process_packet(&bytes, &address, config.clone(), &nio)
		
		
	}

	
	pub fn make_request(packet: &Packet, address: &Ipv4, host: &str, resource: &str, service: DvspService) -> Result<Packet,Failure> {
		
		let (addr, serial) = match service {
			DvspService::Http => (
				 	format!("{}:80", ipv4_to_str_address(address)),
				 	HttpWrapper::serialise_request(packet, host, resource)
			),
			_ => (
				format!("{}:55300", ipv4_to_str_address(address)),
				packet.serialise()
			)
		};
		
		let mut stream = match TcpStream::connect(addr.as_str()) {
			Ok(s) => s,
			Err(_) => return Err(Failure::InvalidArgument)
		};
		
		stream.write(serial.as_slice()).unwrap();
		let mut buf = [0;4096];

		let size = match stream.read(&mut buf) {
					Ok(s) => s,
					Err(_) => 0
		};
		
		if size == 0 { return Err(Failure::InvalidArgument) }
		
		if service == DvspService::Http {
			
			match HttpWrapper::deserialise_response(Vec::from(&buf[0..size])) {
				Ok(bytes) => Packet::deserialise(&bytes),
				Err(_) => Err(Failure::InvalidConversion)
			}
		} else {
			Packet::deserialise(&buf[0..size])
		}
	}  
}

// ToDo clean this lot up -- better failure states
pub fn chain_request(bytes: Vec<u8>, target: &Node) -> Result<Vec<u8>, Failure> {
	
	// ToDo: Handle HTTP service layers
	let address : String = match target.service() {
		DvspService::Dvsp => format!("{}:55301", ipv4_to_str_address(&target.address())),
		_ => return Err(Failure::InvalidArgument)
	};
	
	let socket = match UdpSocket::bind("0.0.0.0:0") {
			Ok(s) => s,
			Err(_) => return Err(Failure::InvalidArgument)
	};
	
	match socket.send_to(bytes.as_ref(), address.as_str()) {
		Ok(_) =>{ },
		_ => return Err(Failure::InvalidArgument),
	}
	
	let mut buf = [0;768];
	let (sz, _) = match socket.recv_from(&mut buf) {
		Ok(t) => t,
		_ => { return Err(Failure::InvalidArgument) }
	};
	
	Ok(Vec::from(&buf[0..sz]))
	
}


mod tests {
	extern crate spring_dvs;
	
	#[allow(unused_imports)]
	use super::*;
	
}

