#!/usr/bin/env python3
"""Convert a PyTorch Network.pt model to ONNX format for Rust inference.

Usage:
    python convert_pt_to_onnx.py <path_to_Network.pt> [output_dir]

Example:
    python convert_pt_to_onnx.py ../../ai4animationpy/Demos/Locomotion/Biped/Network.pt ./models/

This script:
  1. Loads the PyTorch model (CodebookMatching architecture)
  2. Exports it to ONNX format with fixed iteration count
  3. Saves model metadata (dimensions, normalization stats) as .npz

Requirements:
  - Python 3.8+
  - PyTorch (torch)
  - numpy
  - The ai4animation Python package must be importable
    (add ai4animationpy/ to PYTHONPATH or run from the repo root)
"""

import sys
import os
import argparse
import numpy as np

# Ensure ai4animation package is importable
SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
REPO_ROOT = os.path.dirname(SCRIPT_DIR)
PY_REPO = os.path.join(os.path.dirname(REPO_ROOT), "ai4animationpy")
if os.path.isdir(PY_REPO):
    sys.path.insert(0, PY_REPO)


def _to_numpy(val):
    """Convert a tensor or ndarray to a flat numpy array."""
    if hasattr(val, 'detach'):
        return val.detach().cpu().numpy().flatten()
    return np.asarray(val).flatten()


def find_stats(model):
    """Extract InputStats/OutputStats normalization parameters from the model."""
    stats = {}

    # Try common attribute names used in ai4animation networks
    for attr_name in ["InputStats", "inputstats", "input_stats"]:
        obj = getattr(model, attr_name, None)
        if obj is not None:
            mean = getattr(obj, "Mean", getattr(obj, "mean", None))
            std = getattr(obj, "Std", getattr(obj, "std", None))
            if mean is not None:
                stats["input_mean"] = _to_numpy(mean)
                if std is not None:
                    stats["input_std"] = _to_numpy(std)
            break

    for attr_name in ["OutputStats", "outputstats", "output_stats"]:
        obj = getattr(model, attr_name, None)
        if obj is not None:
            mean = getattr(obj, "Mean", getattr(obj, "mean", None))
            std = getattr(obj, "Std", getattr(obj, "std", None))
            if mean is not None:
                stats["output_mean"] = _to_numpy(mean)
                if std is not None:
                    stats["output_std"] = _to_numpy(std)
            break

    return stats


class OnnxExportWrapper(object):
    """Wraps the model for clean ONNX export with fixed iteration count."""

    def __init__(self, model, iterations=3):
        import torch.nn as nn
        # We use object.__init__ to avoid nn.Module registration issues
        # with the loaded model's custom classes
        self.model = model
        self.iterations = iterations

    def __call__(self, x, noise, seed):
        outputs, z, p, _ = self.model(
            x,
            noise=noise,
            iterations=self.iterations,
            seed=seed,
        )
        return outputs


