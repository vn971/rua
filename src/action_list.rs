use crate::alpm_wrapper::new_alpm_wrapper;

pub fn list_real() {
	let alpm = new_alpm_wrapper();
	for (name, version) in alpm.get_non_pacman_packages().unwrap().iter() {
		println!("{} {}", name, version);
	}
}
