#!/usr/bin/env python3
"""Train a CodebookMatching locomotion model from BVH/FBX motion data.

Usage:
    python train_locomotion.py --data-dir <path_to_bvh_files> --output-dir <output>

This script:
  1. Loads motion capture files (BVH/FBX) from the data directory
  2. Processes them into input/output training pairs
  3. Trains a CodebookMatching model
  4. Exports the trained model as .pt and converts to .onnx + _meta.json

Requirements:
  - Python 3.8+
  - PyTorch
  - The ai4animation Python package (ai4animationpy/ must be on PYTHONPATH)
"""

import sys
import os
import argparse
import json
import time
import numpy as np
from pathlib import Path

# Ensure ai4animation package is importable
SCRIPT_DIR = Path(__file__).parent
REPO_ROOT = SCRIPT_DIR.parent
PY_REPO = REPO_ROOT.parent / "ai4animationpy"
if PY_REPO.is_dir():
    sys.path.insert(0, str(PY_REPO))

import torch
import torch.nn as nn


def log(msg):
    """Print with flush for real-time output to Rust parent process."""
    print(msg, flush=True)


def find_motion_files(data_dir):
    """Find all supported motion files in a directory."""
    extensions = {'.bvh', '.fbx', '.glb', '.gltf', '.npz'}
    files = []
    for root, _dirs, filenames in os.walk(data_dir):
        for f in filenames:
            if Path(f).suffix.lower() in extensions:
                files.append(os.path.join(root, f))
    return sorted(files)


def load_bvh_as_frames(filepath, scale=1.0):
    """Load a BVH file and extract per-frame joint transforms."""
    try:
        from ai4animation.Import.BVHImporter import BVHImporter
        bvh = BVHImporter()
        bvh.Load(filepath)
        frames = []
        joint_names = [j.Name for j in bvh.Skeleton.Joints]
        num_joints = len(joint_names)

        for f in range(bvh.GetFrameCount()):
            bvh.SetFrame(f)
            frame_data = []
            for j in range(num_joints):
                pos = bvh.Skeleton.Joints[j].GetWorldPosition() * scale
                rot = bvh.Skeleton.Joints[j].GetWorldRotation()
                frame_data.append({
                    'position': np.array([pos.x, pos.y, pos.z], dtype=np.float32),
                    'rotation': np.array([rot.x, rot.y, rot.z, rot.w], dtype=np.float32),
                })
            frames.append(frame_data)
        return frames, joint_names, bvh.GetFrameRate()
    except Exception:
        # Fallback: use simple BVH parser
        return load_bvh_simple(filepath, scale)


def load_bvh_simple(filepath, scale=1.0):
    """Simple BVH loader that extracts raw channel data as frames."""
    with open(filepath, 'r') as f:
        content = f.read()

    # Parse basic structure
    lines = content.split('\n')
    joint_names = []
    in_hierarchy = True
    channels_per_joint = []
    frame_time = 1.0 / 30.0
    frames_data = []

    i = 0
    while i < len(lines):
        line = lines[i].strip()
        if 'JOINT' in line or 'ROOT' in line:
            parts = line.split()
            if len(parts) >= 2:
                joint_names.append(parts[-1])
        elif line.startswith('CHANNELS'):
            parts = line.split()
            channels_per_joint.append(int(parts[1]))
        elif line == 'MOTION':
            in_hierarchy = False
        elif line.startswith('Frames:'):
            num_frames = int(line.split(':')[1].strip())
        elif line.startswith('Frame Time:'):
            frame_time = float(line.split(':')[1].strip())
        elif not in_hierarchy and line and not line.startswith('Frame'):
            values = [float(v) for v in line.split()]
            frames_data.append(values)
        i += 1

    if not joint_names:
        joint_names = [f"Joint_{i}" for i in range(len(channels_per_joint))]

    fps = 1.0 / frame_time if frame_time > 0 else 30.0

    # Convert raw channel data to position arrays
    frames = []
    total_channels = sum(channels_per_joint)
    for frame_values in frames_data:
        if len(frame_values) < total_channels:
            continue
        frame = []
        offset = 0
        for j, nch in enumerate(channels_per_joint):
            vals = frame_values[offset:offset + nch]
            pos = np.array(vals[:3], dtype=np.float32) * scale if nch >= 3 else np.zeros(3, dtype=np.float32)
            rot = np.array([0, 0, 0, 1], dtype=np.float32)  # identity quat
            if nch >= 6:
                # Euler angles to simple rotation (approximate)
                euler = np.radians(vals[3:6])
                rot = euler_to_quat(euler)
            frame.append({'position': pos, 'rotation': rot})
            offset += nch
        frames.append(frame)

    return frames, joint_names, fps


