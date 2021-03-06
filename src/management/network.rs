use std::str::Split;

use netspace::*;

use prettytable::Table;
use prettytable::row::Row;
use prettytable::cell::Cell;



#[macro_export]
macro_rules! extract_zone_network {
	($e: expr) => (
		match $e {
			ManagementZone::Network(s) => s,
			e => panic!("extract_zone_network -- Unexpected value: {:?}", e) 
		}
	)
}


#[derive(Copy, Clone, PartialEq, Debug)]
pub enum NetworkAction {
	View,
	Remove,
	Update,
}

#[derive(Clone, PartialEq, Debug)]
pub enum NetworkOperand {
	None,
	All,
	Node(String),
	Role(NodeRole),
	State(NodeState),
	Service(NodeService),
	Host(String),
	Address(String),
}

#[derive(Clone, PartialEq, Debug)]
pub struct NetworkZone {
	action: NetworkAction,
	op1: NetworkOperand,
	op2: NetworkOperand,
}

impl NetworkZone {
	pub fn new(action: NetworkAction, op1: NetworkOperand, op2: NetworkOperand) -> NetworkZone {
		NetworkZone {
			action: action,
			op1: op1,
			op2: op2
		}
	}

	pub fn from_str(msg: &str) -> Option<NetworkZone> {
		if msg.len() == 0 { return None; }
		
		let mut atom = msg.split(" ");
		
		let action = match atom.next() {
			Some("view") => NetworkAction::View,
			Some("rem") | Some("remove") => NetworkAction::Remove,
			Some("upd") | Some("update") => NetworkAction::Update,
			_ => return None,
		};

		let op1 = match cascade_none_nowrap!(NetworkZone::extract_operand(&mut atom)) {
			NetworkOperand::None => return None,
			op => op
		};
		
		let op2 = cascade_none_nowrap!(NetworkZone::extract_operand(&mut atom));
		Some(NetworkZone::new(action, op1, op2))
	}
	
	fn extract_operand(atom: &mut Split<&str>) -> Option<NetworkOperand> {
		
		Some(match atom.next() {
			Some("all") =>
						NetworkOperand::All,

			Some("node") =>
						NetworkOperand::Node(
							String::from(
								cascade_none_nowrap!(atom.next())
							)
						),

			Some("springname") =>
						NetworkOperand::Node(
							String::from(
								cascade_none_nowrap!(atom.next())
							)
						),

			Some("role") =>
						NetworkOperand::Role(
							cascade_none_nowrap!(
								NodeRole::from_str(
									cascade_none_nowrap!(atom.next())
								)
							)
						),

			Some("state") =>
						NetworkOperand::State(
							cascade_none_nowrap!(
								NodeState::from_str(
									cascade_none_nowrap!(atom.next())
								)
							)
						),

			Some("service") =>
						NetworkOperand::Service(
							cascade_none_nowrap!(
								NodeService::from_str(
									cascade_none_nowrap!(atom.next())
								)
							)
						),

			Some("hostname") =>
						 NetworkOperand::Host(
							String::from(
								cascade_none_nowrap!(atom.next())
							)
						),
			
			Some("address") =>
						NetworkOperand::Address(
							String::from(
								cascade_none_nowrap!(atom.next())
							)
						),

			_ => NetworkOperand::None,
		})
	}
	
	pub fn process(nz: NetworkZone, nio: &Netspace) -> Option<String> {
		match nz.action {
			NetworkAction::View => NetworkZoneModel::view(nz.op1, nio),
			NetworkAction::Update => NetworkZoneModel::update(nz.op1, nz.op2, nio),
			NetworkAction::Remove => NetworkZoneModel::remove(nz.op1, nio),
		}
	}
}

struct NetworkZoneModel;
	
impl NetworkZoneModel {
	pub fn view(op: NetworkOperand, nio: &Netspace) -> Option<String> {
		match op {
			NetworkOperand::All =>
				Some( Self::tabulate_nodes(&nio.gsn_nodes()) ),

			NetworkOperand::Node(s) =>
				Some( Self::tabulate_node(nio.gsn_node_by_springname(&s)) ),
				
			NetworkOperand::Host(s) =>
				Some( Self::tabulate_node(nio.gsn_node_by_hostname(&s)) ),

			NetworkOperand::Role(r) =>
				Some( Self::tabulate_nodes(&nio.gsn_nodes_by_type(r)) ),
				
			NetworkOperand::State(s) =>
				Some( Self::tabulate_nodes(&nio.gsn_nodes_by_state(s)) ),
				
			NetworkOperand::Address(a) =>
				Some( Self::tabulate_nodes(&nio.gsn_nodes_by_address(&a)) ),

			_ => None
		}
		
	}
	
