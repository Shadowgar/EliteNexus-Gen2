//! Windows: embeds the app icon (part of the original Praetorium branding assets)
//! and the version information into Praetorium.exe, so Explorer, the taskbar and installers show them.

use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../../assets/branding/praetorium.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let out = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    let ico = out.join("praetorium.ico");
    std::fs::write(&ico, include_bytes!("../../assets/branding/praetorium.ico"))
        .expect("write Praetorium icon");

    let version = std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.0.0".into());
    let mut parts = version.split(['.', '-', '+']).map(|p| p.parse::<u16>().unwrap_or(0));
    let (major, minor, patch) = (parts.next().unwrap_or(0), parts.next().unwrap_or(0), parts.next().unwrap_or(0));
    let rc = format!(
        r#"1 ICON "{ico}"
1 VERSIONINFO
FILEVERSION {major},{minor},{patch},0
PRODUCTVERSION {major},{minor},{patch},0
FILEOS 0x40004
FILETYPE 0x1
BEGIN
  BLOCK "StringFileInfo"
  BEGIN
    BLOCK "040904B0"
    BEGIN
      VALUE "CompanyName", "Praetorium contributors"
      VALUE "FileDescription", "Praetorium - Elite Dangerous companion"
      VALUE "FileVersion", "{version}"
      VALUE "InternalName", "Praetorium"
      VALUE "LegalCopyright", "CMDR Alka Uruskan"
      VALUE "OriginalFilename", "Praetorium.exe"
      VALUE "ProductName", "Praetorium"
      VALUE "ProductVersion", "{version}"
    END
  END
  BLOCK "VarFileInfo"
  BEGIN
    VALUE "Translation", 0x409, 1200
  END
END
"#,
        ico = ico.display().to_string().replace('\\', "\\\\"),
    );
    let rc_path = out.join("app.rc");
    std::fs::write(&rc_path, rc).expect("write app.rc");
    embed_resource::compile(&rc_path, embed_resource::NONE).manifest_optional().expect("embed Windows resources");
}
