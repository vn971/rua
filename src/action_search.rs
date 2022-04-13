use crate::print_package_table;
use raur::blocking::Raur;
use raur::Package;
use raur::SearchBy;

fn contains_keyword(pkg: &Package, keyword: &str) -> bool {
	let filter = keyword.to_lowercase();
	pkg.name.to_lowercase().contains(filter.as_str())
		|| pkg
			.description
			.iter()
			.any(|descr| descr.to_lowercase().contains(filter.as_str()))
}

pub fn action_search(keywords: &[String]) {
	let mut keywords = Vec::from(keywords);
	keywords.sort_by_key(|t| -(t.len() as i16));
	let query = keywords
		.first()
		.expect("Zero search arguments, should be impossible in structopt");
	let raur_handle = raur::blocking::Handle::new();
	let result = raur_handle.search_by(query, SearchBy::NameDesc);
	match result {
		Ok(mut result) => {
			if result.is_empty() {
				std::process::exit(1)
			};
			for keyword in &keywords[1..] {
				result.retain(|pkg| contains_keyword(pkg, keyword));
			}
			print_package_table::print_package_table(result, &keywords)
		}
		Err(e) => eprintln!("Search error: {:?}", e),
	}
}
