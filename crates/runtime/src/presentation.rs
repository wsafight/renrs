use crate::syntax::{LayeredImage, Value};
use crate::{Runtime, RuntimeError};
use std::collections::BTreeMap;

impl Runtime {
    pub(crate) fn resolve_image(
        &self,
        path: &str,
        line: usize,
    ) -> Result<Option<LayeredImage>, RuntimeError> {
        let Some(image) = self.program.layered_images.get(path) else {
            return Ok(None);
        };
        let mut selected = vec![false; image.layers.len()];
        let mut grouped = BTreeMap::new();
        for (index, layer) in image.layers.iter().enumerate() {
            if let Some(condition) = &layer.condition {
                match self.evaluate_expression(condition)? {
                    Value::Boolean(true) => {}
                    Value::Boolean(false) => continue,
                    _ => {
                        return Err(RuntimeError::Execution {
                            line,
                            message: "image layer condition must be boolean".to_owned(),
                        });
                    }
                }
            }
            if let Some(group) = &layer.group {
                if let Some(previous) = grouped.insert(group, index) {
                    selected[previous] = false;
                }
            }
            selected[index] = true;
        }
        let layers = image
            .layers
            .iter()
            .enumerate()
            .filter(|(index, _)| selected[*index])
            .map(|(_, layer)| layer.source_layer())
            .collect();
        Ok(Some(image.resolved(layers)))
    }
}