def euler_to_quat(euler):
    """Convert ZYX Euler angles (radians) to quaternion [x,y,z,w]."""
    cx, cy, cz = np.cos(euler / 2)
    sx, sy, sz = np.sin(euler / 2)
    w = cx * cy * cz + sx * sy * sz
    x = sx * cy * cz - cx * sy * sz
    y = cx * sy * cz + sx * cy * sz
    z = cx * cy * sz - sx * sy * cz
    return np.array([x, y, z, w], dtype=np.float32)


def build_training_pairs(all_frames, all_fps, sequence_length=16, sequence_window=0.5):
    """Build input/output training pairs from motion sequences.

    Input (per sample): current pose state + future trajectory control
    Output (per sample): future sequence of poses [sequence_length, output_dim]
    """
    inputs = []
    outputs = []

    for clip_frames, fps in zip(all_frames, all_fps):
        num_frames = len(clip_frames)
        if num_frames < sequence_length + 1:
            continue

        num_joints = len(clip_frames[0])
        frame_step = max(1, int((sequence_window * fps) / sequence_length))

        for start in range(0, num_frames - sequence_length * frame_step, max(1, frame_step)):
            # Current frame
            current = clip_frames[start]

            # Root transform (first joint)
            root_pos = current[0]['position']

            # Build input: positions + velocities relative to root
            inp = []
            for j in range(num_joints):
                rel_pos = current[j]['position'] - root_pos
                inp.extend(rel_pos.tolist())

            # Z-axis and Y-axis (from rotation) for each joint
            for j in range(num_joints):
                q = current[j]['rotation']
                z_axis = quat_rotate(q, [0, 0, 1])
                inp.extend(z_axis)
            for j in range(num_joints):
                q = current[j]['rotation']
                y_axis = quat_rotate(q, [0, 1, 0])
                inp.extend(y_axis)

            # Velocities (from next frame delta)
            next_frame = clip_frames[min(start + 1, num_frames - 1)]
            for j in range(num_joints):
                vel = (next_frame[j]['position'] - current[j]['position']) * fps
                inp.extend(vel.tolist())

            # Future root trajectory (XZ) [sequence_length, 2] × 3 (pos, dir, vel)
            for si in range(sequence_length):
                fi = min(start + si * frame_step, num_frames - 1)
                future_pos = clip_frames[fi][0]['position'] - root_pos
                inp.extend([future_pos[0], future_pos[2]])  # XZ only
            for si in range(sequence_length):
                fi = min(start + si * frame_step, num_frames - 1)
                q = clip_frames[fi][0]['rotation']
                fwd = quat_rotate(q, [0, 0, 1])
                inp.extend([fwd[0], fwd[2]])  # XZ only
            for si in range(sequence_length):
                fi = min(start + si * frame_step, num_frames - 1)
                fi_next = min(fi + 1, num_frames - 1)
                vel = (clip_frames[fi_next][0]['position'] - clip_frames[fi][0]['position']) * fps
                inp.extend([vel[0], vel[2]])  # XZ only

            # Guidance (zeros for now — no style guidance during training)
            for j in range(num_joints):
                inp.extend([0.0, 0.0, 0.0])

            # Build output: sequence of future frames
            out_seq = []
            for si in range(sequence_length):
                fi = min(start + si * frame_step, num_frames - 1)
                frame = clip_frames[fi]
                out_frame = []

                # Root velocity
                fi_next = min(fi + 1, num_frames - 1)
                root_vel = (clip_frames[fi_next][0]['position'] - frame[0]['position']) * fps
                out_frame.extend(root_vel.tolist())

                # Bone positions relative to root
                frame_root = frame[0]['position']
                for j in range(num_joints):
                    rel = frame[j]['position'] - frame_root
                    out_frame.extend(rel.tolist())

                # Bone rotations (z_vec + y_vec = 6 values per bone)
                for j in range(num_joints):
                    q = frame[j]['rotation']
                    z = quat_rotate(q, [0, 0, 1])
                    y = quat_rotate(q, [0, 1, 0])
                    out_frame.extend(z)
                    out_frame.extend(y)

                # Bone velocities
                for j in range(num_joints):
                    vel = (clip_frames[min(fi + 1, num_frames - 1)][j]['position'] - frame[j]['position']) * fps
                    out_frame.extend(vel.tolist())

                # Contacts (4 values — placeholder zeros)
                out_frame.extend([0.0, 0.0, 0.0, 0.0])

                # Guidance (zeros)
                for j in range(num_joints):
                    out_frame.extend([0.0, 0.0, 0.0])

                out_seq.append(out_frame)

            inputs.append(inp)
            outputs.append(out_seq)

    return np.array(inputs, dtype=np.float32), np.array(outputs, dtype=np.float32)


