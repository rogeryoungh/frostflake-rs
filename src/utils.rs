use std::io::{self, Write};

pub fn prompt_user(message: &str) -> bool {
    print!("{}", message);
    io::stdout().flush().unwrap(); // 确保提示信息立即输出
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    return input.trim().to_uppercase() == "Y";
}
