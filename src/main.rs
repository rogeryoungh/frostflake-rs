pub mod registry;
pub mod server;
pub mod utils;
pub mod windows;

use utils::wait_10s_exit;

use crate::registry::{create_scheme_registration, get_registration_path};
use crate::server::start_server;
use crate::utils::prompt_user;

use std::{env, fs};

#[tokio::main]
async fn main() {
    let exe_path = env::current_exe().unwrap().display().to_string();
    env::set_current_dir(env::current_exe().unwrap().parent().unwrap()).unwrap();

    let need_register;
    println!("当前程序路径是：`{}`。", exe_path);
    if let Some(hkey_path_reg) = get_registration_path("cocogoat-control") {
        let hkey_path = hkey_path_reg.split('"').nth(1).unwrap();
        println!("在注册表中读取到注册信息，路径：`{}`。", hkey_path);
        need_register = hkey_path != exe_path;
    } else {
        println!("未在注册表中读取到自定义协议 `cocogoat-control`。");
        need_register = true;
    }

    if need_register {
        let excepted_exe_path = "C:\\Program Files\\frostflake-rs\\frostflake-rs.exe";
        if excepted_exe_path != exe_path {
            println!(
                "⚠️ 提醒：这个程序需要管理员权限哦~ 我们建议安装到路径 {}，这样可以更好地避免安全问题喵！",
                excepted_exe_path
            );
            if prompt_user("是否需要自动安装到建议路径？请输入 [Y/N] ") == "Y" {
                let path = std::path::Path::new(excepted_exe_path);
                let parent_dir = path.parent().unwrap();
                if !parent_dir.exists() {
                    fs::create_dir_all(parent_dir).unwrap();
                }
                fs::copy(&exe_path, path).unwrap();
                println!("移动完成，正在重新启动。");
                std::process::Command::new(path)
                    .spawn()
                    .expect("Failed to start the new process.");
                std::process::exit(0);
            }
        }
        if prompt_user("是否需要将注册路径更新为当前程序路径？请输入 [Y/N] ") == "Y" {
            create_scheme_registration("cocogoat-control", &exe_path);
        } else {
            println!("操作已被用户取消。");
            wait_10s_exit();
        }
    }

    start_server().await;
}
