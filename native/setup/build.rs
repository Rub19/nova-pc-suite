fn main() {
    let conf = std::fs::read_to_string("../app/Cargo.toml").expect("Cargo.toml");
    let version = conf.split("version = \"").nth(1).and_then(|r| r.split('"').next()).expect("version");
    println!("cargo:rustc-env=NOVA_VERSION={version}");
    println!("cargo:rerun-if-changed=../app/Cargo.toml");
    println!("cargo:rerun-if-changed=../app/icons/app.ico");
    slint_build::compile("ui/setup.slint").expect("slint");
    let mut res = winresource::WindowsResource::new();
    res.set_icon("../app/icons/app.ico");
    res.set("ProductName", "Nova PC Suite");
    res.set("FileDescription", "Installation de Nova PC Suite");
    res.set("CompanyName", "Rub19");
    res.compile().expect("ressources Windows");
}
