mod registry;
mod server;
mod utils;

use registry::{create_scheme_registration, get_registration_path};
use server::start_server;
use utils::prompt_user;

use std::env;

#[tokio::main]
async fn main() {
    let exe_path = env::current_exe().unwrap().display().to_string();
    println!("Current executable path is `{}`", exe_path);
    if let Some(hkey_path_reg) = get_registration_path("cocogoat-control") {
        let hkey_path = hkey_path_reg.split('"').nth(1).unwrap();
        println!(
            "Found registration for scheme `cocogoat-control` at path `{}`",
            hkey_path
        );

        if hkey_path != exe_path {
            if prompt_user("Do you want to update the registration to point to current executable? [Y/N] ") {
                create_scheme_registration("cocogoat-control", &exe_path);
            } else {
                panic!("Operation cancelled by the user");
            }
        }
    } else {
        println!("Not found registration for scheme `cocogoat-control`");
        if prompt_user("Do you want to add the registration to point to current executable? [y/N] ") {
            create_scheme_registration("cocogoat-control", &exe_path);
        } else {
            panic!("Operation cancelled by the user");
        }
    }

    start_server().await;
}
