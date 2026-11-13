//  Tauri Python Plugin
//  © Copyright 2024, by Marco Mengelkoch
//  Licensed under MIT License, see License file for more details
//  git clone https://github.com/marcomq/tauri-plugin-python

use super::*;
use tauri::{
    AppHandle,
    test::{self, MockRuntime},
};

/// Creates a mock Tauri app and initializes the PyRunner state.
/// It also runs some initial Python code to set up a variable and a function for testing.
async fn mock_app_handle() -> AppHandle<MockRuntime> {
    let app = test::mock_app();
    let runner = build_runner();
    app.manage(runner);
    app.manage(PluginState::default());

    let runner = app.state::<PyRunner>().inner();
    runner
        .run("my_var = 123\ndef my_func(a, b):\n  return a + b")
        .await
        .unwrap();

    app.handle().clone()
}

#[tokio::test]
async fn test_read_variable() {
    let app = mock_app_handle().await;
    let payload = StringRequest {
        value: "my_var".into(),
    };
    let response = app.read_variable(payload).await.unwrap();
    assert_eq!(response.value, "123");
}

#[tokio::test]
async fn test_run_python() {
    let app = mock_app_handle().await;
    let payload = StringRequest {
        value: "new_var = 456".into(),
    };
    app.run_python(payload).await.unwrap();

    // Verify the code was run by reading the variable back
    let read_payload = StringRequest {
        value: "new_var".into(),
    };
    let response = app.read_variable(read_payload).await.unwrap();
    assert_eq!(response.value, "456");
}

#[tokio::test]
async fn test_register_and_call_function() {
    let app = mock_app_handle().await;

    // 1. Register the function
    let register_payload = RegisterRequest {
        python_function_call: "my_func".into(),
        number_of_args: Some(2),
    };
    app.register_function(register_payload).await.unwrap();

    // 2. Call the registered function
    let call_payload = RunRequest {
        function_name: "my_func".into(),
        args: vec![serde_json::json!(10), serde_json::json!(20)],
    };
    let response = app.call_function(call_payload).await.unwrap();
    assert_eq!(response.value, "30");
}

#[tokio::test]
async fn test_call_unregistered_function_fails() {
    let app = mock_app_handle().await;
    let call_payload = RunRequest {
        function_name: "unregistered_func".into(),
        args: vec![],
    };
    let result = app.call_function(call_payload).await;
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("Function unregistered_func has not been registered yet"));
}

#[tokio::test]
async fn test_register_after_call_fails() {
    let app = mock_app_handle().await;

    // 1. Register and call a function to set the INIT_BLOCKED flag
    let register_payload = RegisterRequest {
        python_function_call: "my_func".into(),
        number_of_args: Some(2),
    };
    app.register_function(register_payload).await.unwrap();
    let call_payload = RunRequest {
        function_name: "my_func".into(),
        args: vec![serde_json::json!(1), serde_json::json!(2)],
    };
    app.call_function(call_payload).await.unwrap();

    // 2. Attempt to register another function, which should now fail
    let second_register_payload = RegisterRequest {
        python_function_call: "my_var".into(), // can be anything
        number_of_args: None,
    };
    let result = app.register_function(second_register_payload).await;
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Cannot register after function called")
    );
}

#[tokio::test]
async fn test_registering_in_second_app_after_first_call_is_allowed() {
    let first_app = mock_app_handle().await;
    first_app
        .register_function(RegisterRequest {
            python_function_call: "my_func".into(),
            number_of_args: Some(2),
        })
        .await
        .unwrap();
    first_app
        .call_function(RunRequest {
            function_name: "my_func".into(),
            args: vec![serde_json::json!(1), serde_json::json!(2)],
        })
        .await
        .unwrap();

    let second_app = mock_app_handle().await;
    let result = second_app
        .register_function(RegisterRequest {
            python_function_call: "my_func".into(),
            number_of_args: Some(2),
        })
        .await;

    assert!(
        result.is_ok(),
        "registration should be app-local: {result:?}"
    );
}

