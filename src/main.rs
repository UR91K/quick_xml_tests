use std::fs::File;
use std::io::{Read, Write, Cursor};
use std::path::Path;
use std::fs;

use flate2::read::GzDecoder;
use quick_xml::Reader;
use quick_xml::events::{BytesText, Event};
use quick_xml::Writer;

fn load_file_data(file_path: &Path) -> Result<Vec<u8>, String> {
    match fs::read(file_path) {
        Ok(data) => Ok(data),
        Err(err) => Err(format!("Failed to read file {}: {}", file_path.display(), err)),
    }
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

fn remove_tags(xml_data: Vec<u8>, tags_to_delete: Vec<&str>) -> Vec<u8> {
    let mut reader = Reader::from_reader(xml_data.as_slice());
    reader.trim_text(true);

    let mut writer = Writer::new(Cursor::new(Vec::new()));
    let mut buf = Vec::new();

    let mut in_target_tag = false;
    let mut depth = 0;
    let mut indent = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref event)) => {
                let name = event.name();
                let name_str = std::str::from_utf8(name.as_ref()).unwrap();
                if tags_to_delete.iter().any(|&tag| tag == name_str) {
                    in_target_tag = true;
                    depth = 1;
                } else if !in_target_tag {
                    writer.write_event(&Event::Text(BytesText::new(&indent))).unwrap();
                    writer.write_event(&Event::Start(event.clone())).unwrap();
                    writer.write_event(&Event::Text(BytesText::new("\n"))).unwrap();
                    indent.push_str("    ");
                } else {
                    depth += 1;
                }
            }
            Ok(Event::End(ref event)) => {
                let name = event.name();
                let name_str = std::str::from_utf8(name.as_ref()).unwrap();
                if in_target_tag {
                    depth -= 1;
                    if depth == 0 {
                        in_target_tag = false;
                    }
                } else if !tags_to_delete.iter().any(|&tag| tag == name_str) {
                    if indent.len() >= 4 {
                        indent.truncate(indent.len() - 4);
                    }
                    writer.write_event(&Event::Text(BytesText::new(&indent))).unwrap();
                    writer.write_event(&Event::End(event.clone())).unwrap();
                    writer.write_event(&Event::Text(BytesText::new("\n"))).unwrap();
                }
            }
            Ok(Event::Text(ref event)) => {
                if !in_target_tag {
                    writer.write_event(&Event::Text(event.clone())).unwrap();
                }
            }
            Ok(Event::Empty(ref event)) => {
                let name = event.name();
                let name_str = std::str::from_utf8(name.as_ref()).unwrap();
                if !tags_to_delete.iter().any(|&tag| tag == name_str) && !in_target_tag {
                    writer.write_event(&Event::Text(BytesText::new(&indent))).unwrap();
                    writer.write_event(&Event::Empty(event.clone())).unwrap();
                    writer.write_event(&Event::Text(BytesText::new("\n"))).unwrap();
                }
            }
            Ok(Event::Eof) => break,
            _ => (),
        }
        buf.clear();
    }

    writer.into_inner().into_inner()
}

fn main() {
    let input_file = "input.xml";
    let output_file = "output.xml";

    let file_path = Path::new(input_file);
    let xml_data = fs::read(file_path).expect("Unable to read file");

    let tags_to_delete = vec!["SideChain"];
    let modified_xml_data = remove_tags(xml_data, tags_to_delete);

    let mut file = File::create(output_file).expect("Unable to create file");
    file.write_all(&modified_xml_data)
        .expect("Unable to write data to file");

    println!("Output written to {}", output_file);
}