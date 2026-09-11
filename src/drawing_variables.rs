use crate::objects::{Dictionary, DictionaryVariable, ObjectType};
use crate::types::Handle;
use crate::CadDocument;

const VARIABLES: &str = "AcDbVariableDictionary";

impl CadDocument {
    /// Read a serialized drawing variable without modifying its dictionary.
    pub fn drawing_variable(&self, name: &str) -> Option<&str> {
        let ObjectType::Dictionary(root) = self.objects.get(&self.header.named_objects_dict_handle)? else { return None; };
        let ObjectType::Dictionary(variables) = self.objects.get(&root.get(VARIABLES)?)? else { return None; };
        let ObjectType::DictionaryVariable(value) = self.objects.get(&variables.get(name)?)? else { return None; };
        Some(&value.value)
    }

    /// Update or create a drawing variable, retaining existing object handles.
    /// Conflicting dictionary/object types are left intact.
    pub fn set_drawing_variable(&mut self, name: &str, value: &str) -> bool {
        if name.is_empty() { return false; }
        let mut root_handle = self.header.named_objects_dict_handle;
        if root_handle == Handle::NULL {
            root_handle = self.allocate_handle();
            let mut root = Dictionary::new();
            root.handle = root_handle;
            self.objects.insert(root_handle, ObjectType::Dictionary(root));
            self.header.named_objects_dict_handle = root_handle;
        }
        let Some(ObjectType::Dictionary(root)) = self.objects.get(&root_handle) else { return false; };
        let variables_handle = root.get(VARIABLES);
        let variables_handle = if let Some(handle) = variables_handle {
            if !matches!(self.objects.get(&handle), Some(ObjectType::Dictionary(_))) { return false; }
            handle
        } else {
            let handle = self.allocate_handle();
            let mut variables = Dictionary::new();
            variables.handle = handle;
            variables.owner = root_handle;
            self.objects.insert(handle, ObjectType::Dictionary(variables));
            if let Some(ObjectType::Dictionary(root)) = self.objects.get_mut(&root_handle) {
                root.add_entry(VARIABLES, handle);
            }
            handle
        };
        let Some(ObjectType::Dictionary(variables)) = self.objects.get(&variables_handle) else { return false; };
        let packed = value.to_owned();
        if let Some(handle) = variables.get(name) {
            let Some(ObjectType::DictionaryVariable(variable)) = self.objects.get_mut(&handle) else { return false; };
            variable.value = packed;
        } else {
            let handle = self.allocate_handle();
            let mut variable = DictionaryVariable::new(name, &packed);
            variable.handle = handle;
            variable.owner_handle = variables_handle;
            self.objects.insert(handle, ObjectType::DictionaryVariable(variable));
            if let Some(ObjectType::Dictionary(variables)) = self.objects.get_mut(&variables_handle) {
                variables.add_entry(name, handle);
            }
        }
        true
    }

    /// Default hatch origin in drawing coordinates. Missing/malformed entries use zero.
    pub fn hatch_origin(&self) -> [f64; 2] {
        let read = || {
            let mut parts = self.drawing_variable("HPORIGIN")?.split(';');
            let x = parts.next()?.trim().parse::<f64>().ok()?;
            let y = parts.next()?.trim().parse::<f64>().ok()?;
            (parts.next().is_none() && x.is_finite() && y.is_finite()).then_some([x, y])
        };
        read().unwrap_or([0.0, 0.0])
    }

    /// Store a finite hatch origin using the standard semicolon-separated point value.
    pub fn set_hatch_origin(&mut self, origin: [f64; 2]) -> bool {
        if !origin.iter().all(|value| value.is_finite()) { return false; }
        self.set_drawing_variable("HPORIGIN", &format!("{};{}", origin[0], origin[1]))
    }
}
