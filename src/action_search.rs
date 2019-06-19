use crate::print_package_table;
use raur::{Package, SearchBy};

fn is_package_ok(pkg: &Package, filter: &str) -> bool {
	let filter = filter.to_lowercase();
	pkg.name.to_lowercase().contains(filter.as_str())
		|| pkg
			.description
			.iter()
			.any(|descr| descr.to_lowercase().contains(filter.as_str()))
}

pub fn action_search(target: Vec<String>) {
	let query = target
		.first()
		.expect("Zero search arguments, should be impossible in structopt");
	let result = raur::search_by(query, SearchBy::NameDesc);
	match result {
		Ok(mut result) => {
			for filter in &target[1..] {
				result.retain(|p| is_package_ok(p, filter));
			}
			print_package_table::print_package_table(result)
		}
		Err(e) => eprintln!("Search error: {:?}", e),
	}
}
