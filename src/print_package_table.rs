use colored::*;
use prettytable::format::*;
use prettytable::*;
use raur::Package;

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
	let mut table = Table::new();
	table.set_titles(row![
		"Name".underline(),
		"Version".underline(),
		"Description".underline()
	]);

	for package in packages {
		let name = highlight(trunc(&package.name, 28), keywords).yellow();
		let version = highlight(trunc(&package.version, 14), keywords).green();
		let description = package.description.unwrap_or_else(|| String::from(""));
		let description = highlight(description, keywords);
		table.add_row(row![name, version, description]);
	}

	let fmt = FormatBuilder::new().padding(0, 1).build();
	table.set_format(fmt);
	table.printstd();
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
