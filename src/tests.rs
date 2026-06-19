//  Tauri Python Plugin
//  © Copyright 2024, by Marco Mengelkoch
//  Licensed under MIT License, see License file for more details
//  git clone https://github.com/marcomq/tauri-plugin-python

use super::*;
use tauri::{
    test::{self, MockRuntime},
    AppHandle,
};

/// Creates a mock Tauri app and initializes the PyRunner state.
/// It also runs some initial Python code to set up a variable and a function for testing.
async fn mock_app_handle() -> AppHandle<MockRuntime> {
    let app = test::mock_app();
    let runner = PyRunner::new();
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
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Cannot register after function called"));
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
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Function parameters don't match"));
}

// The rustpython backend can't import `inspect`, so signature validation can't
// run; registration of an existing function must still succeed (graceful skip)
// rather than failing on the unrelated import error.
#[cfg(feature = "rustpython")]
#[tokio::test]
async fn test_rustpython_register_skips_signature_check() {
    let app = mock_app_handle().await;
    // Deliberately wrong arg count: under pyo3 this would be rejected, but with
    // no `inspect` available the check is skipped and registration succeeds.
    app.register_function(RegisterRequest {
        python_function_call: "my_func".into(),
        number_of_args: Some(1),
    })
    .await
    .expect("registration should succeed when the signature check can't run");

    // And the function is genuinely usable afterwards.
    let response = app
        .call_function(RunRequest {
            function_name: "my_func".into(),
            args: vec![serde_json::json!(10), serde_json::json!(20)],
        })
        .await
        .unwrap();
    assert_eq!(response.value, "30");
}

// Guards the insert-before-validate bug across both backends: a registration
// that fails (here, because the function doesn't exist) must not leave the name
// marked as registered. Uses a nonexistent name so the failure path is backend
// independent (doesn't rely on signature validation, which rustpython skips).
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
    assert!(call_result
        .unwrap_err()
        .to_string()
        .contains("has not been registered yet"));
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

// Only the RustPython backend includes a traceback (with line numbers) in the
// error string; PyO3 surfaces just "ExceptionType: message". This pins down that
// running PY_STDIO_GUARD as a *separate* unit beforehand does not shift the line
// numbers reported for subsequently-run user code (the line-3 raise stays line 3).
#[cfg(feature = "rustpython")]
#[tokio::test]
async fn test_error_line_number_not_shifted_by_stdio_guard() {
    let app = mock_app_handle().await;
    let runner = app.state::<PyRunner>().inner();
    runner.run(PY_STDIO_GUARD).await.unwrap();
    let code = "x = 1\ny = 2\nraise ValueError('boom')\n";
    let err = runner.run(code).await.unwrap_err().to_string();
    assert!(
        err.contains("line 3"),
        "the raise on line 3 must report line 3, not be shifted by the guard: {err}"
    );
}