	pub fn update(target: NetworkOperand, value: NetworkOperand, nio: &Netspace) -> Option<String> {
		let mut v : Vec<String> = Vec::new();
		
		match target {
			NetworkOperand::All => {
				for node in nio.gsn_nodes() {
					v.push(Self::update_node(Ok(node), value.clone(), nio)) 
				}
			},
			
			NetworkOperand::Node(s) => {
				let node = nio.gsn_node_by_springname(&s);
				v.push(Self::update_node(node, value.clone(), nio))
			},

			NetworkOperand::Role(r) => {
				for node in nio.gsn_nodes_by_type(r) {
					v.push(Self::update_node(Ok(node), value.clone(), nio)) 
				}
			},

			NetworkOperand::State(s) => {
				for node in nio.gsn_nodes_by_state(s) {
					v.push(Self::update_node(Ok(node), value.clone(), nio)) 
				}
			},

			NetworkOperand::Address(a) => {
				for node in nio.gsn_nodes_by_address(&a) {
					v.push(Self::update_node(Ok(node), value.clone(), nio)) 
				}
			},

			_ => return None
		}
		
		Some(format!("{}\n",v.join("\n")))
	}
	
	fn update_node(node_result: Result<Node, NetspaceFailure>, value: NetworkOperand, nio: &Netspace ) -> String {
		
		let mut node = match node_result {
			Ok(n) => n,
			Err(e) => return format!("Error requesting node {:?}", e)
		};

		match value {
			NetworkOperand::Role(r) => {
				let old = node.role(); 
				node.update_role(r);
				nio.gsn_node_update_role(&node).unwrap();
				format!("Updated {} role: {} -> {}", node.springname(), old, r)
			},
			
			NetworkOperand::State(s) => {
				let old = node.state(); 
				node.update_state(s);
				nio.gsn_node_update_state(&node).unwrap();
				format!("Updated {} state: {} -> {}", node.springname(), old, s)
			},

			NetworkOperand::Service(s) => {
				let old = node.service(); 
				node.update_service(s);
				nio.gsn_node_update_service(&node).unwrap();
				format!("Updated {} service: {} -> {}", node.springname(), old, s)
			},

			NetworkOperand::Host(s) => {
				let old = node.hostname().to_string(); 
				node.update_hostname(&s);
				nio.gsn_node_update_hostname(&node).unwrap();
				format!("Updated {} hostname: {} -> {}", node.springname(), old, s)
			},

			NetworkOperand::Address(s) => {
				let old = node.address().to_string(); 
				node.update_address(&s);
				nio.gsn_node_update_address(&node).unwrap();
				format!("Updated {} address: {} -> {}", node.springname(), old, s)
			},
			_ => "Error: Unknown or unsupported value for updating".to_string()
		}
	} 
	
	pub fn remove(op: NetworkOperand, nio: &Netspace) -> Option<String> {
		
		Some(match op {
			NetworkOperand::Node(s) => {
				match nio.gsn_node_by_springname(&s) {
					Ok(n) => {
						nio.gsn_node_unregister(&n).unwrap();
						format!("Removed node {}\n", n.springname())
					},
					Err(e) => format!("Error: unabled to retrieve node ({:?})\n", e)
				}
								
			},
			e => format!("Error: Unknown or unsupported target filter ({:?})\n", e)		
		})
	}
	
	fn tabulate_nodes(nodes: &Vec<Node>) -> String {
		let mut table = Table::new();
		Self::add_headings(&mut table);
		for node in nodes {
			table.add_row(Row::new(vec![
							Cell::new(node.springname()),
							Cell::new(&node.hostfield()),
							Cell::new(node.address()),
							Cell::new( &format!("{}", node.role()) ),
							Cell::new( &format!("{}", node.state()) ),
							Cell::new( &format!("{}", node.service()) )
							]));
		}
		
		
		format!("{}", table)
	}
	
	fn tabulate_node(node_result: Result<Node, NetspaceFailure>) -> String {
		
		let node = match node_result {
			Ok(n) => n,
			Err(e) => return format!("Error requesting node {:?}", e)
		};

		let mut table = Table::new();
		Self::add_headings(&mut table);
		table.add_row(Row::new(vec![
						Cell::new(node.springname()),
						Cell::new(node.hostname()),
						Cell::new(node.address()),
						Cell::new( &format!("{}", node.role()) ),
						Cell::new( &format!("{}", node.state()) ),
						Cell::new( &format!("{}", node.service()) )
					]));
		
		format!("{}", table)	
	}
	
