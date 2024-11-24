fn main() {
    if cfg!(target_os = "windows") {
        let mut res = winres::WindowsResource::new();
        res.set_manifest_file("./assets/app.manifest");
        res.compile().expect("Failed to compile resources");
    }
}