// Signature-count validation relies on `inspect.signature`, which only runs on
// the pyo3 backend; the rustpython backend can't import `inspect`, so this check
// is intentionally skipped there (see `test_rustpython_register_skips_signature_check`).
#[cfg(all(feature = "pyo3", not(feature = "rustpython")))]
#[tokio::test]
async fn test_register_function_arg_mismatch_returns_error_instead_of_panicking() {
    let app = mock_app_handle().await;
    let handle = tokio::spawn(async move {
        app.register_function(RegisterRequest {
            python_function_call: "my_func".into(),
            number_of_args: Some(1),
        })
        .await
    });

    let join_result = handle.await;
    assert!(
        join_result.is_ok(),
        "register_function should not panic on invalid signatures: {join_result:?}"
    );

    let result = join_result.unwrap();
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Function parameters don't match")
    );
}

// Guards the insert-before-validate bug across both backends: a registration
// that fails (here, because the function doesn't exist) must not leave the name
// marked as registered. Uses a nonexistent name so the failure path is backend
// independent (doesn't rely on signature validation).
#[tokio::test]
async fn test_failed_registration_does_not_allow_call() {
    let app = mock_app_handle().await;
    let register_result = app
        .register_function(RegisterRequest {
            python_function_call: "does_not_exist".into(),
            number_of_args: None,
        })
        .await;
    assert!(register_result.is_err());

    let call_result = app
        .call_function(RunRequest {
            function_name: "does_not_exist".into(),
            args: vec![],
        })
        .await;
    assert!(
        call_result.is_err(),
        "calling a function whose registration failed should error: {call_result:?}"
    );
    assert!(
        call_result
            .unwrap_err()
            .to_string()
            .contains("has not been registered yet")
    );
}

#[tokio::test]
async fn test_register_nonexistent_function_fails() {
    let app = mock_app_handle().await;
    let result = app
        .register_function(RegisterRequest {
            python_function_call: "does_not_exist".into(),
            number_of_args: None,
        })
        .await;
    assert!(result.is_err());
}

// The stdio guard runs on startup on every platform/backend; make sure the
// snippet is valid Python under both interpreters and that print() keeps working
// after stdout/stderr are wrapped (guards the Windows hidden-console fix for
// issues #4/#15/#17 against accidentally breaking normal output).
#[tokio::test]
async fn test_stdio_guard_keeps_print_working() {
    let app = mock_app_handle().await;
    let runner = app.state::<PyRunner>().inner();
    runner
        .run(PY_STDIO_GUARD)
        .await
        .expect("stdio guard snippet must be valid on this backend");
    runner
        .run("print('hello from python'); import sys; sys.stdout.flush()")
        .await
        .expect("print() must still work after stdio is wrapped");
}

// A registered function that prints (a side effect that historically broke on a
// hidden-console Windows build) must still run and return its value after the
// stdio guard is in place.
#[tokio::test]
async fn test_registered_function_can_print_and_return() {
    let app = mock_app_handle().await;
    let runner = app.state::<PyRunner>().inner();
    runner.run(PY_STDIO_GUARD).await.unwrap();
    runner
        .run("def printer(x):\n  print('side effect', x)\n  return x * 2")
        .await
        .unwrap();
    app.register_function(RegisterRequest {
        python_function_call: "printer".into(),
        number_of_args: None,
    })
    .await
    .unwrap();
    let response = app
        .call_function(RunRequest {
            function_name: "printer".into(),
            args: vec![serde_json::json!(21)],
        })
        .await
        .unwrap();
    assert_eq!(response.value, "42");
}

// read_variable must round-trip the common JSON-compatible types identically on
// both backends (regression guard for type conversion).
#[tokio::test]
async fn test_read_variable_types() {
    let app = mock_app_handle().await;
    let runner = app.state::<PyRunner>().inner();
    runner
        .run("v_int=7\nv_float=1.5\nv_bool=True\nv_list=[1,2,3]\nv_str='hi'")
        .await
        .unwrap();
    let cases = [
        ("v_int", "7"),
        ("v_float", "1.5"),
        ("v_bool", "true"),
        ("v_list", "[1,2,3]"),
        ("v_str", "\"hi\""),
    ];
    for (name, expected) in cases {
        let resp = app
            .read_variable(StringRequest { value: name.into() })
            .await
            .unwrap();
        assert_eq!(resp.value, expected, "read_variable({name})");
    }
}