def quat_rotate(q, v):
    """Rotate vector v by quaternion q=[x,y,z,w]."""
    qx, qy, qz, qw = q
    vx, vy, vz = v
    # q * v * q^-1
    tx = 2.0 * (qy * vz - qz * vy)
    ty = 2.0 * (qz * vx - qx * vz)
    tz = 2.0 * (qx * vy - qy * vx)
    return [
        vx + qw * tx + qy * tz - qz * ty,
        vy + qw * ty + qz * tx - qx * tz,
        vz + qw * tz + qx * ty - qy * tx,
    ]


def create_model(input_dim, output_dim, sequence_length, sequence_window):
    """Create a CodebookMatching model."""
    try:
        from ai4animation.AI.Networks.CodebookMatching import Model
        model = Model(
            input_dim=input_dim,
            output_dim=output_dim,
            sequence_length=sequence_length,
            sequence_window=sequence_window,
            encoder_dim=512,
            estimator_dim=512,
            codebook_channels=128,
            codebook_dims=8,
            decoder_dim=512,
            dropout=0.1,
            hard=False,
            plotting=0,  # no GUI plotting
        )
        return model
    except ImportError:
        log("WARNING: ai4animation package not found, using simplified model")
        return create_simple_model(input_dim, output_dim, sequence_length, sequence_window)


def create_simple_model(input_dim, output_dim, sequence_length, sequence_window):
    """Simplified MLP model when CodebookMatching is not available."""

    class SimpleLocomotionModel(nn.Module):
        def __init__(self, input_dim, output_dim, seq_len, seq_window):
            super().__init__()
            self.InputDim = input_dim
            self.OutputDim = output_dim
            self.SequenceLength = seq_len
            self.SequenceWindow = seq_window
            self.LatentDim = 128
            hidden = 512

            # Input normalization stats
            self.InputStats = RunningStats(input_dim)
            self.OutputStats = RunningStats(output_dim)

            # Encoder: input -> latent
            self.encoder = nn.Sequential(
                nn.Linear(input_dim, hidden),
                nn.ELU(),
                nn.Dropout(0.1),
                nn.Linear(hidden, hidden),
                nn.ELU(),
                nn.Dropout(0.1),
                nn.Linear(hidden, self.LatentDim),
            )

            # Decoder: latent + input -> output sequence
            self.decoder = nn.Sequential(
                nn.Linear(self.LatentDim + input_dim, hidden),
                nn.ELU(),
                nn.Dropout(0.1),
                nn.Linear(hidden, hidden),
                nn.ELU(),
                nn.Dropout(0.1),
                nn.Linear(hidden, seq_len * output_dim),
            )

        def forward(self, x, noise=None, iterations=0, seed=None):
            x_norm = self.InputStats.Normalize(x)
            z = self.encoder(x_norm)
            if noise is not None:
                z = z + noise[:, :self.LatentDim] * 0.1
            combined = torch.cat([z, x_norm], dim=-1)
            y_flat = self.decoder(combined)
            y = y_flat.reshape(-1, self.SequenceLength, self.OutputDim)
            y = self.OutputStats.Denormalize(y)
            return y, z, z, None

        def learn(self, x, y, update_stats):
            x_norm = self.InputStats.UpdateAndNormalize(x) if update_stats else self.InputStats.Normalize(x)
            y_norm = self.OutputStats.UpdateAndNormalize(y) if update_stats else self.OutputStats.Normalize(y)
            z = self.encoder(x_norm)
            combined = torch.cat([z, x_norm], dim=-1)
            y_pred = self.decoder(combined).reshape(-1, self.SequenceLength, self.OutputDim)
            loss = nn.MSELoss()(y_pred, y_norm)
            return {"Y": self.OutputStats.Denormalize(y_pred)}, {"Reconstruction Loss": loss}

    return SimpleLocomotionModel(input_dim, output_dim, sequence_length, sequence_window)


