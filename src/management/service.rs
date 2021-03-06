use std::str::Split;

use ::netservice;
use ::netservice::cert;

use ::protocol::Svr;
use ::management::ManagedService;


#[derive(Clone, PartialEq, Debug)]
pub enum ServiceAction {
	Init, Manage
}

#[derive(Clone, PartialEq, Debug)]
pub enum ServiceOperand {
	None,
	All,
	Module(netservice::Module),
	Pass(Vec<String>)
}

#[derive(Clone, PartialEq, Debug)]
pub struct ServiceZone {
	action: ServiceAction,
	op1: ServiceOperand,
	op2: ServiceOperand,
}


#[macro_export]
macro_rules! extract_zone_service {
	($e: expr) => (
		match $e {
			ManagementZone::Service(s) => s,
			e => panic!("extract_zone_service -- Unexpected value: {:?}", e) 
		}
	)
}


impl ServiceZone {
	pub fn new(action: ServiceAction, op1: ServiceOperand, op2: ServiceOperand) -> ServiceZone {
		ServiceZone {
			action: action,
			op1: op1,
			op2: op2
		}
	}

	pub fn from_str(msg: &str) -> Option<ServiceZone> {
		if msg.len() == 0 { return None; }
		
		let mut atom = msg.split(" ");
		
		let action = match atom.next() {
			Some("ini") | Some("init") => ServiceAction::Init,
			Some("man") | Some("manage") => ServiceAction::Manage,
			_ => return None 	
		};

		let op1 = match cascade_none_nowrap!(ServiceZone::extract_operand(&mut atom)) {
			ServiceOperand::None => return None,
			op => op
		};

		let op2 = cascade_none_nowrap!(ServiceZone::extract_operand(&mut atom));
		
		Some(ServiceZone::new(action, op1, op2))
	}
	
	fn extract_operand(atom: &mut Split<&str>) -> Option<ServiceOperand> {
		
		Some(match atom.next() {
			Some("all") =>
						ServiceOperand::All,

			Some("mod") | Some("module") =>
						ServiceOperand::Module(cascade_none_nowrap!(
								netservice::Module::from_str(
									cascade_none_nowrap!(atom.next())
								)
							)),
			Some(":") | Some("<") => {
						let mut v :Vec<String> = Vec::new();
						let mut o = atom.next();
						while o != None {
							v.push(o.unwrap().to_string());
							o = atom.next();
						}
						ServiceOperand::Pass(v)
			}

			_ => ServiceOperand::None,
		})
	}
	
	pub fn process(sz: ServiceZone, svr: &Svr) -> Option<String> {
		Some(match sz.action {
			ServiceAction::Init => ServiceZoneModel::init(sz.op1),
			ServiceAction::Manage => ServiceZoneModel::manage(sz.op1, sz.op2, svr)
		})
	}
}

struct ServiceZoneModel;

impl ServiceZoneModel {
	pub fn init(op: ServiceOperand) -> String {
		match op {
			ServiceOperand::Module(m) => ServiceZoneModel::module_init(m),
			_ => format!("Init operation is not supported by target filter")
		}
		
	}
	
	fn module_init(module: netservice::Module) -> String {
		match module {
			netservice::Module::Cert => {
				cert::manager::CertManagementInterface::new().init()
			}
		}
	}
	
	fn manage(target: ServiceOperand, pass: ServiceOperand, svr: &Svr) -> String {
		match target {
			ServiceOperand::Module(m) => ServiceZoneModel::module_manage(m, pass, svr),
			_ => format!("Manage operation is not supported by target filter")
		}
	}
	
	fn module_manage(module: netservice::Module, pass: ServiceOperand, svr: &Svr) -> String {
		let v = match pass {
			ServiceOperand::Pass(p) => p,
			_ => return "Bad operand for Manage operation".to_string()
		};

		match module {
			netservice::Module::Cert => {
				cert::manager::CertManagementInterface::new().hook(&v, svr)
			}
		}		
	}
}


#[cfg(test)]
mod tests {
	use super::*;
	use management::ManagementZone;
	use ::netservice;

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
	fn ts_service_init_module_cert_p() {
		let mz = unwrap_some!(ManagementZone::from_str("service init module cert"));
		let sz : ServiceZone = extract_zone_service!(mz);
		assert_eq!(sz.action, ServiceAction::Init);
		assert_eq!(sz.op1, ServiceOperand::Module(netservice::Module::Cert));
	}
}