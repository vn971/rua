use crate::aur_rpc_utils::info_map;
use crate::print_format::date;
use crate::print_format::opt;
use crate::print_format::print_indent;

use colored::*;
use failure::Error;
use term_size::dimensions_stdout;

pub fn info(pkgs: &[String], verbose: bool) -> Result<(), Error> {
	let pkg_map = info_map(&pkgs)?;

	let mut all_pkgs = Vec::with_capacity(pkg_map.len());

	for pkg in pkgs {
		if let Some(found_pkg) = pkg_map.get(pkg) {
			all_pkgs.push(found_pkg)
		} else {
			eprintln!("{} package '{}' was not found", "error:".red(), pkg);
		}
	}

	let cols = dimensions_stdout().map(|x| x.0);
	let print = |k: &str, v: &str| print(18, cols, k, v);
	let print_list = |k: &str, v: &[_]| print_list(18, cols, k, v);

	for pkg in all_pkgs {
		print("Repository", "aur");
		print("Name", &pkg.name);
		print("Description", opt(&pkg.description));
		print("URL", opt(&pkg.url));
		print("AUR URL", &pkg.package_base);
		print_list("Groups", &pkg.groups);
		print_list("Licenses", &pkg.license);
		print_list("Provides", &pkg.provides);
		print_list("Depends On", &pkg.depends);
		print_list("Make Deps", &pkg.make_depends);
		print_list("Check Deps", &pkg.check_depends);
		print_list("Optional Deps", &pkg.opt_depends);
		print_list("Conflicts With", &pkg.conflicts);
		print("Maintainer", opt(&pkg.maintainer));
		print("Votes", &pkg.num_votes.to_string());
		print("Popularity", &pkg.popularity.to_string());
		print("First Submitted", &date(pkg.first_submitted));
		print("Last Modified", &date(pkg.last_modified));
		print(
			"Out Of Date",
			pkg.out_of_date
				.map(date)
				.as_ref()
				.map(String::as_ref)
				.unwrap_or("No"),
		);

		if verbose {
			print("ID", &pkg.id.to_string());
			print("Package Base ID", &pkg.package_base_id.to_string());
			print_list("Keywords", &pkg.keywords);
			print("Snapshot URL", &pkg.url_path);
		}

		eprintln!();
	}

	Ok(())
}

fn print(indent: usize, cols: Option<usize>, k: &str, v: &str) {
	print_indent(false, indent, cols, k, v.split_whitespace())
}

fn print_list(indent: usize, cols: Option<usize>, k: &str, v: &[String]) {
	if v.is_empty() {
		print(indent, cols, k, "None")
	} else {
		print_indent(true, indent, cols, k, v.iter().map(|s| s.as_str()))
	}
}
