pub fn read_at_compile_time<'a>() -> &'a str {
    // no actual error, file auto generated in build.rs, no idea how to ignore this
    // moved to separate file to not ignore other errors
    include_str!(concat!(env!("PWD"), "/", "src-tauri/src-python/main.py"))
}
