fn main() {
	// Check that a single TLS feature has been used
	#[cfg(all(feature = "rustls-tls", feature = "native-tls"))]
	compile_error!("You must select either the `rustls-tls` or the `native-tls` feature.");

	#[cfg(not(any(feature = "rustls-tls", feature = "native-tls")))]
	compile_error!("You must select either the `rustls-tls` or the `native-tls` feature.");

	// generate the shell completions
	shell_completions::generate();

	// generate seccomp.bpf for the target architecture
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
	use libscmp::resolve_syscall_name;
	use libscmp::Action;
	use libscmp::Arch;
	use libscmp::Filter;
	use std::fs::File;
	use std::os::unix::io::IntoRawFd;
	use std::path::Path;
	use std::str::FromStr;

	pub fn generate() {
		let mut ctx = Filter::new(Action::Allow).unwrap();

		// Get the target_arch configured in cargo to allow cross compiling
		let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").expect(
			"Failed to compile seccomp filter, environment CARGO_CFG_TARGET_ARCH not found. This env is normally set by cargo.",
		);
		let target_arch = Arch::from_str(&target_arch)
			.expect("Failed to compile seccomp filter, CARGO_CFG_TARGET_ARCH is not supported by crate libscmp.");
		ctx.remove_arch(Arch::NATIVE).unwrap();
		ctx.add_arch(target_arch).unwrap();

		// Deny these syscalls
		for syscall in &[
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
			"migrate_pages",
			"modify_ldt",
			"mount",
			"move_pages",
			"name_to_handle_at",
			"nfsservctl",
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
			// Resolve the syscall number on the compiling host. (Not directly for the TARGET_ARCH, that mapping will be done automatically by libseccomp when the filter is exported.).
			let syscall_num = resolve_syscall_name(syscall).unwrap_or_else(|| {
				panic!(
					"Failed to compile seccomp filter, syscall {} could not be resolved.",
					syscall
				)
			});

			// Add rule to filter. The syscall number will later be translated for all enabled architectures in the filter.
			ctx.add_rule(Action::KillThread, syscall_num, &[])
				.unwrap_or_else(|err| {
					panic!(
						"Failed to compile seccomp filter, failed to add rule for syscall {}({}). Error: {}",
						syscall, syscall_num, err
					);
				});
		}

		let out_dir = std::env::var("OUT_DIR")
			.expect("Failed to save generated seccomp filter, no compile-time OUT_DIR defined.");

		// Export the bpf file that will be "inlined" at RUA build time
		let fd = File::create(Path::new(&out_dir).join("seccomp.bpf"))
			.expect("Cannot create file seccomp.bpf in OUT_DIR.");
		ctx.export_bpf(fd.into_raw_fd())
			.expect("Failed to export seccomp.bpf.");

		// Export the pfc file for debugging (not used for the actual build)
		let fd = File::create(Path::new(&out_dir).join("seccomp.pfc"))
			.expect("Cannot create file seccomp.pfc in OUT_DIR.");
		ctx.export_pfc(fd.into_raw_fd())
			.expect("Failed to export seccomp.pfc.");
	}
}
