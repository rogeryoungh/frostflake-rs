use std::{
    error::Error,
    io::{self, Write},
    path::PathBuf,
};

pub async fn download(url: &str, path: &str) -> Result<(), Box<dyn Error>> {
    let response = reqwest::get(url).await?;
    let bytes = response.bytes().await?;
    let mut file = std::fs::File::create(path)?;
    file.write(&bytes)?;
    Ok(())
}

pub fn prompt_user(message: &str) -> String {
    print!("{}", message);
    io::stdout().flush().expect("Failed to flush stdout"); // 确保提示信息立即输出
    let mut input = String::new();
    io::stdin().read_line(&mut input).expect("Failed to read line");
    return input.trim().to_uppercase();
}

pub fn current_dir_file(file_name: &str) -> PathBuf {
    let current_dir = std::env::current_dir().expect("Failed to get current dir");
    return current_dir.join(file_name);
}

pub fn wait_10s_exit() {
    println!("程序将在 10 秒后退出。");
    std::thread::sleep(std::time::Duration::from_secs(10));
    std::process::exit(1);
}
