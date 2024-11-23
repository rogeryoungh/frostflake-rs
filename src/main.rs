use std::env;
use std::io::{self, Write};
use winreg::RegKey;

fn main() {
    let exe_path = env::current_exe().unwrap();
    println!("Current executable path is `{}`", exe_path.display());
    if let Some(hkey_path_reg) = get_registration_path("cocogoat-control") {
        let hkey_path = hkey_path_reg.split('"').nth(1).unwrap();
        let exe_path_reg = format!("\"{}\" \"%1\"", exe_path.display());
        println!(
            "Found registration for scheme `cocogoat-control` at path `{}`",
            hkey_path
        );

        if hkey_path != exe_path_reg {
            if prompt_user("Do you want to update the registration to point to current executable? [Y/N] ") {
                regist_path("cocogoat-control", &exe_path_reg);
            } else {
                eprintln!("Operation cancelled by the user");
            }
        }
    } else {
        println!("Not found registration for scheme `cocogoat-control`");
        let exe_path_reg = format!("\"{}\" \"%1\"", exe_path.display());
        if prompt_user("Do you want to add the registration to point to current executable? [y/N] ") {
            regist_path("cocogoat-control", &exe_path_reg);
        } else {
            eprintln!("Operation cancelled by the user");
        }
    }
}

fn get_registration_path(scheme: &str) -> Option<String> {
    let hkey_root = RegKey::predef(winreg::enums::HKEY_CLASSES_ROOT);
    match hkey_root.open_subkey(String::from(scheme)) {
        Ok(key) => match key.open_subkey("shell\\open\\command") {
            Ok(subkey) => match subkey.get_value("") {
                Ok(value) => Some(value),
                Err(_) => None,
            },
            Err(_) => None,
        },
        Err(_) => None,
    }
}

fn regist_path(scheme: &str, path: &str) {
    let hkey_root = RegKey::predef(winreg::enums::HKEY_CLASSES_ROOT);
    if hkey_root.open_subkey(scheme).is_ok() {
        hkey_root.delete_subkey_all(scheme).unwrap();
    }

    // https://learn.microsoft.com/en-us/previous-versions/windows/internet-explorer/ie-developer/platform-apis/aa767914(v=vs.85)
    // HKEY_CLASSES_ROOT
    //    alert
    //       (Default) = "URL:Alert Protocol"
    //       URL Protocol = ""
    //       DefaultIcon
    //          (Default) = "alert.exe,1"
    //       shell
    //          open
    //             command
    //                (Default) = "C:\Program Files\Alert\alert.exe" "%1"

    let uri_protocol_str = format!("URL:{} Protocol", scheme);
    let icon_path_str = format!("{},1", path);
    let command_str = format!("\"{}\" \"%1\"", path);

    let (hkey_scheme, _) = hkey_root
        .create_subkey(scheme)
        .expect("Failed to edit registry.\nDo you have Administrator permission?");

    hkey_scheme.set_value("", &uri_protocol_str).unwrap();
    hkey_scheme.set_value("URL Protocol", &"").unwrap();
    let (hkey_icon, _) = hkey_scheme.create_subkey("DefaultIcon").unwrap();
    let (hkey_open, _) = hkey_scheme.create_subkey("shell\\open\\command").unwrap();
    hkey_icon.set_value("", &icon_path_str).unwrap();
    hkey_open.set_value("", &command_str).unwrap();
}

fn prompt_user(message: &str) -> bool {
    print!("{}", message);
    io::stdout().flush().unwrap(); // 确保提示信息立即输出
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    return input.trim().to_uppercase() == "Y";
}
