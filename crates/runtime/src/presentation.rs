use crate::syntax::{LayeredImage, Value};
use crate::{Runtime, RuntimeError};

impl Runtime {
    pub(crate) fn resolve_image(
        &self,
        path: &str,
        line: usize,
    ) -> Result<Option<LayeredImage>, RuntimeError> {
        let Some(image) = self.program.layered_images.get(path) else {
            return Ok(None);
        };
        let mut layers = Vec::new();
        for layer in &image.layers {
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
            layers.push(layer.source_layer());
        }
        Ok(Some(image.resolved(layers)))
    }
}
