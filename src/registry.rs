use std::error::Error;

use winreg::RegKey;

pub fn get_registration_path(scheme: &str) -> Result<String, Box<dyn Error>> {
    let hkey_root = RegKey::predef(winreg::enums::HKEY_CLASSES_ROOT);
    let key = hkey_root.open_subkey(String::from(scheme))?;
    let subkey = key.open_subkey("shell\\open\\command")?;
    let value: String = subkey.get_value("")?;
    let path = value.split('"').nth(1).ok_or("Failed to parse path")?;
    Ok(path.to_string())
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

pub fn create_scheme_registration(scheme: &str, path: &str) -> Result<(), Box<dyn Error>> {
    let hkey_root = RegKey::predef(winreg::enums::HKEY_CLASSES_ROOT);
    if hkey_root.open_subkey(scheme).is_ok() {
        hkey_root
            .delete_subkey_all(scheme)
            .expect("Failed to delete the old registration");
    }

    let uri_protocol_str = format!("URL:{scheme} Protocol");
    let icon_path_str = format!("{path},1");
    let command_str = format!("\"{path}\" \"%1\"");

    let (hkey_scheme, _) = hkey_root.create_subkey(scheme)?;

    hkey_scheme.set_value("", &uri_protocol_str)?;
    hkey_scheme.set_value("URL Protocol", &"")?;
    let (hkey_icon, _) = hkey_scheme.create_subkey("DefaultIcon")?;
    let (hkey_open, _) = hkey_scheme.create_subkey("shell\\open\\command")?;
    hkey_icon.set_value("", &icon_path_str)?;
    hkey_open.set_value("", &command_str)?;

    Ok(())
}
