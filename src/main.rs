mod registry;

use registry::{create_scheme_registration, get_registration_path};

use std::env;
use std::io::{self, Write};

fn main() {
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
}

fn prompt_user(message: &str) -> bool {
    print!("{}", message);
    io::stdout().flush().unwrap(); // 确保提示信息立即输出
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    return input.trim().to_uppercase() == "Y";
}
