//  Tauri Python Plugin
//  © Copyright 2024, by Marco Mengelkoch
//  Licensed under MIT License, see License file for more details
//  git clone https://github.com/marcomq/tauri-plugin-python

//! Unit tests for pure helpers that don't need a Python interpreter.

use super::*;
use serde_json::json;

mod timeout {
    use super::*;

    #[test]
    fn unset_uses_default() {
        assert_eq!(
            call_timeout(None),
            Some(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
        );
    }

    #[test]
    fn zero_disables_timeout() {
        assert_eq!(call_timeout(Some("0")), None);
    }

    #[test]
    fn explicit_seconds_are_used() {
        assert_eq!(call_timeout(Some("42")), Some(Duration::from_secs(42)));
    }

    #[test]
    fn surrounding_whitespace_is_ignored() {
        assert_eq!(call_timeout(Some(" 7\n")), Some(Duration::from_secs(7)));
    }

    #[test]
    fn invalid_values_fall_back_to_default() {
        let default = Some(Duration::from_secs(DEFAULT_TIMEOUT_SECS));
        for value in ["", "abc", "-1", "1.5", "99999999999999999999999"] {
            assert_eq!(call_timeout(Some(value)), default, "value: {value:?}");
        }
    }
}

mod python_paths {
    use super::*;

    #[test]
    fn cleanup_uses_forward_slashes_and_absolute_path() {
        let cleaned = cleanup_path_for_python(&PathBuf::from("."));
        assert!(!cleaned.contains('\\'), "{cleaned}");
        assert!(PathBuf::from(&cleaned).is_absolute(), "{cleaned}");
    }

    #[test]
    fn cleanup_resolves_relative_components() {
        let cwd = std::env::current_dir().unwrap();
        let via_parent = cwd.join("src").join("..");
        assert_eq!(
            cleanup_path_for_python(&via_parent),
            cleanup_path_for_python(&cwd)
        );
    }

    #[test]
    fn print_path_is_a_quoted_python_literal() {
        let printed = print_path_for_python(&PathBuf::from("."));
        let cleaned = cleanup_path_for_python(&PathBuf::from("."));
        #[cfg(not(target_os = "windows"))]
        assert_eq!(printed, format!("\"{cleaned}\""));
        #[cfg(target_os = "windows")]
        assert_eq!(printed, format!("r\"{cleaned}\""));
    }

    #[test]
    #[should_panic]
    fn cleanup_panics_for_missing_path() {
        cleanup_path_for_python(&PathBuf::from("/definitely/not/existing/src-python"));
    }

    #[test]
    fn src_python_dir_is_relative_to_cwd() {
        assert_eq!(
            get_src_python_dir(),
            std::env::current_dir().unwrap().join("src-python")
        );
    }
}

mod errors {
    use super::*;

    #[test]
    fn string_error_display() {
        assert_eq!(Error::String("boom".into()).to_string(), "Error: boom");
    }

    #[test]
    fn from_str() {
        let err: Error = "boom".into();
        assert!(matches!(err, Error::String(ref s) if s == "boom"));
    }

    #[test]
    fn from_io_error_is_transparent() {
        let err: Error = std::io::Error::new(std::io::ErrorKind::NotFound, "gone").into();
        assert!(matches!(err, Error::Io(_)));
        assert_eq!(err.to_string(), "gone");
    }

    #[test]
    fn from_py_runner_error_is_transparent() {
        let inner = async_py::PyRunnerError::PyError("ValueError: nope".into());
        let expected = inner.to_string();
        let err: Error = inner.into();
        assert!(matches!(err, Error::PyRunner(_)));
        assert_eq!(err.to_string(), expected);
    }

    #[test]
    fn serializes_as_plain_string_for_the_frontend() {
        let value = serde_json::to_value(Error::String("boom".into())).unwrap();
        assert_eq!(value, json!("Error: boom"));
    }
}

mod context {
    use super::*;

    #[test]
    fn ok_passes_through() {
        let res: std::result::Result<u8, Error> = Ok(3);
        assert_eq!(py_context(res, || unreachable!()).unwrap(), 3);
    }

    #[test]
    fn error_is_prefixed_with_context() {
        let res: std::result::Result<(), Error> = Err("inner".into());
        let err = py_context(res, || "Outer".into()).unwrap_err();
        assert_eq!(err.to_string(), "Error: Outer: Error: inner");
    }

    #[test]
    fn python_message_is_preserved() {
        let res: std::result::Result<(), _> = Err(async_py::PyRunnerError::PyError(
            "ValueError: custom".into(),
        ));
        let err = py_context(res, || "Calling f".into()).unwrap_err();
        assert!(err.to_string().contains("ValueError: custom"), "{err}");
        assert!(err.to_string().starts_with("Error: Calling f: "), "{err}");
    }
}

mod models {
    use super::*;

