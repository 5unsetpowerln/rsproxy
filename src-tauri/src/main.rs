// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod exchange;
mod http_parser;
mod proxy;
mod types;
mod common;
use std::sync::{Arc, Mutex};

use proxy::run_proxy_server;
use tauri::Manager;
use tokio::sync::{broadcast, mpsc};
use types::{Request, Response};

#[tokio::main]
async fn main() {
    // let headers = http_parser::headers::Headers::from_json(
    //     "{\"accept\":\"hello\",\"content-type\":\"text/html\"}",
    // );

    // ** shared state for proxy
    tauri::Builder::default()
        .setup(|app| {
            // * proxy
            let pilot_state = Arc::new(Mutex::new(false));

            let pilot_state_alt = pilot_state.clone();
            let proxy_app_handle = app.app_handle();
            tokio::spawn(async move {
                run_proxy_server(pilot_state_alt, proxy_app_handle).await;
            });

            app.listen_global("change_pilot_state", move |event| {
                let mut pilot_state = pilot_state.lock().unwrap();
                let pilot_state_str = event.payload().unwrap();
                if pilot_state_str == "true" {
                    *pilot_state = true;
                } else if pilot_state_str == "false" {
                    *pilot_state = false;
                } else {
                    println!("invalid pilot state: {}", pilot_state_str);
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
