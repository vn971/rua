use anyhow::anyhow;
use anyhow::Result;
use chrono::DateTime;
use colored::*;

const DATE_FORMAT: &str = "%Y-%m-%d %H:%M:%S UTC";

pub fn opt(opt: &Option<String>) -> &str {
	opt.as_ref().map(String::as_ref).unwrap_or("None")
}

pub fn date(timestamp: i64) -> Result<String> {
	match DateTime::from_timestamp(timestamp, 0) {
		Some(dt) => Ok(dt.format(DATE_FORMAT).to_string()),
		None => Err(anyhow!(
			"Cannot convert timestamp {} to date/time",
			timestamp
		)),
	}
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

	println!(); // This is the _result_ of rua execution, not a side log. Thus no `eprintln`.
}