    #[test]
    fn string_request_from_frontend() {
        let req: StringRequest = serde_json::from_value(json!({ "value": "x = 1" })).unwrap();
        assert_eq!(req.value, "x = 1");
    }

    #[test]
    fn register_request_uses_camel_case() {
        let req: RegisterRequest = serde_json::from_value(json!({
            "pythonFunctionCall": "mod.fn",
            "numberOfArgs": 2
        }))
        .unwrap();
        assert_eq!(req.python_function_call, "mod.fn");
        assert_eq!(req.number_of_args, Some(2));
    }

    #[test]
    fn register_request_number_of_args_is_optional() {
        for payload in [
            json!({ "pythonFunctionCall": "f" }),
            json!({ "pythonFunctionCall": "f", "numberOfArgs": null }),
        ] {
            let req: RegisterRequest = serde_json::from_value(payload).unwrap();
            assert_eq!(req.number_of_args, None);
        }
    }

    #[test]
    fn register_request_rejects_snake_case() {
        let res = serde_json::from_value::<RegisterRequest>(json!({
            "python_function_call": "f"
        }));
        assert!(res.is_err());
    }

    #[test]
    fn register_request_rejects_out_of_range_arg_count() {
        let res = serde_json::from_value::<RegisterRequest>(json!({
            "pythonFunctionCall": "f",
            "numberOfArgs": 256
        }));
        assert!(res.is_err());
    }

    #[test]
    fn run_request_keeps_arbitrary_json_args() {
        let req: RunRequest = serde_json::from_value(json!({
            "functionName": "f",
            "args": [1, "two", [3], { "four": 4 }, null, true]
        }))
        .unwrap();
        assert_eq!(req.function_name, "f");
        assert_eq!(
            req.args,
            vec![
                json!(1),
                json!("two"),
                json!([3]),
                json!({ "four": 4 }),
                json!(null),
                json!(true)
            ]
        );
    }

    #[test]
    fn run_request_requires_args() {
        let res = serde_json::from_value::<RunRequest>(json!({ "functionName": "f" }));
        assert!(res.is_err());
    }

    #[test]
    fn string_response_serializes_for_frontend() {
        let value = serde_json::to_value(StringResponse { value: "Ok".into() }).unwrap();
        assert_eq!(value, json!({ "value": "Ok" }));
    }
}

mod plugin_state {
    use super::*;
    use std::sync::atomic::Ordering;

    #[test]
    fn default_state_is_open_and_empty() {
        let state = PluginState::default();
        assert!(!state.init_blocked.load(Ordering::Relaxed));
        assert!(state.function_map.lock().unwrap().is_empty());
    }
}

mod entry_detection {
    use super::*;
    use std::fs;

    fn dir_with(files: &[&str]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for f in files {
            fs::write(dir.path().join(f), "").unwrap();
        }
        dir
    }

    #[test]
    fn compiled_entry_names() {
        for name in [
            "main.so",
            "main.cpython-312-darwin.so",
            "main.cpython-311-x86_64-linux-gnu.so",
            "main.pyd",
            "main.cp312-win_amd64.pyd",
            "main.pyc",
        ] {
            assert!(is_compiled_entry(name), "{name}");
        }
    }

    #[test]
    fn non_entry_names() {
        for name in [
            "main.py",
            "main.pyi",
            "main.build",
            "mainx.so",
            "other.cpython-312-darwin.so",
            "libmain.so",
            "main",
        ] {
            assert!(!is_compiled_entry(name), "{name}");
        }
    }

    #[test]
    fn main_py_is_a_file_entry() {
        let dir = dir_with(&["main.py"]);
        assert_eq!(
            find_entry(dir.path()),
            Some(PythonEntry::File(dir.path().join("main.py")))
        );
    }

    #[test]
    fn main_py_wins_over_compiled_module() {
        let dir = dir_with(&["main.py", "main.cpython-312-darwin.so"]);
        assert!(matches!(find_entry(dir.path()), Some(PythonEntry::File(_))));
    }

    #[test]
    fn nuitka_module_is_a_module_entry() {
        for file in [
            "main.cpython-312-darwin.so",
            "main.cp312-win_amd64.pyd",
            "main.pyc",
        ] {
            let dir = dir_with(&[file]);
            assert_eq!(
                find_entry(dir.path()),
                Some(PythonEntry::Module {
                    name: "main".into(),
                    dir: dir.path().to_path_buf()
                }),
                "{file}"
            );
        }
    }

    #[test]
    fn directory_named_like_module_is_ignored() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("main.so")).unwrap();
        assert_eq!(find_entry(dir.path()), None);
    }

    #[test]
    fn unrelated_files_yield_no_entry() {
        let dir = dir_with(&["helper.py", "other.so", "requirements.txt"]);
        assert_eq!(find_entry(dir.path()), None);
    }

    #[test]
    fn missing_directory_yields_no_entry() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(find_entry(&dir.path().join("missing")), None);
    }
}
