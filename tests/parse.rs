#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::{
    env,
    ffi::OsStr,
    fs,
    path::{self, Path, PathBuf},
    process::{Command, Output},
};

use mini_alive::syntax::{parse::Parser, source::SourceFile};

#[test]
#[ignore = "no tests yet"]
fn parse_mini_alive() {
    run_mini_alive(false);
}
#[test]
#[ignore = "manually update"]
fn update_mini_alive() {
    run_mini_alive(true);
}

#[test]
#[ignore = "no tests yet"]
fn parse_llvm() {
    run_llvm(false);
}
#[test]
#[ignore = "manually update"]
fn update_llvm() {
    run_llvm(true);
}

fn run_mini_alive(update: bool) {
    let root = root();
    for ll_path in case_paths(&root) {
        if ll_path.with_extension("skip").is_file() {
            continue;
        }
        let pretty_path = ll_path.with_extension("mini.pretty.ll");
        let expected_pretty = fs::read(&pretty_path).ok();
        let path = ll_path.strip_prefix(&root).unwrap_or(&ll_path).display();
        let Some(expected_pretty) = expected_pretty else {
            panic!("{path}: test must have .skip or .mini.pretty.ll");
        };

        let text = fs::read_to_string(&ll_path).unwrap();
        let src = SourceFile::new(text, ll_path.as_path().into());
        let mut parser = Parser::new(&src);
        let module = parser
            .parse_module()
            .unwrap_or_else(|err| panic!("{path}: {err}"));
        assert!(parser.eof(), "{path}: parser did not consume input");
        let pretty = module.to_string();
        compare(
            pretty.as_bytes(),
            &expected_pretty,
            &pretty_path,
            path,
            update,
        );
    }
}

fn run_llvm(update: bool) {
    let Some(llvm_as) = find_llvm_tool("llvm-as") else {
        eprintln!("skipping: llvm-as not found");
        return;
    };
    let Some(llvm_dis) = find_llvm_tool("llvm-dis") else {
        eprintln!("skipping: llvm-dis not found");
        return;
    };

    let root = root();
    for ll_path in case_paths(&root) {
        let pretty_path = ll_path.with_extension("llvm.pretty.ll");
        let err_path = ll_path.with_extension("llvm.err");
        let bc_path = ll_path.with_extension("bc");
        let _bc_remove = DropRemove(&bc_path);
        let as_output = run_llvm_as(&llvm_as, &ll_path, &bc_path);
        let expected_pretty = fs::read(&pretty_path).ok();
        let expected_err = fs::read(&err_path).ok();

        let path = ll_path.strip_prefix(&root).unwrap_or(&ll_path).display();
        match (expected_pretty, expected_err) {
            (Some(expected_pretty), None) => {
                if !as_output.status.success() {
                    panic!("{path}:\n{}", String::from_utf8_lossy(&as_output.stderr));
                }
                let dis_output = run_llvm_dis(&llvm_dis, &bc_path);
                if !dis_output.status.success() {
                    panic!("{path}:\n{}", String::from_utf8_lossy(&dis_output.stderr));
                }
                compare(
                    &dis_output.stdout,
                    &expected_pretty,
                    &pretty_path,
                    path,
                    update,
                );
            }
            (None, Some(expected_err)) => {
                assert!(
                    !as_output.status.success(),
                    "{path}: expected llvm-as to fail",
                );
                compare(&as_output.stderr, &expected_err, &err_path, path, update);
            }
            (Some(_), Some(_)) => panic!("{path}: test has both .llvm.pretty.ll and .llvm.err"),
            (None, None) => panic!("{path}: test must have either .llvm.err or .llvm.pretty.ll"),
        }
    }
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/parse")
}

fn case_paths(root: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let mut dirs = Vec::new();
    dirs.push(root.to_owned());
    while let Some(dir) = dirs.pop() {
        let entries = fs::read_dir(dir).unwrap();
        for res in entries {
            let path = res.unwrap().path();
            if path.is_dir() {
                dirs.push(path);
            } else if path.extension() == Some(OsStr::new("ll"))
                && let path_bytes = path.as_os_str().as_encoded_bytes()
                && !path_bytes.ends_with(b".pretty.ll")
            {
                paths.push(path);
            }
        }
    }
    paths.sort();
    paths
}

fn find_llvm_tool(name: &str) -> Option<PathBuf> {
    if let Some(path) = env::var_os("LLVM_DIR") {
        let mut path = PathBuf::from(path);
        path.push(name);
        if path.is_file() {
            return Some(path);
        }
    }
    for prefix in ["/usr/bin", "/usr/local/opt/llvm/bin"] {
        let path = Path::new(prefix).join(name);
        if path.is_file() {
            return Some(path);
        }
    }
    None
}

fn run_llvm_as(llvm_as: &Path, ll_path: &Path, bc_path: &Path) -> Output {
    let dir = ll_path.parent().unwrap();
    let mut cmd = Command::new(llvm_as);
    cmd.current_dir(dir)
        .arg(ll_path.file_name().unwrap())
        .arg("-o")
        .arg(bc_path.file_name().unwrap());
    #[cfg(unix)]
    cmd.arg0("llvm-as");
    cmd.output()
        .unwrap_or_else(|err| panic!("llvm-as {}: {err}", ll_path.display()))
}

fn run_llvm_dis(llvm_dis: &Path, bc_path: &Path) -> Output {
    let dir = bc_path.parent().unwrap();
    let mut cmd = Command::new(llvm_dis);
    cmd.current_dir(dir)
        .arg(bc_path.file_name().unwrap())
        .arg("-o")
        .arg("-");
    #[cfg(unix)]
    cmd.arg0("llvm-dis");
    cmd.output()
        .unwrap_or_else(|err| panic!("llvm-dis {}: {err}", bc_path.display()))
}

fn compare(actual: &[u8], expected: &[u8], path: &Path, ll_path: path::Display<'_>, update: bool) {
    if actual != expected {
        if update {
            fs::write(path, actual).unwrap();
        } else {
            panic!(
                "{ll_path}:\n   actual: \"{}\"\n expected: \"{}\"",
                actual.escape_ascii(),
                expected.escape_ascii(),
            );
        }
    }
}

struct DropRemove<'a>(&'a Path);

impl Drop for DropRemove<'_> {
    fn drop(&mut self) {
        _ = fs::remove_file(self.0);
    }
}
