use prettytable::*;
use prettytable::format::*;
use raur::Package;

pub fn print_package_table(packages: Vec<Package>) {
	let mut table = Table::new();
	table.set_titles(row!["Name", "Version", "Description"]);

	for package in packages {
		table.add_row(row![
			package.name,
			package.version,
			package.description
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
