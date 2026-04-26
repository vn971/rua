use crate::alpm_wrapper::new_alpm_wrapper;

pub fn list_real() {
	let alpm = new_alpm_wrapper();
	let packages = match alpm.get_non_pacman_packages() {
		Ok(packages) => packages,
		Err(err) => {
			eprintln!("Error: {}", err);
			return;
		}
	};
	for (name, version) in packages.iter() {
		println!("{} {}", name, version);
	}
}