class RunningStats:
    """Running mean/std tracker compatible with export."""
    def __init__(self, dim):
        self.dim = dim
        self.Mean = np.zeros(dim, dtype=np.float32)
        self.Std = np.ones(dim, dtype=np.float32)
        self._sum = np.zeros(dim, dtype=np.float64)
        self._sq_sum = np.zeros(dim, dtype=np.float64)
        self._count = 0

    def Update(self, x):
        """Update stats with a batch of data [batch, dim] or [batch, seq, dim]."""
        if isinstance(x, torch.Tensor):
            x = x.detach().cpu().numpy()
        x = x.reshape(-1, self.dim)
        self._sum += x.sum(axis=0)
        self._sq_sum += (x ** 2).sum(axis=0)
        self._count += x.shape[0]
        if self._count > 1:
            self.Mean = (self._sum / self._count).astype(np.float32)
            variance = (self._sq_sum / self._count - self.Mean ** 2).clip(min=1e-8)
            self.Std = np.sqrt(variance).astype(np.float32)

    def Normalize(self, x):
        mean = torch.tensor(self.Mean, device=x.device, dtype=x.dtype)
        std = torch.tensor(self.Std, device=x.device, dtype=x.dtype)
        return (x - mean) / std.clamp(min=1e-8)

    def UpdateAndNormalize(self, x):
        self.Update(x)
        return self.Normalize(x)

    def Denormalize(self, x):
        mean = torch.tensor(self.Mean, device=x.device, dtype=x.dtype)
        std = torch.tensor(self.Std, device=x.device, dtype=x.dtype)
        return x * std + mean


