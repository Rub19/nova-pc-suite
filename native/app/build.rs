fn main() {
    slint_build::compile("ui/app.slint").expect("Interface Slint invalide");
    let mut resource = winresource::WindowsResource::new();
    resource.set_icon("icons/app.ico");
    resource.set("ProductName", "Nova PC Suite");
    resource.set("FileDescription", "Nova PC Suite");
    if let Err(error) = resource.compile() {
        println!("cargo:warning=icône de l'exe non intégrée : {error}");
    }
}
