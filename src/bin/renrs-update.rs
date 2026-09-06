use std::io::Write;
use std::path::Path;
fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        ["keygen",prefix]=>{
            let private=format!("{prefix}.key");let public=format!("{prefix}.pub");
            if Path::new(&private).exists()||Path::new(&public).exists(){return Err("key files exist".into());}
            let mut secret=[0;32];getrandom::fill(&mut secret).map_err(|error| error.to_string())?;
            let mut options=std::fs::OpenOptions::new();options.write(true).create_new(true);
            #[cfg(unix)] {use std::os::unix::fs::OpenOptionsExt;options.mode(0o600);}
            options.open(private)?.write_all(&secret)?;
            std::fs::OpenOptions::new().write(true).create_new(true).open(public)?.write_all(&ed25519_dalek::SigningKey::from_bytes(&secret).verifying_key().to_bytes())?;
        }
        ["create",old,new,out,key]=>{let count=renrs::updates::create(Path::new(old),Path::new(new),Path::new(out),&key_bytes(key)?)?;println!("Created signed update with {count} changed resources");}
        ["apply",old,patch,out,key]=>{renrs::updates::apply(Path::new(old),Path::new(patch),Path::new(out),&key_bytes(key)?)?;println!("Verified update: {out}");}
        _=>return Err("usage: renrs-update keygen <prefix> | create <old.renrs> <new.renrs> <patch-dir> <private.key> | apply <old.renrs> <patch-dir> <new.renrs> <trusted.pub>".into()),
    }
    Ok(())
}
fn key_bytes(path: &str) -> Result<[u8; 32], Box<dyn std::error::Error>> {
    std::fs::read(path)?
        .try_into()
        .map_err(|_| "key must contain exactly 32 bytes".into())
}