// Non-ASCII / UTF-8 must survive the round-trip. This is the class of data that
// triggered the Windows non-UTF8 stdio crash, so it's worth pinning down.
#[tokio::test]
async fn test_unicode_round_trip() {
    let app = mock_app_handle().await;
    let runner = app.state::<PyRunner>().inner();
    runner.run("v_uni = 'héllo🐍'").await.unwrap();
    let resp = app
        .read_variable(StringRequest {
            value: "v_uni".into(),
        })
        .await
        .unwrap();
    assert_eq!(resp.value, "\"héllo🐍\"");
}

// Passing a JSON array through to a Python function as a list argument.
#[tokio::test]
async fn test_call_function_with_list_arg() {
    let app = mock_app_handle().await;
    let runner = app.state::<PyRunner>().inner();
    runner
        .run("def join_list(items):\n  return ','.join(str(x) for x in items)")
        .await
        .unwrap();
    app.register_function(RegisterRequest {
        python_function_call: "join_list".into(),
        number_of_args: Some(1),
    })
    .await
    .unwrap();
    let response = app
        .call_function(RunRequest {
            function_name: "join_list".into(),
            args: vec![serde_json::json!([1, 2, 3])],
        })
        .await
        .unwrap();
    assert_eq!(response.value, "1,2,3");
}

// The interpreter is a single long-lived instance, so state set by one call must
// be visible to the next.
#[tokio::test]
async fn test_state_persists_between_calls() {
    let app = mock_app_handle().await;
    app.run_python(StringRequest {
        value: "counter = 0".into(),
    })
    .await
    .unwrap();
    app.run_python(StringRequest {
        value: "counter += 5".into(),
    })
    .await
    .unwrap();
    let resp = app
        .read_variable(StringRequest {
            value: "counter".into(),
        })
        .await
        .unwrap();
    assert_eq!(resp.value, "5");
}

// An exception raised inside a called function must surface its message to the
// caller (not be swallowed) on both backends.
#[tokio::test]
async fn test_error_message_propagates_from_called_function() {
    let app = mock_app_handle().await;
    let runner = app.state::<PyRunner>().inner();
    runner
        .run("def raiser(x):\n  raise ValueError('custom boom ' + str(x))")
        .await
        .unwrap();
    app.register_function(RegisterRequest {
        python_function_call: "raiser".into(),
        number_of_args: None,
    })
    .await
    .unwrap();
    let err = app
        .call_function(RunRequest {
            function_name: "raiser".into(),
            args: vec![serde_json::json!(42)],
        })
        .await
        .unwrap_err()
        .to_string();
    assert!(
        err.contains("custom boom 42"),
        "error should carry the Python message: {err}"
    );
    assert!(
        err.contains("Error calling Python function 'raiser'"),
        "error should carry the operation context: {err}"
    );
}

// Pins down that running PY_STDIO_GUARD as a *separate* unit beforehand does not
// shift the line numbers in the traceback of subsequently-run user code. Tracebacks
// are forced on, as `cargo test --release` would otherwise run without them.
#[tokio::test]
async fn test_error_line_number_not_shifted_by_stdio_guard() {
    let runner = PyRunner::new().with_tracebacks(true);
    runner.run(PY_STDIO_GUARD).await.unwrap();
    let code = "x = 1\ny = 2\nraise ValueError('boom')\n";
    let err = runner.run(code).await.unwrap_err().to_string();
    assert!(
        err.contains("line 3"),
        "the raise on line 3 must report line 3, not be shifted by the guard: {err}"
    );
}

// Writes `<name>.py` into a temp dir and puts that dir on `sys.path`, the same
// way `init_python` does for `src-python`. Module names must be unique per test
// because PyO3 shares one interpreter (and `sys.modules`) across all tests.
fn module_entry(name: &str, dir: &tempfile::TempDir) -> PythonEntry {
    PythonEntry::Module {
        name: name.into(),
        dir: dir.path().to_path_buf(),
    }
}

async fn module_dir(runner: &PyRunner, name: &str, code: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join(format!("{name}.py")), code).unwrap();
    init_python(runner, dir.path().to_path_buf()).await;
    dir
}