def main():
    parser = argparse.ArgumentParser(description="Convert Network.pt to ONNX")
    parser.add_argument("model_path", help="Path to Network.pt")
    parser.add_argument(
        "output_dir",
        nargs="?",
        default=".",
        help="Output directory (default: current dir)",
    )
    parser.add_argument(
        "--iterations",
        type=int,
        default=3,
        help="Number of denoiser iterations (default: 3)",
    )
    parser.add_argument(
        "--opset",
        type=int,
        default=17,
        help="ONNX opset version (default: 17)",
    )
    args = parser.parse_args()

    import torch

    print(f"Loading model from: {args.model_path}")
    model = torch.load(args.model_path, weights_only=False, map_location="cpu")
    model.eval()

    # Extract model dimensions
    input_dim = getattr(model, "InputDim", None)
    output_dim = getattr(model, "OutputDim", None)
    latent_dim = getattr(model, "LatentDim", None)
    seq_length = getattr(model, "SequenceLength", 16)
    seq_window = getattr(model, "SequenceWindow", 0.5)

    if input_dim is None:
        print("ERROR: Model does not have InputDim attribute.")
        print("Available attributes:", [a for a in dir(model) if not a.startswith("_")])
        sys.exit(1)

    if latent_dim is None:
        # Try to infer from codebook dimensions
        codebook_channels = getattr(model, "codebook_channels", None)
        codebook_dims = getattr(model, "codebook_dims", None)
        if codebook_channels and codebook_dims:
            latent_dim = codebook_channels * codebook_dims
        else:
            print("WARNING: Could not determine LatentDim, defaulting to 64")
            latent_dim = 64

    print(f"Model dimensions:")
    print(f"  InputDim:       {input_dim}")
    print(f"  OutputDim:      {output_dim}")
    print(f"  LatentDim:      {latent_dim}")
    print(f"  SequenceLength: {seq_length}")
    print(f"  SequenceWindow: {seq_window}")

    os.makedirs(args.output_dir, exist_ok=True)

    # ── Export to ONNX ──────────────────────────────────────
    print(f"\nExporting to ONNX (opset {args.opset}, {args.iterations} iterations)...")

    # Create wrapper for clean export
    # We use torch.nn.Module subclass for proper tracing
    class ExportModule(torch.nn.Module):
        def __init__(self, mdl, iters):
            super().__init__()
            self.mdl = mdl
            self.iters = iters

        def forward(self, x, noise, seed):
            out, _, _, _ = self.mdl(x, noise=noise, iterations=self.iters, seed=seed)
            return out

    wrapper = ExportModule(model, args.iterations)
    wrapper.eval()

    # Create dummy inputs
    dummy_input = torch.randn(1, input_dim)
    dummy_noise = torch.randn(1, latent_dim)
    dummy_seed = torch.zeros(1, latent_dim)

    onnx_path = os.path.join(args.output_dir, "Network.onnx")

    try:
        torch.onnx.export(
            wrapper,
            (dummy_input, dummy_noise, dummy_seed),
            onnx_path,
            input_names=["input", "noise", "seed"],
            output_names=["output"],
            dynamic_axes={
                "input": {0: "batch"},
                "noise": {0: "batch"},
                "seed": {0: "batch"},
                "output": {0: "batch"},
            },
            opset_version=args.opset,
            do_constant_folding=True,
        )
        print(f"  Saved ONNX model: {onnx_path}")
    except Exception as e:
        print(f"  ONNX export failed: {e}")
        print("  Trying with torch.jit.trace fallback...")

        # Fallback: trace then export
        try:
            traced = torch.jit.trace(wrapper, (dummy_input, dummy_noise, dummy_seed))
            torch.onnx.export(
                traced,
                (dummy_input, dummy_noise, dummy_seed),
                onnx_path,
                input_names=["input", "noise", "seed"],
                output_names=["output"],
                opset_version=args.opset,
            )
            print(f"  Saved ONNX model (traced): {onnx_path}")
        except Exception as e2:
            print(f"  Trace fallback also failed: {e2}")
            print("  You may need to simplify the model or use a lower opset version.")
            # Still save metadata for manual conversion
            print("  Saving metadata only...")

    # ── Save metadata + normalization stats as JSON ──────────
    import json

    metadata = {
        "input_dim": int(input_dim),
        "output_dim": int(output_dim) if output_dim else 0,
        "latent_dim": int(latent_dim),
        "sequence_length": int(seq_length),
        "sequence_window": float(seq_window),
        "iterations": int(args.iterations),
    }

    # Also save bone count info if available
    num_bones = getattr(model, "NumBones", None)
    if num_bones:
        metadata["num_bones"] = int(num_bones)

    # Extract normalization stats
    stats = find_stats(model)
    for key, val in stats.items():
        metadata[key] = val.tolist()

    json_path = os.path.join(args.output_dir, "Network_meta.json")
    with open(json_path, "w") as f:
        json.dump(metadata, f, indent=2)
    print(f"  Saved metadata: {json_path}")

    # ── Verify ONNX model ──────────────────────────────────
    if os.path.exists(onnx_path):
        try:
            import onnxruntime as ort

            print("\nVerifying ONNX model with onnxruntime...")
            sess = ort.InferenceSession(onnx_path, providers=["CPUExecutionProvider"])

            print("  Inputs:")
            for inp in sess.get_inputs():
                print(f"    {inp.name}: {inp.shape} ({inp.type})")

            print("  Outputs:")
            for out in sess.get_outputs():
                print(f"    {out.name}: {out.shape} ({out.type})")

            # Test inference
            test_input = np.random.randn(1, input_dim).astype(np.float32)
            test_noise = np.random.randn(1, latent_dim).astype(np.float32) * 0.5
            test_seed = np.zeros((1, latent_dim), dtype=np.float32)

            result = sess.run(
                None,
                {"input": test_input, "noise": test_noise, "seed": test_seed},
            )
            print(f"  Output shape: {result[0].shape}")
            print(f"  Output range: [{result[0].min():.4f}, {result[0].max():.4f}]")
            print("  Verification PASSED")

        except ImportError:
            print("\n  onnxruntime not installed — skipping verification.")
            print("  Install with: pip install onnxruntime")
        except Exception as e:
            print(f"\n  Verification FAILED: {e}")

    print("\nDone!")
    print(f"Files in {args.output_dir}:")
    for fname in os.listdir(args.output_dir):
        if fname.startswith("Network"):
            size = os.path.getsize(os.path.join(args.output_dir, fname))
            print(f"  {fname} ({size / 1024:.1f} KB)")


if __name__ == "__main__":
    main()
