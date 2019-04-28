use prettytable::Table;
use raur::Package;

pub fn print_table_representation(packages: Vec<Package>) {
    // Create the table
    let mut table = Table::new();
    // Table footer
    table.add_row(row!["Id", "Name", "Version", "Description"]);

    // Add a row per package
    for package in packages {
        table.add_row(row![
			package.id,
			package.name,
			package.version,
			package.description
		]);
    }

    // Print the table to stdout
    table.printstd();
}
