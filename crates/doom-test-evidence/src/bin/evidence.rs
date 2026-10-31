//! Checked offline evidence reproduction utility.
use doom_test_evidence::{
    MAX_COMPRESSED, MAX_DECODED, MAX_MANIFEST, Manifest, decode, encode, load, read_bounded,
};
use std::{fs, path::Path};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<_> = std::env::args().collect();
    match a.get(1).map(String::as_str) {
        Some("encode") if a.len()==4 => {
            let raw=read_bounded(Path::new(&a[2]),MAX_DECODED)?;
            fs::write(&a[3],encode(&raw)?)?;
        }
        Some("decode") if a.len()==5 => {
            let bytes=read_bounded(Path::new(&a[2]),MAX_COMPRESSED)?;
            let length:usize=a[4].parse()?;
            fs::write(&a[3],decode(&bytes,length,length)?)?;
        }
        Some("verify"|"extract") if a.len()==4 || a.len()==5 => {
            let root=Path::new(&a[2]);
            let bytes=read_bounded(Path::new(&a[3]),MAX_MANIFEST)?;
            let m=Manifest::parse(std::str::from_utf8(&bytes)?)?;
            let extract=a[1]=="extract";
            if extract && a.len()!=5 {return Err("extract destination required".into());}
            for entry in &m.entries {
                let decoded=load(root,entry)?;
                if extract {
                    let target=Path::new(&a[4]).join(&entry.original_path);
                    fs::create_dir_all(target.parent().ok_or("parent")?)?;
                    fs::write(target,decoded)?;
                }
            }
            println!("Verified {} original trace identities",m.entries.len());
        }
        _ => return Err("usage: evidence encode RAW ZST | decode ZST RAW LENGTH | verify ROOT MANIFEST | extract ROOT MANIFEST DEST".into()),
    }
    Ok(())
}
