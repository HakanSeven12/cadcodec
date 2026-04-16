//! Quick diagnostic dump of a DWG file's dictionary/object graph,
//! mimicking the AutoCAD AUDIT style output.

use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use acadrust::objects::ObjectType;
use acadrust::types::Handle;
use acadrust::{CadDocument, DwgReader};

fn object_kind(obj: &ObjectType) -> &'static str {
    match obj {
        ObjectType::Dictionary(_) => "Dictionary",
        ObjectType::Layout(_) => "Layout",
        ObjectType::XRecord(_) => "XRecord",
        ObjectType::Group(_) => "Group",
        ObjectType::MLineStyle(_) => "MLineStyle",
        ObjectType::ImageDefinition(_) => "ImageDefinition",
        ObjectType::PlotSettings(_) => "PlotSettings",
        ObjectType::MultiLeaderStyle(_) => "MultiLeaderStyle",
        ObjectType::TableStyle(_) => "TableStyle",
        ObjectType::Scale(_) => "Scale",
        ObjectType::SortEntitiesTable(_) => "SortEntitiesTable",
        ObjectType::DictionaryVariable(_) => "DictionaryVariable",
        ObjectType::VisualStyle(_) => "VisualStyle",
        ObjectType::Material(_) => "Material",
        ObjectType::ImageDefinitionReactor(_) => "ImageDefinitionReactor",
        ObjectType::GeoData(_) => "GeoData",
        ObjectType::SpatialFilter(_) => "SpatialFilter",
        ObjectType::RasterVariables(_) => "RasterVariables",
        ObjectType::BookColor(_) => "BookColor",
        ObjectType::PlaceHolder(_) => "PlaceHolder",
        ObjectType::DictionaryWithDefault(_) => "DictionaryWithDefault",
        ObjectType::WipeoutVariables(_) => "WipeoutVariables",
        ObjectType::Unknown { .. } => "Unknown",
    }
}

fn unknown_has_raw(obj: &ObjectType) -> bool {
    matches!(obj, ObjectType::Unknown { raw_dwg_data: Some(_), .. })
}

fn is_writable(doc: &CadDocument, h: Handle) -> bool {
    match doc.objects.get(&h) {
        None => false,
        Some(ObjectType::Unknown { raw_dwg_data, .. }) => raw_dwg_data.is_some(),
        Some(_) => true,
    }
}

