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
        let mut image = image.clone();
        let mut layers = Vec::new();
        for layer in image.layers {
            if let Some(condition) = &layer.when {
                let expression =
                    renrs_compiler::expression::parse_expression(condition, path, line, 1)
                        .map_err(|error| RuntimeError::Execution {
                            line,
                            message: error.to_string(),
                        })?;
                match self.evaluate_expression(&expression)? {
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
            layers.push(layer);
        }
        image.layers = layers;
        Ok(Some(image))
    }
}
