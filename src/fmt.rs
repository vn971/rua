use crate::config::Config;

use chrono::{DateTime, NaiveDateTime};

pub fn opt(opt: &Option<String>) -> &str {
    opt.as_ref().map(String::as_ref).unwrap_or("None")
}

pub fn date(date: i64) -> String {
    let date = NaiveDateTime::from_timestamp(date, 0);
    let date = DateTime::<chrono::Utc>::from_utc(date, chrono::Utc);
    date.to_rfc2822()
}

pub fn print_indent<'a>(
    conf: &Config,
    list: bool,
    indent: usize,
    cols: Option<usize>,
    k: &str,
    v: impl Iterator<Item = &'a str>,
) {
    let field = &conf.color.field;

    let prefix = format!("{:<padding$}: ", k, padding = indent - 2);
    print!("{}", field.paint(prefix));

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

    println!();
}

