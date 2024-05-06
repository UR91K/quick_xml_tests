use quick_xml::name::QName;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use flate2::read::GzDecoder;
use std::io::Write;
use std::env;

use quick_xml::Reader;
use quick_xml::events::Event;
use quick_xml::Writer;
use std::io::Cursor;

fn remove_tags(xml_data: Vec<u8>, tags_to_delete: Vec<&[u8]>, tag_to_search: Option<&[u8]>) -> Vec<u8> {
    let mut reader = Reader::from_reader(Cursor::new(xml_data));
    reader.trim_text(true);

    let mut writer = Writer::new(Cursor::new(Vec::new()));

    let mut buf = Vec::new();
    let mut in_target_tag = false;
    let mut depth = 0;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref event)) => {
                if let tag = tag_to_search.as_ref().map(|arg0: &&[u8]| QName(*arg0)) {
                    if let name = event.name() {
                        if let Some(QName(tag_bytes)) = tag {
                            if name == QName(tag_bytes) {
                                in_target_tag = true;
                                depth += 1;
                            }
                        }
                    }
                }
                if !in_target_tag && !tags_to_delete.iter().any(|&tag| {
                    if let name = event.name() {
                        name.into_inner() == tag
                    } else {
                        false
                    }
                }) {
                    writer.write_event(&Event::Start(event.clone())).unwrap();
                }
                if in_target_tag {
                    depth += 1;
                }
            }
            Ok(Event::End(ref event)) => {
                if let Some(tag) = tag_to_search.as_ref().map(|arg0: &&[u8]| QName(*arg0)) {
                    if let name = event.name() {
                        if name == tag {
                            in_target_tag = false;
                            depth -= 1;
                        }
                    }
                }
                if !in_target_tag && !tags_to_delete.iter().any(|&tag| {
                    if let name = event.name() {
                        name.into_inner() == tag
                    } else {
                        false
                    }
                }) {
                    writer.write_event(&Event::End(event.clone())).unwrap();
                }
                if in_target_tag {
                    depth -= 1;
                }
            }
            Ok(Event::Eof) => break,
            Ok(event) => {
                if !in_target_tag {
                    writer.write_event(&event).unwrap();
                }
            }
            Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
        }

        if depth == 0 {
            buf.clear();
        }
    }

    writer.into_inner().into_inner()
}

fn decode_als_data(file_path: &Path) -> Result<Vec<u8>, String> {
    let mut file = match File::open(&file_path) {
        Ok(file) => file,
        Err(err) => return Err(format!("Failed to open file {}: {}", file_path.display(), err)),
    };
    let mut gzip_decoder = GzDecoder::new(&mut file);
    let mut decompressed_data = Vec::new();
    if let Err(err) = gzip_decoder.read_to_end(&mut decompressed_data) {
        return Err(format!("Failed to decompress file {}: {}", file_path.display(), err));
    }
    Ok(decompressed_data)
}

fn main() {
    let current_dir = env::current_dir().expect("Failed to get current directory");
    println!("{:?}", current_dir);
    let file_path: PathBuf = current_dir.join("4 catjam.als");

    let decompressed_data = decode_als_data(&file_path).unwrap();

    let mut file = File::create("output.xml").expect("Unable to create file");

    let tags_to_delete = vec![
        "SideChain".as_bytes()
    ];

    let modified_xml_data = remove_tags(decompressed_data, tags_to_delete, None);

    file.write_all(&modified_xml_data).expect("Unable to write data to file");
}
