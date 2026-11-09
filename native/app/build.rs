fn main() {
    let config = slint_build::CompilerConfiguration::new().with_debug_info(std::env::var_os("CARGO_FEATURE_E2E").is_some());
    slint_build::compile_with_config("ui/app.slint", config).expect("Interface Slint invalide");
    let mut resource = winresource::WindowsResource::new();
    resource.set_icon("icons/app.ico");
    resource.set("ProductName", "Nova PC Suite");
    resource.set("FileDescription", "Nova PC Suite");
    if let Err(error) = resource.compile() {
        println!("cargo:warning=icône de l'exe non intégrée : {error}");
    }
}
