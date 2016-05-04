/* Notice:  Copyright 2016, The Care Connections Initiative c.i.c.
 * Author:  Charlie Fyvie-Gauld (cfg@zunautica.org)
 * License: GPLv3 (http://www.gnu.org/licenses/gpl-3.0.txt)
 */
#![allow(unused_imports)]
extern crate spring_dvs;

use spring_dvs::model::Netspace;
use spring_dvs::protocol::*;


mod netspace;
mod service;
mod protocol;
mod config;
mod resolution;
mod node_config;
mod requests;
mod unit_test_env;

fn main() {
    println!("Spring GSN Root Node\n{}.{}.uk", node_config::node_springname(), node_config::node_geosub());
    let mut config = config::Config::new();
    
    
    config.live_test = true;
    match service::Dvsp::start(&config) {
    	Ok(_) => println!("Service OK"),
    	Err(_) => println!("Service finished with error"),
    }
    
    match service::Tcp::start(&config) {
    	Ok(_) => println!("Service OK"),
    	Err(_) => println!("Service finished with error"),
    }
}
