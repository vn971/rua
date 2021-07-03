fn main() {
	//create the shell completions
	shell_completions::generate();

	//create seccomp.bpf for target_arch
	seccomp::generate();
}

mod shell_completions {
	extern crate structopt;

	use structopt::clap::Shell;

	include!("src/cli_args.rs");

	pub fn generate() {
		let directory = match std::env::var_os("COMPLETIONS_DIR") {
			None => return,
			Some(out_dir) => out_dir,
		};
		let mut app = CliArgs::clap();
		app.gen_completions(env!("CARGO_PKG_NAME"), Shell::Bash, &directory);
		app.gen_completions(env!("CARGO_PKG_NAME"), Shell::Fish, &directory);
		app.gen_completions(env!("CARGO_PKG_NAME"), Shell::Zsh, &directory);
	}
}

mod seccomp {
	use libscmp::{resolve_syscall_name, Action, Arch, Filter};
	use std::{fs::File, path::Path};
	use std::{os::unix::io::IntoRawFd, str::FromStr};

	pub fn generate() {
		let mut ctx = Filter::new(Action::Allow).unwrap();

		//Get the target_arch configured in cargo to allow cross compiling
		let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").expect(
			"Compiling without cargo is not supported! Env CARGO_CFG_TARGET_ARCH not found.",
		);
		let target_arch = Arch::from_str(&target_arch)
			.expect("CARGO_CFG_TARGET_ARCH is not a supported seccomp architecture!");
		ctx.remove_arch(Arch::NATIVE).unwrap();
		ctx.add_arch(target_arch).unwrap();

		//Deny these syscalls
		for syscall in &[
			"add_key",
			"_sysctl",
			"acct",
			"add_key",
			"adjtimex",
			"chroot",
			"clock_adjtime",
			"create_module",
			"delete_module",
			"fanotify_init",
			"finit_module",
			"get_kernel_syms",
			"get_mempolicy",
			"init_module",
			"io_cancel",
			"io_destroy",
			"io_getevents",
			"io_setup",
			"io_submit",
			"ioperm",
			"iopl",
			"ioprio_set",
			"kcmp",
			"kexec_file_load",
			"kexec_load",
			"keyctl",
			"lookup_dcookie",
			"mbind",
			"nfsservctl",
			"migrate_pages",
			"modify_ldt",
			"mount",
			"move_pages",
			"name_to_handle_at",
			"open_by_handle_at",
			"perf_event_open",
			"pivot_root",
			"process_vm_readv",
			"process_vm_writev",
			"ptrace",
			"reboot",
			"remap_file_pages",
			"request_key",
			"set_mempolicy",
			"swapoff",
			"swapon",
			"sysfs",
			"syslog",
			"tuxcall",
			"umount2",
			"uselib",
			"vmsplice",
		] {
			//Resolve the syscall number on the compiling host. (Not directly for the TARGET_ARCH, that mapping will be done automatically by libseccomp when the filter is exported.).
			let syscall_num = resolve_syscall_name(syscall)
				.unwrap_or_else(|| panic!("Syscall: {} could not be resolved!", syscall));

			//Add rule to filter. The syscall number will later be translated for all enabled architectures in the filter.
			ctx.add_rule(Action::KillThread, syscall_num, &[])
				.unwrap_or_else(|err| {
					panic!(
						"Failed to add rule for syscall {}({}). Error: {}",
						syscall, syscall_num, err
					);
				});
		}

		//Export the bpf and pfc file to OUT_DIR of the build process
		let out_dir = std::env::var("OUT_DIR").expect("No compile-time OUT_DIR defined!");

		let fd = File::create(Path::new(&out_dir).join("seccomp.bpf"))
			.expect("Cannot create file seccomp.bpf in OUT_DIR!");
		ctx.export_bpf(fd.into_raw_fd())
			.expect("Failed to export seccomp.bpf!");

		let fd = File::create(Path::new(&out_dir).join("seccomp.pfc"))
			.expect("Cannot create file seccomp.pfc in OUT_DIR!");
		ctx.export_pfc(fd.into_raw_fd())
			.expect("Failed to export seccomp.pfc!");
	}
}
