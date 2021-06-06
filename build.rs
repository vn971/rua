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
	use core::panic;
	use libscmp::{resolve_syscall_name, Action, Arch, Filter};
	use std::{fs::File, path::Path};
	use std::{os::unix::io::IntoRawFd, str::FromStr};

	#[cfg(target_arch = "x86_64")]
	const TARGET_ARCH: Arch = Arch::X86_64;
	#[cfg(target_arch = "x86")]
	const TARGET_ARCH: Arch = Arch::X86;
	#[cfg(target_arch = "arm")]
	const TARGET_ARCH: Arch = Arch::ARM;
	#[cfg(target_arch = "aarch64")]
	const TARGET_ARCH: Arch = Arch::AARCH64;

	pub fn generate() {
		let mut ctx = Filter::new(Action::Allow).unwrap();

		let mut target_arch = TARGET_ARCH;
		//Consider cross-compilation via cargo otherwise continue with the configured rustc target_arch
		if let Ok(cargo_cfg_target_arch) = std::env::var("CARGO_CFG_TARGET_ARCH") {
			target_arch = Arch::from_str(&cargo_cfg_target_arch)
				.expect("CARGO_CFG_TARGET_ARCH is not a supported seccomp architecture!");
		}
		ctx.remove_arch(Arch::native()).unwrap();
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
			//Resolve to syscall number on the compiling host (not for the TARGET_ARCH).
			let syscall_num = resolve_syscall_name(syscall)
				.unwrap_or_else(|| panic!("Syscall: {} could not be resolved!", syscall));

			//Add rule to filter. The syscall number will now be translated for all enabled architectures in the filter.
			ctx.add_rule(Action::KillThread, syscall_num, &[])
				.unwrap_or_else(|err| {
					panic!(
						"Failed to add rule for syscall {}({}). Error: {}",
						syscall, syscall_num, err
					);
				});
		}

		//Write the pfc / bpf in the OUT_DIR of the build process
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
