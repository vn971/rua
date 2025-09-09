use cli_table::{
	format::{Border, Separator},
	Table,
};
use colored::*;
use raur::Package;

use crate::terminal_util::try_hyperlink_package_name;

pub fn trunc(s: &str, max_chars: usize) -> String {
	match s.char_indices().nth(max_chars.max(2)) {
		None => s.to_owned(),
		Some((idx, _)) => {
			let substr = &s[..idx - 2];
			format!("{}..", substr)
		}
	}
}

pub fn print_package_table(mut packages: Vec<Package>, keywords: &[String]) {
	packages.sort_by(|a, b| b.popularity.partial_cmp(&a.popularity).unwrap());
	let mut table = vec![vec![
		"Name".underline(),
		"Version".underline(),
		"Description".underline(),
	]];

	for package in packages {
		let name = highlight(package.name.clone(), keywords);
		let name = try_hyperlink_package_name(name, &package.name);
		let name = name.yellow();
		let version = highlight(trunc(&package.version, 14), keywords).green();
		let description = package.description.unwrap_or_else(|| String::from(""));
		let description = highlight(description, keywords);
		table.push(vec![name, version, description.into()]);
	}

	let table = table
		.table()
		.border(Border::builder().build())
		.separator(Separator::builder().build());
	cli_table::print_stdout(table).unwrap();
}

fn highlight(mut text: String, keywords: &[String]) -> String {
	for word in keywords {
		let mut minimum = 0;
		while let Some(index) = text[minimum..].to_lowercase().find(word) {
			let start = index + minimum;
			let end = start + word.len();
			let left = &text[0..start];
			let middle = &text[start..end].bold().underline();
			let right = &text[end..];
			// need to carefully construct the new String while not losing formatting in the process
			let mut text_new = format!("{}{}", left, middle);
			minimum = text_new.len();
			text_new.push_str(right);
			text = text_new;
		}
	}
	text
}