	fn add_headings(table: &mut Table) {
		table.add_row(row!["_spring_", "_hostfield_",
							"_address_", "_role_", 
							"_state_", "_service_"]);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use management::ManagementZone;
	use netspace::*;

	macro_rules! assert_match {
	
		($chk:expr, $pass:pat) => (
			assert!(match $chk {
						$pass => true,
						_ => false
			}))
	}
	
	macro_rules! unwrap_some {
		($chk:expr) => (
			match $chk {
						Some(s) => s,
						_ => panic!("Unwrapping a None")
			})		
	}
	
	#[test]
	fn ts_network_view_all_p() {
		let mz = unwrap_some!(ManagementZone::from_str("network view all"));
		let nz : NetworkZone = extract_zone_network!(mz);
		assert_eq!(nz.action, NetworkAction::View);
		assert_eq!(nz.op1, NetworkOperand::All);
	}
	
	#[test]
	fn ts_network_view_node_p() {
		let mz = unwrap_some!(ManagementZone::from_str("network view node foo"));
		let nz : NetworkZone = extract_zone_network!(mz);
		assert_eq!(nz.action, NetworkAction::View);
		assert_eq!(nz.op1, NetworkOperand::Node(String::from("foo")));
	}
	
	#[test]
	fn ts_network_view_spring_p() {
		let mz = unwrap_some!(ManagementZone::from_str("network view springname foo"));
		let nz : NetworkZone = extract_zone_network!(mz);
		assert_eq!(nz.action, NetworkAction::View);
		assert_eq!(nz.op1, NetworkOperand::Node(String::from("foo")));
	}
	
	#[test]
	fn ts_network_view_role_p() {
		let mz = unwrap_some!(ManagementZone::from_str("network view role hybrid"));
		let nz : NetworkZone = extract_zone_network!(mz);
		assert_eq!(nz.action, NetworkAction::View);
		assert_eq!(nz.op1, NetworkOperand::Role(NodeRole::Hybrid));
	}
	
	#[test]
	fn ts_network_view_state_p() {
		let mz = unwrap_some!(ManagementZone::from_str("network view state unresponsive"));
		let nz : NetworkZone = extract_zone_network!(mz);
		assert_eq!(nz.action, NetworkAction::View);
		assert_eq!(nz.op1, NetworkOperand::State(NodeState::Unresponsive));
	}
	
	#[test]
	fn ts_network_view_service_p() {
		let mz = unwrap_some!(ManagementZone::from_str("network view service http"));
		let nz : NetworkZone = extract_zone_network!(mz);
		assert_eq!(nz.action, NetworkAction::View);
		assert_eq!(nz.op1, NetworkOperand::Service(NodeService::Http));
	}
	
	#[test]
	fn ts_network_view_address_p() {
		let mz = unwrap_some!(ManagementZone::from_str("network view address 127.0.0.1"));
		let nz : NetworkZone = extract_zone_network!(mz);
		assert_eq!(nz.action, NetworkAction::View);
		assert_eq!(nz.op1, NetworkOperand::Address(String::from("127.0.0.1")));
	}
	
	#[test]
	fn ts_network_view_host_p() {
		let mz = unwrap_some!(ManagementZone::from_str("network view hostname foo.bar"));
		let nz : NetworkZone = extract_zone_network!(mz);
		assert_eq!(nz.action, NetworkAction::View);
		assert_eq!(nz.op1, NetworkOperand::Host(String::from("foo.bar")));
	}
	
	#[test]
	fn ts_network_update_node_spring_p() {
		let mz = unwrap_some!(ManagementZone::from_str("network update node foo springname bar"));
		let nz : NetworkZone = extract_zone_network!(mz);
		assert_eq!(nz.action, NetworkAction::Update);
		assert_eq!(nz.op1, NetworkOperand::Node(String::from("foo")));
		assert_eq!(nz.op2, NetworkOperand::Node(String::from("bar")));
	}
	
	#[test]
	fn ts_network_update_node_state_p() {
		let mz = unwrap_some!(ManagementZone::from_str("network update node foo state disabled"));
		let nz : NetworkZone = extract_zone_network!(mz);
		assert_eq!(nz.action, NetworkAction::Update);
		assert_eq!(nz.op1, NetworkOperand::Node(String::from("foo")));
		assert_eq!(nz.op2, NetworkOperand::State(NodeState::Disabled));
	}
	
	#[test]
	fn ts_network_update_role_service_p() {
		let mz = unwrap_some!(ManagementZone::from_str("network update role org service dvsp"));
		let nz : NetworkZone = extract_zone_network!(mz);
		assert_eq!(nz.action, NetworkAction::Update);
		assert_eq!(nz.op1, NetworkOperand::Role(NodeRole::Org));
		assert_eq!(nz.op2, NetworkOperand::Service(NodeService::Dvsp));
	}
}