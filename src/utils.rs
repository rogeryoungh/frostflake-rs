use std::io::{self, Write};

pub fn prompt_user(message: &str) -> String {
    print!("{}", message);
    io::stdout().flush().unwrap(); // 确保提示信息立即输出
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    return input.trim().to_uppercase();
}

pub fn wait_10s_exit() {
    println!("程序将在 10 秒后退出。");
    std::thread::sleep(std::time::Duration::from_secs(10));
    std::process::exit(1);
}
