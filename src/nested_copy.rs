use std::collections::{HashMap, HashSet};
use crate::{CadDocument, EntityType};
use crate::tables::{Layer, LineType};
use crate::types::Handle;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NestedCopyMode { #[default] Insert, Bind }

/// Stable destination names for all placements of one nested-copy operation.
#[derive(Clone, Debug, Default)]
pub struct NestedCopySymbolNames {
    layers: HashMap<String,String>,
    line_types: HashMap<String,String>,
}

fn destinations(names: impl Iterator<Item=String>,mode:NestedCopyMode)->HashMap<String,String> {
    let mut names=names.collect::<Vec<_>>();
    names.sort_by_key(|name|name.to_ascii_uppercase());
    let mut used=names.iter().map(|name|name.to_ascii_uppercase()).collect::<HashSet<_>>();
    let mut result=HashMap::new();
    for source in names {
        let Some((prefix,local))=source.rsplit_once('|') else{continue;};
        // Multi-level external-reference binding needs its own dependency walk.
        if prefix.contains('|') {continue;}
        let destination=match mode {
            NestedCopyMode::Insert=>local.to_owned(),
            NestedCopyMode::Bind=>{
                let mut index=0usize;
                loop {
                    let name=format!("{prefix}${index}${local}");
                    if !used.contains(&name.to_ascii_uppercase()){break name;}
                    index+=1;
                }
            },
        };
        used.insert(destination.to_ascii_uppercase());
        result.insert(source.to_ascii_uppercase(),destination);
    }
    result
}

impl CadDocument {
    pub fn nested_copy_symbol_names(&self,mode:NestedCopyMode)->NestedCopySymbolNames {
        NestedCopySymbolNames {
            layers:destinations(self.layers.iter().map(|layer|layer.name.clone()),mode),
            line_types:destinations(self.line_types.iter().map(|line|line.name.clone()),mode),
        }
    }

    /// Localize supported layer and simple-linetype dependencies of extracted entities.
    /// Unsupported style-bearing entities retain their original imported references.
    /// Returns the number of entities whose imported references were retained.
    pub fn localize_nested_copy_symbols(&mut self,entities:&mut [EntityType],names:&NestedCopySymbolNames)->usize {
        let mut retained=0;
        for entity in entities {
            let common=entity.common();
            let external=common.layer.contains('|')||common.linetype.contains('|');
            if !external {continue;}
            if common.material_handle.is_some_and(|handle| !handle.is_null())
                || common.plotstyle_handle.is_some_and(|handle| !handle.is_null())
                || !matches!(entity,EntityType::Point(_)|EntityType::Line(_)|EntityType::Circle(_)|EntityType::Arc(_)|EntityType::Ellipse(_)|EntityType::Polyline(_)|EntityType::Polyline2D(_)|EntityType::Polyline3D(_)|EntityType::LwPolyline(_)|EntityType::Spline(_)|EntityType::Helix(_)|EntityType::Solid(_)|EntityType::Face3D(_)|EntityType::Ray(_)|EntityType::XLine(_)|EntityType::Mesh(_)|EntityType::PolyfaceMesh(_)) {
                retained+=1;continue;
            }
            let plan=(||->Option<(Option<Layer>,Vec<LineType>,String,String)> {
                let common=entity.common();
                let mut line_types=Vec::new();
                let local_line=|source:&str,output:&mut Vec<LineType>|->Option<String> {
                    if !source.contains('|'){return Some(source.to_owned());}
                    let destination=names.line_types.get(&source.to_ascii_uppercase())?.clone();
                    if let Some(existing)=self.line_types.get(&destination){return Some(existing.name.clone());}
                    let mut line=self.line_types.get(source)?.clone();
                    if line.elements.iter().any(|element|element.complex.is_some()){return None;}
                    line.name=destination.clone();line.handle=Handle::NULL;line.xref_dependent=false;
                    if !output.iter().any(|existing|existing.name.eq_ignore_ascii_case(&destination)){output.push(line);}
                    Some(destination)
                };
                let mut layer=None;
                let layer_name=if common.layer.contains('|') {
                    let destination=names.layers.get(&common.layer.to_ascii_uppercase())?.clone();
                    if let Some(existing)=self.layers.get(&destination){existing.name.clone()}
                    else {
                        let mut imported=self.layers.get(&common.layer)?.clone();
                        if !imported.material.is_null()||!imported.plotstyle_handle.is_null(){return None;}
                        let prefix=common.layer.rsplit_once('|')?.0;
                        let prefixed=format!("{prefix}|{}",imported.line_type);
                        let source_line=if !imported.line_type.contains('|')&&self.line_types.get(&prefixed).is_some(){prefixed}else{imported.line_type.clone()};
                        imported.line_type=local_line(&source_line,&mut line_types)?;
                        imported.name=destination.clone();imported.handle=Handle::NULL;
                        imported.flags.xref_dependent=false;imported.xref_block_record_handle=Handle::NULL;
                        layer=Some(imported);destination
                    }
                }else{common.layer.clone()};
                let line_name=local_line(&common.linetype,&mut line_types)?;
                Some((layer,line_types,layer_name,line_name))
            })();
            let Some((layer,line_types,layer_name,line_name))=plan else{retained+=1;continue;};
            for mut line in line_types {
                if self.line_types.get(&line.name).is_none(){line.handle=self.allocate_handle();self.line_types.add_or_replace(line);}
            }
            if let Some(mut layer)=layer {
                layer.handle=self.allocate_handle();self.layers.add_or_replace(layer);
            }
            let line_handle=self.line_types.get(&line_name).map(|line|line.handle);
            let common=entity.common_mut();
            common.layer=layer_name;common.linetype=line_name;common.linetype_handle=line_handle;
        }
        retained
    }
}
