//! ONNX model inference — load and run .onnx models for AI-driven animation.
//!
//! Uses the `ort` crate (ONNX Runtime bindings for Rust).
//! Models are exported from PyTorch using `tools/convert_pt_to_onnx.py`.

use std::path::Path;
use std::collections::HashMap;
use anyhow::{Result, Context};
use ort::session::Session;
use ort::session::builder::GraphOptimizationLevel;
use ort::value::{Tensor, ValueType, Outlet};

/// A loaded ONNX model ready for inference.
pub struct OnnxModel {
    /// ONNX Runtime session.
    session: Session,
    /// Model file path (for display/logging).
    pub path: String,
    /// Input names and their expected shapes (None = dynamic).
    pub input_info: Vec<TensorInfo>,
    /// Output names and shapes.
    pub output_info: Vec<TensorInfo>,
}

/// Metadata about a model tensor (input or output).
#[derive(Debug, Clone)]
pub struct TensorInfo {
    pub name: String,
    /// Shape dimensions (None = dynamic axis).
    pub shape: Vec<Option<usize>>,
}

impl OnnxModel {
    /// Load an ONNX model from a file path.
    pub fn load(path: &Path) -> Result<Self> {
        log::info!("Chargement du modèle ONNX: {}", path.display());

        let session = Session::builder()
            .map_err(|e| anyhow::anyhow!("Impossible de créer le builder ONNX Runtime: {e}"))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| anyhow::anyhow!("Impossible de configurer l'optimisation: {e}"))?
            .commit_from_file(path)
            .map_err(|e| anyhow::anyhow!("Impossible de charger le modèle {}: {e}", path.display()))?;

        let input_info: Vec<TensorInfo> = session.inputs().iter().map(|outlet: &Outlet| {
            let shape = extract_shape(outlet.dtype());
            TensorInfo {
                name: outlet.name().to_string(),
                shape,
            }
        }).collect();

        let output_info: Vec<TensorInfo> = session.outputs().iter().map(|outlet: &Outlet| {
            let shape = extract_shape(outlet.dtype());
            TensorInfo {
                name: outlet.name().to_string(),
                shape,
            }
        }).collect();

        log::info!(
            "Modèle ONNX chargé: {} entrées, {} sorties",
            input_info.len(),
            output_info.len()
        );
        for info in &input_info {
            log::info!("  Entrée '{}': {:?}", info.name, info.shape);
        }
        for info in &output_info {
            log::info!("  Sortie '{}': {:?}", info.name, info.shape);
        }

