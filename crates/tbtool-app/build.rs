#[cfg(windows)]
fn main() {
    let mut resources = winres::WindowsResource::new();
    resources.set_icon("assets/icon.ico");
    resources.set_manifest(
        r#"<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
      <requestedPrivileges>
        <requestedExecutionLevel level="requireAdministrator" uiAccess="false" />
      </requestedPrivileges>
    </security>
  </trustInfo>
</assembly>"#,
    );
    resources
        .compile()
        .expect("failed to compile Windows resources");

    // GNU ld discards the resource object when winres packages it in a static
    // archive because no symbol references it. Link it explicitly instead.
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("gnu") {
        let resource = std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("resource.o");
        println!("cargo:rustc-link-arg-bins={}", resource.display());
    }
}

#[cfg(not(windows))]
fn main() {}