#[tokio::test]
async fn test_run_entry_file() {
    let app = mock_app_handle().await;
    let runner = app.state::<PyRunner>().inner();
    let dir = tempfile::tempdir().unwrap();
    let main_py = dir.path().join("main.py");
    std::fs::write(&main_py, "entry_file_var = 'from file'\n").unwrap();
    run_entry(runner, &PythonEntry::File(main_py))
        .await
        .unwrap();
    let resp = app
        .read_variable(StringRequest {
            value: "entry_file_var".into(),
        })
        .await
        .unwrap();
    assert_eq!(resp.value, "\"from file\"");
}

// Mirrors the Nuitka entry point (`nuitka --module main.py`): the module is
// imported instead of executed as a file. A pure .py module exercises the same
// `run_module` path without requiring Nuitka in the test environment.
#[tokio::test]
async fn test_run_entry_module_registers_and_calls_functions() {
    let app = mock_app_handle().await;
    let runner = app.state::<PyRunner>().inner();
    let dir = module_dir(
        runner,
        "tauri_entry_mod_call",
        "_tauri_plugin_functions = ['greet']\ndef greet(name):\n  return 'hi ' + name\n",
    )
    .await;
    run_entry(runner, &module_entry("tauri_entry_mod_call", &dir))
        .await
        .unwrap();

    let functions = runner
        .read_variable("_tauri_plugin_functions")
        .await
        .unwrap();
    let functions: Vec<String> = serde_json::from_value(functions).unwrap();
    register_python_functions(&app, functions).await;

    let resp = app
        .call_function(RunRequest {
            function_name: "greet".into(),
            args: vec![serde_json::json!("nuitka")],
        })
        .await
        .unwrap();
    assert_eq!(resp.value, "hi nuitka");
}

#[tokio::test]
async fn test_run_entry_module_replaces_global_scope() {
    let app = mock_app_handle().await;
    let runner = app.state::<PyRunner>().inner();
    let dir = module_dir(runner, "tauri_entry_mod_scope", "mod_var = 1\n").await;
    run_entry(runner, &module_entry("tauri_entry_mod_scope", &dir))
        .await
        .unwrap();

    let mod_var = app
        .read_variable(StringRequest {
            value: "mod_var".into(),
        })
        .await
        .unwrap();
    assert_eq!(mod_var.value, "1");
    let old_var = app
        .read_variable(StringRequest {
            value: "my_var".into(),
        })
        .await;
    assert!(
        old_var.is_err(),
        "names from before run_module must be gone"
    );
}

#[tokio::test]
async fn test_run_entry_module_ignores_same_named_module_elsewhere() {
    let app = mock_app_handle().await;
    let runner = app.state::<PyRunner>().inner();
    runner
        .run("import sys, types\nsys.modules['tauri_entry_mod_shadow'] = types.ModuleType('x')\n")
        .await
        .unwrap();
    let dir = module_dir(
        runner,
        "tauri_entry_mod_shadow",
        "shadow_var = 'entry dir'\n",
    )
    .await;
    run_entry(runner, &module_entry("tauri_entry_mod_shadow", &dir))
        .await
        .unwrap();
    let resp = app
        .read_variable(StringRequest {
            value: "shadow_var".into(),
        })
        .await
        .unwrap();
    assert_eq!(resp.value, "\"entry dir\"");
}

#[tokio::test]
async fn test_run_entry_missing_module_fails() {
    let app = mock_app_handle().await;
    let runner = app.state::<PyRunner>().inner();
    let result = run_entry(
        runner,
        &PythonEntry::Module {
            name: "tauri_entry_mod_missing".into(),
            dir: std::env::temp_dir(),
        },
    )
    .await;
    assert!(result.is_err());
    // A failed import keeps the previous scope.
    let resp = app
        .read_variable(StringRequest {
            value: "my_var".into(),
        })
        .await
        .unwrap();
    assert_eq!(resp.value, "123");
}