        Ok(Self {
            session,
            path: path.display().to_string(),
            input_info,
            output_info,
        })
    }

    /// Run inference with named inputs.
    ///
    /// Input: HashMap of name -> (flat f32 data, shape).
    /// Output: HashMap of name -> flat f32 data.
    pub fn run(
        &mut self,
        inputs: &HashMap<String, (Vec<f32>, Vec<usize>)>,
    ) -> Result<HashMap<String, Vec<f32>>> {
        // Build named input values
        let mut ort_inputs: Vec<(
            std::borrow::Cow<'_, str>,
            ort::session::SessionInputValue<'_>,
        )> = Vec::new();

        for info in &self.input_info {
            let (data, shape) = inputs.get(&info.name)
                .ok_or_else(|| anyhow::anyhow!(
                    "Entrée '{}' manquante. Entrées fournies: {:?}",
                    info.name,
                    inputs.keys().collect::<Vec<_>>()
                ))?;

            let shape_i64: Vec<i64> = shape.iter().map(|&d| d as i64).collect();

            let tensor = Tensor::from_array((shape_i64, data.clone().into_boxed_slice()))
                .map_err(|e| anyhow::anyhow!(
                    "Impossible de créer le tensor pour '{}' (shape {:?}, {} éléments): {e}",
                    info.name, shape, data.len()
                ))?;

            ort_inputs.push((
                std::borrow::Cow::Owned(info.name.clone()),
                ort::session::SessionInputValue::from(tensor),
            ));
        }

        // Collect output names before running (avoid borrow conflict)
        let output_names: Vec<String> = self.output_info.iter()
            .map(|info| info.name.clone())
            .collect();

        let outputs = self.session.run(ort_inputs)
            .map_err(|e| anyhow::anyhow!("Erreur lors de l'inférence ONNX: {e}"))?;

        // Extract outputs
        let mut result = HashMap::new();
        for (i, name) in output_names.iter().enumerate() {
            if i < outputs.len() {
                let (_shape, data) = outputs[i]
                    .try_extract_tensor::<f32>()
                    .map_err(|e| anyhow::anyhow!("Impossible d'extraire la sortie '{}': {e}", name))?;
                result.insert(name.clone(), data.to_vec());
            }
        }

        Ok(result)
    }

    /// Convenience: single-input, single-output inference.
    pub fn predict(&mut self, input: &[f32], input_shape: &[usize]) -> Result<Vec<f32>> {
        let input_name = self.input_info.first()
            .ok_or_else(|| anyhow::anyhow!("Modèle sans entrée définie"))?
            .name.clone();
        let output_name = self.output_info.first()
            .ok_or_else(|| anyhow::anyhow!("Modèle sans sortie définie"))?
            .name.clone();

        let mut inputs = HashMap::new();
        inputs.insert(input_name, (input.to_vec(), input_shape.to_vec()));

        let outputs = self.run(&inputs)?;
        outputs.get(&output_name)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Sortie '{}' manquante", output_name))
    }

    /// Run the locomotion model specifically.
    ///
    /// Takes the flat input vector, noise, and seed; returns the flat output.
    /// This matches the Network.onnx exported by convert_pt_to_onnx.py.
    pub fn run_locomotion(
        &mut self,
        input: &[f32],
        input_dim: usize,
        noise: &[f32],
        latent_dim: usize,
        seed: &[f32],
    ) -> Result<Vec<f32>> {
        let mut inputs = HashMap::new();
        inputs.insert("input".to_string(), (input.to_vec(), vec![1, input_dim]));
        inputs.insert("noise".to_string(), (noise.to_vec(), vec![1, latent_dim]));
        inputs.insert("seed".to_string(), (seed.to_vec(), vec![1, latent_dim]));

        let outputs = self.run(&inputs)?;
        outputs.get("output")
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Sortie 'output' manquante du modèle locomotion"))
    }

    /// Get input dimension (first input, last axis).
    pub fn input_dim(&self) -> Option<usize> {
        self.input_info.first()
            .and_then(|info| info.shape.last().copied().flatten())
    }

    /// Get the number of inputs.
    pub fn num_inputs(&self) -> usize {
        self.input_info.len()
    }

    /// Get the number of outputs.
    pub fn num_outputs(&self) -> usize {
        self.output_info.len()
    }
}

/// Model metadata loaded from the companion _meta.json file.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ModelMetadata {
    pub input_dim: usize,
    pub output_dim: usize,
    pub latent_dim: usize,
    pub sequence_length: usize,
    pub sequence_window: f32,
    pub iterations: usize,
    #[serde(default)]
    pub num_bones: Option<usize>,
    #[serde(default)]
    pub input_mean: Option<Vec<f32>>,
    #[serde(default)]
    pub input_std: Option<Vec<f32>>,
    #[serde(default)]
    pub output_mean: Option<Vec<f32>>,
    #[serde(default)]
    pub output_std: Option<Vec<f32>>,
}

impl ModelMetadata {
    /// Load metadata from a Network_meta.json file.
    pub fn load(path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("Impossible de lire les métadonnées: {}", path.display()))?;
        let meta: ModelMetadata = serde_json::from_str(&contents)
            .with_context(|| "Format JSON invalide pour les métadonnées")?;
        Ok(meta)
    }
}

/// Cache of loaded ONNX models.
pub struct OnnxCache {
    models: HashMap<String, OnnxModel>,
    max_size: usize,
}

impl OnnxCache {
    pub fn new(max_size: usize) -> Self {
        Self {
            models: HashMap::new(),
            max_size,
        }
    }

    /// Get or load a model.
    pub fn get_or_load(&mut self, path: &Path) -> Result<&mut OnnxModel> {
        let key = path.display().to_string();
        if !self.models.contains_key(&key) {
            if self.models.len() >= self.max_size {
                if let Some(first_key) = self.models.keys().next().cloned() {
                    self.models.remove(&first_key);
                }
            }
            let model = OnnxModel::load(path)?;
            self.models.insert(key.clone(), model);
        }
        Ok(self.models.get_mut(&key).unwrap())
    }

    pub fn clear(&mut self) {
        self.models.clear();
    }

    pub fn len(&self) -> usize {
        self.models.len()
    }

    pub fn is_empty(&self) -> bool {
        self.models.is_empty()
    }
}

// ── Helpers ──────────────────────────────────────────────────

/// Extract shape information from an ort ValueType.
fn extract_shape(value_type: &ValueType) -> Vec<Option<usize>> {
    match value_type {
        ValueType::Tensor { shape, .. } => {
            shape.iter().map(|&d| {
                if d < 0 { None } else { Some(d as usize) }
            }).collect()
        }
        _ => Vec::new(),
    }
}
