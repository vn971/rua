use chrono::offset::TimeZone;
use chrono::Utc;
use colored::*;

const DATE_FORMAT: &str = "%Y-%m-%d %H:%M:%S UTC";

pub fn opt(opt: &Option<String>) -> &str {
	opt.as_ref().map(String::as_ref).unwrap_or("None")
}

pub fn date(date: i64) -> String {
	Utc.timestamp(date, 0).format(DATE_FORMAT).to_string()
}

pub fn print_indent<'a>(
	list: bool,
	indent: usize,
	cols: Option<usize>,
	k: &str,
	v: impl Iterator<Item = &'a str>,
) {
	let prefix = format!("{:<padding$}: ", k, padding = indent - 2);
	print!("{}", prefix.bold());

	match cols {
		Some(cols) if cols > indent + 2 => {
			let mut pos = 0;
			for word in v {
				if word.len() + pos + indent + 2 >= cols {
					print!("\n{:>padding$}", "", padding = indent);
					pos = 0;
				}

				if list {
					print!("{}  ", word);
					pos += word.len() + 2;
				} else {
					print!("{} ", word);
					pos += word.len() + 1;
				}
			}
		}
		_ if list => print!("{}", v.collect::<Vec<_>>().join("  ")),
		_ => print!("{}", v.collect::<Vec<_>>().join(" ")),
	}

	eprintln!();
}
