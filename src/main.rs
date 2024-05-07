use std::fmt::Debug;
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

fn remove_tags(xml_data: &Vec<u8>, tags_to_delete: Vec<&str>) -> Vec<u8> {
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

#[derive(Debug, Clone)]
struct XmlTag {
    name: String,
    attributes: Vec<(String, String)>,
}

fn find_tags(xml_data: &[u8], search_query: &str) -> Vec<Vec<XmlTag>> {
    println!("Starting to find tags with search query: {}", search_query);
    let mut reader = Reader::from_reader(xml_data);
    reader.trim_text(true);

    let mut buf = Vec::new();
    let mut all_tags = Vec::new();
    let mut current_tags = Vec::new();

    let mut in_target_tag = false;
    let mut depth = 0;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref event)) => {
                let name = std::str::from_utf8(event.name().as_ref()).unwrap().to_string();
                if name == search_query {
                    println!("Entering target tag: {}", name);
                    in_target_tag = true;
                    depth = 0;
                } else if in_target_tag {
                    depth += 1;
                }
            }
            Ok(Event::Empty(ref event)) => {
                if in_target_tag && depth == 0 {
                    let name = std::str::from_utf8(event.name().as_ref()).unwrap().to_string();
                    println!("Found empty tag: {}", name);
                    let mut attributes = Vec::new();
                    for attr in event.attributes() {
                        let attr = attr.unwrap();
                        let key = std::str::from_utf8(attr.key.as_ref()).unwrap().to_string();
                        let value = std::str::from_utf8(attr.value.as_ref()).unwrap().to_string();
                        println!("Found attribute in {}: {} = {}", name, key, value);
                        attributes.push((key, value));
                    }
                    current_tags.push(XmlTag {
                        name,
                        attributes,
                    });
                }
            }
            Ok(Event::End(ref event)) => {
                let name = std::str::from_utf8(event.name().as_ref()).unwrap().to_string();
                if name == search_query {
                    println!("Exiting target tag: {}", name);
                    in_target_tag = false;
                    all_tags.push(current_tags.clone());
                    current_tags.clear();
                } else if in_target_tag {
                    depth -= 1;
                }
            }
            Ok(Event::Eof) => {
                println!("Reached end of XML data");
                break;
            }
            _ => (),
        }
        buf.clear();
    }

    println!("Found {} tag(s) matching search query: {}", all_tags.len(), search_query);
    all_tags
}

fn find_attribute(tags: &[XmlTag], tag_query: &str, attribute_query: &str) -> Option<String> {
    println!("Searching for attribute '{}' in tag '{}'", attribute_query, tag_query);
    for tag in tags {
        if tag.name == tag_query {
            for (key, value) in &tag.attributes {
                if key == attribute_query {
                    println!("Found attribute '{}' with value: {}", attribute_query, value);
                    return Some(value.clone());
                }
            }
        }
    }
    println!("Attribute '{}' not found in tag '{}'", attribute_query, tag_query);
    None
}

fn find_vst_plugins(xml_data: &[u8]) -> Vec<String> {
    println!("Starting to find VST plugins");
    let vst_plugin_tags = find_tags(xml_data, "VstPluginInfo");
    let mut vst_plugin_names = Vec::new();

    for tags in vst_plugin_tags {
        if let Some(plug_name) = find_attribute(&tags, "PlugName", "Value") {
            println!("Found VST plugin: {}", plug_name);
            vst_plugin_names.push(plug_name);
        }
    }

    println!("Found {} VST plugin(s)", vst_plugin_names.len());
    vst_plugin_names
}

fn find_vst3_plugins(xml_data: &[u8]) -> Vec<String> {
    println!("Starting to find VST3 plugins");
    let vst3_plugin_tags = find_tags(xml_data, "Vst3PluginInfo");
    println!("VST3 {:?}", vst3_plugin_tags);
    let mut vst3_plugin_names = Vec::new();

    for tags in vst3_plugin_tags {
        if let Some(name) = find_attribute(&tags, "Name", "Value") {
            println!("Found VST plugin: {}", name);
            vst3_plugin_names.push(name);
        }
    }

    println!("Found {} VST3 plugin(s)", vst3_plugin_names.len());
    vst3_plugin_names
}

fn main() {
    let input_file = "4 catjam.xml";
    // let output_file = "output.xml";

    let file_path = Path::new(input_file);
    let xml_data = fs::read(file_path).expect("Unable to read file");

    let tags_to_delete = vec![
        "Buffer"
    ];
    let cleaned_xml_data = remove_tags(&xml_data, tags_to_delete);

    let vst_plugin_names = find_vst_plugins(&xml_data);
    let vst3_plugin_names = find_vst3_plugins(&xml_data);
    println!("VST2 plugins found: {:?}", vst_plugin_names);
    println!("VST3 plugins found: {:?}", vst3_plugin_names);
}