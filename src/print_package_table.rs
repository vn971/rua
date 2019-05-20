use chrono::offset::TimeZone;
use chrono::Utc;
use prettytable::format::*;
use prettytable::*;
use raur::Package;

const DATE_FORMAT: &str = "%Y-%m-%d %H:%M";

fn trunc(s: &str, max_chars: usize) -> String {
	match s.char_indices().nth(max_chars.max(2)) {
		None => s.to_owned(),
		Some((idx, _)) => {
			let substr = &s[..idx - 2];
			format!("{}..", substr)
		}
	}
}

pub fn print_package_table(mut packages: Vec<Package>) {
	packages.sort_unstable_by(|a, b| b.popularity.partial_cmp(&a.popularity).unwrap());
	let mut table = Table::new();
	table.set_titles(row!["Name", "Version", "Description"]);

	for package in packages {
		table.add_row(row![
			trunc(&package.name, 28),
			trunc(&package.version, 12),
			package.description.unwrap_or_else(|| String::from(""))
		]);
	}

	let separator: LineSeparator = LineSeparator::new('=', '+', '+', '+');
	let fmt = FormatBuilder::new()
		.padding(0, 2)
		.separator(LinePosition::Title, separator)
		.build();
	table.set_format(fmt);
	table.printstd();
}

pub fn print_separate_packages(packages: Vec<Package>) {
	for package in packages {
		eprintln!("Name: {}", package.name);
		eprintln!("Version: {}", package.version);
		eprintln!("License: {}", package.license.join(" "));
		eprintln!(
			"Description: {}",
			package.description.unwrap_or_else(|| String::from(""))
		);
		eprintln!("Popularity: {}", package.popularity);
		let result = Utc
			.timestamp(package.first_submitted, 0)
			.format(DATE_FORMAT);
		eprintln!("FirstSubmitted: {}", result);
		let result = Utc.timestamp(package.last_modified, 0).format(DATE_FORMAT);
		eprintln!("LastModified: {}", result);
		eprintln!();
	}
}
