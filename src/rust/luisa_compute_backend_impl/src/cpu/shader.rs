use crate::cpu::llvm::LLVM_PATH;
use crate::panic_abort;
use luisa_compute_cpu_kernel_defs as defs;
use luisa_compute_cpu_kernel_defs::KernelFnArgs;
use std::{
    env::{self, current_exe},
    fs::{canonicalize, File},
    io::Write,
    mem::transmute,
    path::PathBuf,
    process::{Command, Stdio},
};

use super::llvm;
fn canonicalize_and_fix_windows_path(path: PathBuf) -> std::io::Result<PathBuf> {
    let path = canonicalize(path)?;
    let mut s: String = path.to_str().unwrap().into();
    if s.starts_with(r"\\?\") {
        // s(r"\\?\".len());
        s = s[r"\\?\".len()..].into();
    }
    Ok(PathBuf::from(s))
}
// fn with_file_lock<T>(file: &str, f: impl FnOnce() -> T) -> T {
//     let file = File::create(file).unwrap();
//     file.lock_exclusive().unwrap();
//     let ret = f();
//     file.unlock().unwrap();
//     ret
// }
pub(super) fn clang_args() -> Vec<&'static str> {
    let mut args = vec![];
    match env::var("LUISA_DEBUG") {
        Ok(s) => {
            if s == "full" {
                args.push("-DLUISA_DEBUG");
                args.push("-DLUISA_DEBUG_FULL");
            }
            if s == "1" {
                args.push("-DLUISA_DEBUG");
            } else {
                args.push("-O3");
            }
        }
        Err(_) => {
            args.push("-O3");
        }
    }
    args.push("-march=native");
    args.push("-std=c++20");
    args.push("-fno-math-errno");
    if cfg!(target_arch = "x86_64") {
        args.push("-mavx2");
        args.push("-DLUISA_ARCH_X86_64");
    } else if cfg!(target_arch = "aarch64") {
        args.push("-DLUISA_ARCH_ARM64");
    } else {
        panic_abort!("unsupported target architecture");
    }
    args.push("-ffast-math");
    args.push("-fno-rtti");
    args.push("-fno-exceptions");
    args.push("-fno-stack-protector");
    args
}
pub(super) fn compile(
    target: &String,
    source: &String,
    force_recompile: bool,
) -> std::io::Result<PathBuf> {
    let self_path = current_exe().map_err(|e| {
        eprintln!("current_exe() failed");
        e
    })?;
    let self_path: PathBuf = canonicalize_and_fix_windows_path(self_path)?
        .parent()
        .unwrap()
        .into();
    let mut build_dir = self_path.clone();
    build_dir.push(".cache/");

    if !build_dir.exists() {
        std::fs::create_dir_all(&build_dir).map_err(|e| {
            eprintln!("fs::create_dir_all({}) failed", build_dir.display());
            e
        })?;
    }

    let target_lib = format!("{}.bc", target);
    let lib_path = PathBuf::from(format!("{}/{}", build_dir.display(), target_lib));
    if lib_path.exists() && !force_recompile {
        log::info!("Loading cached LLVM IR {}", &target_lib[1..17]);
        return Ok(lib_path);
    }
    let dump_src = match env::var("LUISA_DUMP_SOURCE") {
        Ok(s) => s == "1",
        Err(_) => false,
    };
    let source_file = if dump_src {
        let source_file = format!("{}/{}.cc", build_dir.display(), target);
        std::fs::write(&source_file, &source).map_err(|e| {
            eprintln!("fs::write({}) failed", source_file);
            e
        })?;
        source_file
    } else {
        "-".to_string()
    };
    // log::info!("compiling kernel {}", source_file);
    {
        let mut args: Vec<&str> = clang_args();
        args.push("-c");
        args.push("-emit-llvm");
        args.push("-x");
        args.push("c++");
        args.push(&source_file);
        args.push("-o");
        args.push(&target_lib);
        let clang = &LLVM_PATH.clang;
        let tic = std::time::Instant::now();
        let mut child = Command::new(clang)
            .args(args)
            .current_dir(&build_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("clang++ failed to start");
        if source_file == "-" {
            let mut stdin = child.stdin.take().expect("failed to open stdin");
            stdin
                .write_all(source.as_bytes())
                .expect("failed to write to stdin");
        }
        match child.wait_with_output().expect("clang++ failed") {
            output @ _ => match output.status.success() {
                true => {
                    log::info!(
                        "LLVM IR generated in {:.3}ms",
                        (std::time::Instant::now() - tic).as_secs_f64() * 1e3
                    );
                }
                false => {
                    eprintln!(
                        "clang++ output: {}",
                        String::from_utf8(output.stdout).unwrap(),
                    );
                    panic_abort!("compile failed")
                }
            },
        }

        Ok(lib_path)
    }
}

pub(crate) type KernelFn = unsafe extern "C" fn(*const KernelFnArgs);

pub(crate) struct ShaderImpl {
    // #[allow(dead_code)]
    // lib: libloading::Library,
    // entry: libloading::Symbol<'static, KernelFn>,
    entry: KernelFn,
    pub(crate) dir: PathBuf,
    pub(crate) captures: Vec<defs::KernelFnArg>,
    pub(crate) custom_ops: Vec<defs::CpuCustomOp>,
    pub(crate) block_size: [u32; 3],
    pub(crate) messages: Vec<String>,
}
impl ShaderImpl {
    pub(crate) fn new(
        name: String,
        path: PathBuf,
        captures: Vec<defs::KernelFnArg>,
        custom_ops: Vec<defs::CpuCustomOp>,
        block_size: [u32; 3],
        messages: &Vec<String>,
    ) -> Option<Self> {
        // unsafe {
        // let lib = libloading::Library::new(&path)
        //     .unwrap_or_else(|_| panic_abort!("cannot load library {:?}", &path));
        // let entry: libloading::Symbol<KernelFn> = lib.get(b"kernel_fn").unwrap();
        // let entry: libloading::Symbol<'static, KernelFn> = transmute(entry);
        let tic = std::time::Instant::now();
        let entry = llvm::compile_llvm_ir(&name, &String::from(path.to_str().unwrap()))?;
        let elapsed = (std::time::Instant::now() - tic).as_secs_f64() * 1e3;
        log::info!("LLVM IR compiled in {:.3}ms", elapsed);
        Some(Self {
            // lib,
            entry,
            captures,
            dir: path.clone(),
            custom_ops,
            block_size,
            messages: messages.clone(),
        })
        // }
    }
    pub(crate) fn fn_ptr(&self) -> KernelFn {
        self.entry
    }
}