def train(model, X_train, Y_train, epochs, batch_size, lr, output_dir, device):
    """Train the model and save checkpoints."""
    model = model.to(device)
    X_train = torch.tensor(X_train, dtype=torch.float32, device=device)
    Y_train = torch.tensor(Y_train, dtype=torch.float32, device=device)

    num_samples = X_train.shape[0]
    log(f"Training samples: {num_samples}, input_dim: {X_train.shape[1]}, output: {Y_train.shape[1:]}")

    # Use separate optimizers if CodebookMatching, single optimizer otherwise
    if hasattr(model, 'Encoder') and hasattr(model, 'Estimator'):
        # CodebookMatching style
        opt_prior = torch.optim.AdamW(
            list(model.Encoder.parameters()) + list(model.Decoder.parameters()),
            lr=lr
        )
        opt_matcher = torch.optim.AdamW(model.Estimator.parameters(), lr=lr)
        opt_denoiser = torch.optim.AdamW(model.Denoiser.parameters(), lr=lr)
        is_codebook = True
    else:
        optimizer = torch.optim.AdamW(model.parameters(), lr=lr)
        is_codebook = False

    best_loss = float('inf')
    start_time = time.time()

    for epoch in range(epochs):
        model.train()
        indices = torch.randperm(num_samples, device=device)
        epoch_loss = 0.0
        num_batches = 0

        for b in range(0, num_samples, batch_size):
            batch_idx = indices[b:b + batch_size]
            x_batch = X_train[batch_idx]
            y_batch = Y_train[batch_idx]

            update_stats = epoch < 5  # Only update running stats in early epochs

            _, losses = model.learn(x_batch, y_batch, update_stats)

            if is_codebook:
                opt_prior.zero_grad()
                losses["Reconstruction Loss"].backward()
                torch.nn.utils.clip_grad_norm_(model.parameters(), 1.0)
                opt_prior.step()

                opt_matcher.zero_grad()
                losses["Matching Loss"].backward()
                opt_matcher.step()

                opt_denoiser.zero_grad()
                losses["Denoising Loss"].backward()
                opt_denoiser.step()

                total = sum(v.item() for v in losses.values())
            else:
                optimizer.zero_grad()
                losses["Reconstruction Loss"].backward()
                torch.nn.utils.clip_grad_norm_(model.parameters(), 1.0)
                optimizer.step()
                total = losses["Reconstruction Loss"].item()

            epoch_loss += total
            num_batches += 1

        avg_loss = epoch_loss / max(num_batches, 1)
        elapsed = time.time() - start_time
        eta = (elapsed / (epoch + 1)) * (epochs - epoch - 1)

        loss_parts = " | ".join(f"{k}: {v.item():.6f}" for k, v in losses.items())
        log(f"Epoch {epoch+1}/{epochs} | Loss: {avg_loss:.6f} | {loss_parts} | ETA: {eta:.0f}s")

        # Save best model
        if avg_loss < best_loss:
            best_loss = avg_loss
            save_path = os.path.join(output_dir, "Network.pt")
            torch.save(model, save_path)

        # Periodic checkpoint
        if (epoch + 1) % 10 == 0:
            ckpt_path = os.path.join(output_dir, f"checkpoint_epoch{epoch+1}.pt")
            torch.save(model, ckpt_path)
            log(f"Checkpoint saved: {ckpt_path}")

    log(f"Training complete! Best loss: {best_loss:.6f}")
    return model


def export_to_onnx(model, output_dir):
    """Export trained model to ONNX + metadata JSON."""
    log("Exporting to ONNX...")

    # Use the conversion script
    convert_script = SCRIPT_DIR / "convert_pt_to_onnx.py"
    model_path = os.path.join(output_dir, "Network.pt")

    if convert_script.exists():
        import subprocess
        result = subprocess.run(
            [sys.executable, str(convert_script), model_path, output_dir],
            capture_output=True, text=True
        )
        for line in result.stdout.splitlines():
            log(f"  {line}")
        if result.returncode != 0:
            log(f"  ONNX export error: {result.stderr}")
    else:
        log(f"  Conversion script not found at {convert_script}")
        log("  Saving metadata manually...")

        # Save metadata manually
        meta = {
            "input_dim": int(model.InputDim),
            "output_dim": int(model.OutputDim),
            "latent_dim": int(model.LatentDim),
            "sequence_length": int(model.SequenceLength),
            "sequence_window": float(model.SequenceWindow),
            "iterations": 3,
        }
        if hasattr(model, 'InputStats'):
            stats = model.InputStats
            if hasattr(stats, 'Mean'):
                mean = stats.Mean
                std = stats.Std
                if isinstance(mean, torch.Tensor):
                    mean = mean.detach().cpu().numpy()
                    std = std.detach().cpu().numpy()
                meta["input_mean"] = mean.flatten().tolist()
                meta["input_std"] = std.flatten().tolist()
        if hasattr(model, 'OutputStats'):
            stats = model.OutputStats
            if hasattr(stats, 'Mean'):
                mean = stats.Mean
                std = stats.Std
                if isinstance(mean, torch.Tensor):
                    mean = mean.detach().cpu().numpy()
                    std = std.detach().cpu().numpy()
                meta["output_mean"] = mean.flatten().tolist()
                meta["output_std"] = std.flatten().tolist()

        meta_path = os.path.join(output_dir, "Network_meta.json")
        with open(meta_path, 'w') as f:
            json.dump(meta, f, indent=2)
        log(f"  Saved metadata: {meta_path}")