fn main() {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "tests/roundtrip/samplekitchen.dwg".to_string());
    println!("Auditing: {}", path);

    let mut reader = DwgReader::from_file(Path::new(&path)).expect("open");
    let doc = reader.read().expect("read");

    println!(
        "Version: {:?}  Objects: {}  Entities: {}",
        doc.version,
        doc.objects.len(),
        doc.entity_count()
    );

    // Object kind tallies
    let mut kinds: BTreeMap<&'static str, usize> = BTreeMap::new();
    for obj in doc.objects.values() {
        *kinds.entry(object_kind(obj)).or_insert(0) += 1;
    }
    println!("\nObject kinds:");
    for (k, n) in &kinds {
        println!("  {:<25} {}", k, n);
    }

    // Unknown object stats
    let unknown_with_raw = doc
        .objects
        .values()
        .filter(|o| matches!(o, ObjectType::Unknown { raw_dwg_data: Some(_), .. }))
        .count();
    let unknown_no_raw = doc
        .objects
        .values()
        .filter(|o| matches!(o, ObjectType::Unknown { raw_dwg_data: None, .. }))
        .count();
    println!(
        "\nUnknown: {} total  (with raw DWG: {},  without: {})",
        unknown_with_raw + unknown_no_raw,
        unknown_with_raw,
        unknown_no_raw
    );

    // Named object dictionary traversal
    let named = doc.header.named_objects_dict_handle;
    println!("\nNamed Object Dictionary handle: {:#X}", named.value());
    if let Some(ObjectType::Dictionary(d)) = doc.objects.get(&named) {
        println!("  Entries ({}):", d.entries.len());
        for (name, h) in &d.entries {
            let kind = doc.objects.get(h).map(object_kind).unwrap_or("<MISSING>");
            println!("    {:<30} {:#X}  kind={}", name, h.value(), kind);
        }
    } else {
        println!("  (named object dictionary not found)");
    }

    // Walk ALL dictionaries and find dangling handle entries
    println!("\n── DANGLING DICTIONARY REFERENCES ──");
    let mut dangling: Vec<(Handle, String, Handle, String)> = Vec::new();
    for (dict_h, obj) in &doc.objects {
        let entries: &[(String, Handle)] = match obj {
            ObjectType::Dictionary(d) => &d.entries,
            ObjectType::DictionaryWithDefault(d) => &d.entries,
            _ => continue,
        };
        for (name, h) in entries {
            if h.is_null() {
                continue;
            }
            // Object missing or an Unknown without raw data = won't be re-written
            match doc.objects.get(h) {
                None => dangling.push((*dict_h, name.clone(), *h, "MISSING".to_string())),
                Some(ObjectType::Unknown { raw_dwg_data, type_name, .. }) if raw_dwg_data.is_none() => {
                    dangling.push((*dict_h, name.clone(), *h, format!("Unknown({})_no_raw", type_name)))
                }
                _ => {}
            }
        }
    }
    if dangling.is_empty() {
        println!("  None found.");
    } else {
        println!("  {} dangling references:", dangling.len());
        for (dh, name, th, reason) in dangling.iter().take(30) {
            println!(
                "    AcDbDictionary({:#X})  entry {:<30} -> {:#X}   [{}]",
                dh.value(),
                name,
                th.value(),
                reason
            );
        }
        if dangling.len() > 30 {
            println!("    ... and {} more", dangling.len() - 30);
        }
    }

    // Detect unreachable (orphaned) objects — in objects map but not reachable
    // from named objects dictionary via BFS over dictionary entries
    println!("\n── UNREACHABLE OBJECTS ──");
    let mut reachable: HashSet<Handle> = HashSet::new();
    let mut queue: Vec<Handle> = vec![named];
    while let Some(h) = queue.pop() {
        if !reachable.insert(h) {
            continue;
        }
        if let Some(obj) = doc.objects.get(&h) {
            let entries: &[(String, Handle)] = match obj {
                ObjectType::Dictionary(d) => &d.entries,
                ObjectType::DictionaryWithDefault(d) => &d.entries,
                _ => &[],
            };
            for (_, eh) in entries {
                if !eh.is_null() {
                    queue.push(*eh);
                }
            }
            // Follow extension dictionaries (xdictionary_handle)
            if let ObjectType::Dictionary(d) = obj {
                if let Some(xd) = d.xdictionary_handle {
                    queue.push(xd);
                }
            }
        }
    }
    let unreachable: Vec<_> = doc
        .objects
        .keys()
        .filter(|h| !reachable.contains(h))
        .collect();
    println!(
        "  {} objects unreachable from Named Object Dictionary (may be reached via entities' xdicts)",
        unreachable.len()
    );
    let mut by_kind: BTreeMap<&'static str, usize> = BTreeMap::new();
    for h in &unreachable {
        let k = object_kind(&doc.objects[h]);
        *by_kind.entry(k).or_insert(0) += 1;
    }
    for (k, n) in &by_kind {
        println!("    {:<25} {}", k, n);
    }

    // Check ACAD_TABLESTYLE dictionary
    println!("\n── ACAD_TABLESTYLE check ──");
    let mut table_style_dict: Option<Handle> = None;
    if let Some(ObjectType::Dictionary(d)) = doc.objects.get(&named) {
        for (n, h) in &d.entries {
            if n.eq_ignore_ascii_case("ACAD_TABLESTYLE") {
                table_style_dict = Some(*h);
                break;
            }
        }
    }
    match table_style_dict {
        None => println!("  ACAD_TABLESTYLE entry missing from Named Object Dictionary"),
        Some(h) => {
            println!("  ACAD_TABLESTYLE dictionary: {:#X}", h.value());
            if let Some(ObjectType::Dictionary(d)) = doc.objects.get(&h) {
                println!("    Entries ({}):", d.entries.len());
                let mut has_standard = false;
                for (n, eh) in &d.entries {
                    let ok = is_writable(&doc, *eh);
                    println!(
                        "      {:<30} {:#X}   writable={}",
                        n,
                        eh.value(),
                        ok
                    );
                    if n.eq_ignore_ascii_case("Standard") {
                        has_standard = true;
                    }
                }
                if !has_standard {
                    println!("    WARNING: no 'Standard' TableStyle entry");
                }
            } else {
                println!("    ERROR: object is not a Dictionary");
            }
        }
    }

    // CTABLESTYLE DictionaryVariable search
    println!("\n── CTABLESTYLE DictionaryVariable search ──");
    let mut found_ctable = false;
    for obj in doc.objects.values() {
        if let ObjectType::DictionaryVariable(dv) = obj {
            // DictionaryVariable doesn't store its key, but we can see its value
            println!(
                "  DictionaryVariable: value=\"{}\"  schema_number={}",
                dv.value, dv.schema_number
            );
            found_ctable = true;
        }
    }
    if !found_ctable {
        println!("  No DictionaryVariable objects present.");
    }
    let _ = unknown_has_raw;
}
