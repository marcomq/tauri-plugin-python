//  Tauri Python Plugin
//  © Copyright 2024, by Marco Mengelkoch
//  Licensed under MIT License, see License file for more details
//  git clone https://github.com/marcomq/tauri-plugin-python

use super::*;
use tauri::{test::{self, MockRuntime}, AppHandle};

/// Creates a mock Tauri app and initializes the PyRunner state.
/// It also runs some initial Python code to set up a variable and a function for testing.
async fn mock_app_handle() -> AppHandle<MockRuntime> {
    let app = test::mock_app();
    let runner = PyRunner::new();
    app.manage(runner);

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
    assert!(result.unwrap_err().to_string().contains("Cannot register after function called"));
}