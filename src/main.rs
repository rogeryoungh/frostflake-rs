pub mod registry;
pub mod server;
pub mod utils;
pub mod windows;

use utils::wait_10s_exit;

use crate::registry::{create_scheme_registration, get_registration_path};
use crate::server::start_server;
use crate::utils::prompt_user;

use std::path::Path;
use std::{env, fs, process};

#[tokio::main]
async fn main() {
    let exe_path = env::current_exe().expect("Failed to get current executable path");
    let exe_dir = exe_path.parent().unwrap();
    env::set_current_dir(exe_dir).expect("Failed to set current directory");
    let exe_path_str = exe_path.display().to_string();

    println!("当前程序路径是：`{}`。", exe_path_str);

    let need_register = match get_registration_path("cocogoat-control") {
        Ok(hkey_path) => {
            println!("在注册表中读取到注册信息，路径：`{}`。", hkey_path);
            hkey_path != exe_path_str
        },
        Err(err) => {
            println!("未在注册表中读取到自定义协议 `cocogoat-control`。");
            eprintln!("{}", err);
            true
        },
    };

    if need_register {
        let excepted_exe_path_str: &str = "C:\\Program Files\\frostflake-rs\\frostflake-rs.exe";
        if excepted_exe_path_str != exe_path_str {
            println!(
                "⚠️ 提醒：这个程序需要管理员权限喵~\n我们建议安装到路径 {}，这样可以更好地避免安全问题喵！",
                excepted_exe_path_str
            );
            if prompt_user("是否需要自动安装到建议路径？请输入 [Y/N] ") == "Y" {
                let excepted_exe_path = Path::new(excepted_exe_path_str);
                let excepted_exe_dir = excepted_exe_path.parent().unwrap();
                if !excepted_exe_dir.exists() {
                    fs::create_dir_all(excepted_exe_dir).expect("Failed to create directory");
                }
                fs::copy(&exe_path_str, excepted_exe_path).expect("Failed to copy the executable");
                println!("移动完成，正在重新启动。");
                process::Command::new(excepted_exe_path)
                    .spawn()
                    .expect("Failed to start the new process");
                process::exit(0);
            }
        }
        if prompt_user("是否需要将注册路径更新为当前程序路径？请输入 [Y/N] ") == "Y" {
            create_scheme_registration("cocogoat-control", &exe_path_str)
                .expect("编辑注册表失败了喵！\n检查一下有没有开启管理员权限呀？(≧◡≦)");
            println!("注册表已更新。");
        } else {
            println!("操作已被用户取消。");
            wait_10s_exit();
        }
    }

    start_server("127.0.0.1:32333").await;
}