async fn register_async_fn(app: &AppHandle<MockRuntime>) {
    let runner = app.state::<PyRunner>().inner();
    runner
        .run("async def coro_fn(x):\n  return x * 2")
        .await
        .unwrap();
    app.register_function(RegisterRequest {
        python_function_call: "coro_fn".into(),
        number_of_args: Some(1),
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn test_async_def_is_detected_on_registration() {
    let app = mock_app_handle().await;
    register_async_fn(&app).await;
    let state = app.state::<PluginState>();
    let map = state.function_map.lock().unwrap();
    assert_eq!(map.get("coro_fn"), Some(&true));
}

#[tokio::test]
async fn test_sync_def_is_not_async() {
    let app = mock_app_handle().await;
    app.register_function(RegisterRequest {
        python_function_call: "my_func".into(),
        number_of_args: None,
    })
    .await
    .unwrap();
    let state = app.state::<PluginState>();
    assert_eq!(
        state.function_map.lock().unwrap().get("my_func"),
        Some(&false)
    );
}

#[cfg(all(feature = "pyo3", not(feature = "rustpython")))]
#[tokio::test]
async fn test_call_async_def_is_awaited() {
    let app = mock_app_handle().await;
    register_async_fn(&app).await;
    let resp = app
        .call_function(RunRequest {
            function_name: "coro_fn".into(),
            args: vec![serde_json::json!(21)],
        })
        .await
        .unwrap();
    assert_eq!(resp.value, "42");
}

#[cfg(feature = "rustpython")]
#[tokio::test]
async fn test_call_async_def_reports_unsupported_on_rustpython() {
    let app = mock_app_handle().await;
    register_async_fn(&app).await;
    let err = app
        .call_function(RunRequest {
            function_name: "coro_fn".into(),
            args: vec![serde_json::json!(21)],
        })
        .await
        .unwrap_err()
        .to_string();
    assert!(err.contains("not supported"), "{err}");
}

#[cfg(feature = "rustpython")]
mod rustpython_stdlib_tests {
    use super::*;

    #[tokio::test]
    async fn frozen_stdlib_and_codecs_work() {
        let runner = PyRunner::new();
        runner
            .run(
                "import json, typing, inspect, dataclasses, collections, re, logging\n\
                 assert json.loads('[1]') == [1]\n\
                 assert 'é'.encode('latin1') == b'\\xe9'\n\
                 assert b'\\x80'.decode('cp1252') == '\\u20ac'",
            )
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn signature_check_works_with_stdlib() {
        let app = mock_app_handle().await;
        let result = app
            .register_function(RegisterRequest {
                python_function_call: "my_func".into(),
                number_of_args: Some(1),
            })
            .await;
        assert!(
            result.is_err(),
            "inspect is available, mismatch must be rejected"
        );
    }
}

#[cfg(all(feature = "venv", feature = "pyo3", not(feature = "rustpython")))]
#[tokio::test]
async fn test_init_python_loads_src_python_venv() {
    let runner = PyRunner::new();
    runner.run("import sys").await.unwrap();
    let version = runner
        .eval("f'python{sys.version_info.major}.{sys.version_info.minor}'")
        .await
        .unwrap();
    let src_python = tempfile::tempdir().unwrap();
    let site_packages = if cfg!(windows) {
        src_python.path().join(".venv/Lib/site-packages")
    } else {
        src_python
            .path()
            .join(".venv/lib")
            .join(version.as_str().unwrap())
            .join("site-packages")
    };
    std::fs::create_dir_all(&site_packages).unwrap();
    std::fs::write(site_packages.join("tauri_venv_probe.py"), "VALUE = 7\n").unwrap();

    init_python(&runner, src_python.path().to_path_buf()).await;
    runner
        .run("import tauri_venv_probe\nvenv_value = tauri_venv_probe.VALUE")
        .await
        .unwrap();
    assert_eq!(runner.read_variable("venv_value").await.unwrap(), 7);
}

#[cfg(all(feature = "venv", feature = "pyo3", not(feature = "rustpython")))]
#[tokio::test]
async fn test_init_python_survives_venv_of_other_python_version() {
    let runner = PyRunner::new();
    let src_python = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(src_python.path().join(".venv/lib/python2.7/site-packages")).unwrap();
    init_python(&runner, src_python.path().to_path_buf()).await;
    runner.run("x = 1").await.unwrap();
}