def main():
    parser = argparse.ArgumentParser(description="Train a locomotion model")
    parser.add_argument("--data-dir", required=True, help="Directory with BVH/FBX motion files")
    parser.add_argument("--output-dir", default="models/locomotion", help="Output directory")
    parser.add_argument("--epochs", type=int, default=100, help="Training epochs")
    parser.add_argument("--batch-size", type=int, default=32, help="Batch size")
    parser.add_argument("--lr", type=float, default=1e-4, help="Learning rate")
    parser.add_argument("--sequence-length", type=int, default=16, help="Prediction sequence length")
    parser.add_argument("--sequence-window", type=float, default=0.5, help="Prediction window (seconds)")
    parser.add_argument("--device", default="auto", help="Device: cpu, cuda, or auto")
    args = parser.parse_args()

    log("=" * 60)
    log("AI4Animation — Locomotion Model Training")
    log("=" * 60)

    # Device selection
    if args.device == "auto":
        device = "cuda" if torch.cuda.is_available() else "cpu"
    else:
        device = args.device
    log(f"Device: {device}")
    if device == "cuda":
        log(f"  GPU: {torch.cuda.get_device_name(0)}")

    # Find motion files
    log(f"\nScanning data directory: {args.data_dir}")
    motion_files = find_motion_files(args.data_dir)
    if not motion_files:
        log("ERROR: No motion files found!")
        log(f"  Supported formats: .bvh, .fbx, .glb, .gltf, .npz")
        sys.exit(1)
    log(f"  Found {len(motion_files)} motion files")

    # Load motion data
    log("\nLoading motion data...")
    all_frames = []
    all_fps = []
    for i, fpath in enumerate(motion_files):
        try:
            frames, joint_names, fps = load_bvh_simple(fpath)
            if frames:
                all_frames.append(frames)
                all_fps.append(fps)
                if i == 0:
                    log(f"  Skeleton: {len(joint_names)} joints")
                    log(f"  FPS: {fps}")
            log(f"  [{i+1}/{len(motion_files)}] {Path(fpath).name}: {len(frames)} frames")
        except Exception as e:
            log(f"  [{i+1}/{len(motion_files)}] {Path(fpath).name}: SKIP ({e})")

    if not all_frames:
        log("ERROR: No valid motion data loaded!")
        sys.exit(1)

    total_frames = sum(len(f) for f in all_frames)
    log(f"\nTotal: {len(all_frames)} clips, {total_frames} frames")

    # Build training pairs
    log("\nBuilding training pairs...")
    X, Y = build_training_pairs(
        all_frames, all_fps,
        sequence_length=args.sequence_length,
        sequence_window=args.sequence_window,
    )
    log(f"  Input shape:  {X.shape}")
    log(f"  Output shape: {Y.shape}")

    if X.shape[0] == 0:
        log("ERROR: No training pairs generated! Clips may be too short.")
        sys.exit(1)

    input_dim = X.shape[1]
    output_dim = Y.shape[2]

    # Create model
    log(f"\nCreating model: input={input_dim}, output={output_dim}")
    model = create_model(input_dim, output_dim, args.sequence_length, args.sequence_window)
    num_params = sum(p.numel() for p in model.parameters())
    log(f"  Parameters: {num_params:,}")

    # Output directory
    os.makedirs(args.output_dir, exist_ok=True)

    # Train
    log(f"\nTraining for {args.epochs} epochs (batch_size={args.batch_size}, lr={args.lr})...")
    log("-" * 60)
    model = train(model, X, Y, args.epochs, args.batch_size, args.lr, args.output_dir, device)

    # Export
    log("\n" + "-" * 60)
    export_to_onnx(model, args.output_dir)

    log("\nDone! Files in output directory:")
    for f in os.listdir(args.output_dir):
        size = os.path.getsize(os.path.join(args.output_dir, f))
        log(f"  {f} ({size / 1024:.1f} KB)")


if __name__ == "__main__":
    main()
