//! Generate sample data files for AI4Animation Engine.
//!
//! Run with: cargo run --example gen_samples
//! Output goes to ../../samples/

use std::path::Path;
use anim_import::{
    generate_humanoid_with_animation, generate_creature,
    export_bvh_sequence, export_npz, export_fbx,
    HumanoidConfig, BodyColors,
};

fn main() {
    let samples_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()
        .parent().unwrap()
        .join("samples");

    std::fs::create_dir_all(&samples_dir).expect("Failed to create samples dir");

    println!("=== AI4Animation Sample Generator ===");
    println!("Output: {}\n", samples_dir.display());

    // ── Humanoid animations ──────────────────────────────
    let anims = [
        ("walk",  "Marche",   2.5),
        ("run",   "Course",   2.0),
        ("idle",  "Repos",    3.0),
        ("jump",  "Saut",     1.5),
    ];

    for (anim_type, label, duration) in &anims {
        let config = HumanoidConfig {
            name: format!("Humanoide_{}", label),
            height: 1.75,
            ..Default::default()
        };
        let model = generate_humanoid_with_animation(&config, anim_type, *duration);

        if let Some(ref anim_data) = model.animation_frames {
            // BVH
            let bvh_path = samples_dir.join(format!("humanoid_{}.bvh", anim_type));
            match export_bvh_sequence(
                &bvh_path,
                &model.joint_names,
                &model.parent_indices,
                &anim_data.frames,
                anim_data.framerate,
            ) {
                Ok(()) => println!("  ✓ {} ({} frames)", bvh_path.display(), anim_data.frames.len()),
                Err(e) => eprintln!("  ✗ {}: {}", bvh_path.display(), e),
            }

            // NPZ
            let npz_path = samples_dir.join(format!("humanoid_{}.npz", anim_type));
            match export_npz(
                &npz_path,
                &model.joint_names,
                &model.parent_indices,
                &anim_data.frames,
                anim_data.framerate,
            ) {
                Ok(()) => println!("  ✓ {} ({} frames)", npz_path.display(), anim_data.frames.len()),
                Err(e) => eprintln!("  ✗ {}: {}", npz_path.display(), e),
            }
        }

        // FBX
        let fbx_path = samples_dir.join(format!("humanoid_{}.fbx", anim_type));
        match export_fbx(
            &fbx_path,
            &model,
            model.animation_frames.as_ref().map(|a| &a.frames),
            30.0,
        ) {
            Ok(()) => println!("  ✓ {}", fbx_path.display()),
            Err(e) => eprintln!("  ✗ {}: {}", fbx_path.display(), e),
        }
    }

    // ── Humanoid with custom colors ──────────────────────
    let colored_config = HumanoidConfig {
        name: "Runner_Blue".into(),
        height: 1.80,
        colors: BodyColors {
            skin: [100, 80, 60, 255],
            shirt: [30, 60, 180, 255],
            pants: [20, 20, 20, 255],
            shoes: [200, 50, 50, 255],
            hair: [40, 30, 20, 255],
        },
        ..Default::default()
    };
    let colored_model = generate_humanoid_with_animation(&colored_config, "run", 2.0);
    let fbx_path = samples_dir.join("humanoid_colored_run.fbx");
    match export_fbx(
        &fbx_path,
        &colored_model,
        colored_model.animation_frames.as_ref().map(|a| &a.frames),
        30.0,
    ) {
        Ok(()) => println!("  ✓ {}", fbx_path.display()),
        Err(e) => eprintln!("  ✗ {}: {}", fbx_path.display(), e),
    }

    // ── Creatures ────────────────────────────────────────
    let creatures = [
        ("spider",    0.3),
        ("crab",      0.25),
        ("bird",      0.4),
        ("snake",     0.15),
        ("quadruped", 0.8),
    ];

    println!("\n── Créatures procédurales ──");
    for (creature_type, height) in &creatures {
        let model = generate_creature(creature_type, *height);

        // FBX export for creatures
        let fbx_path = samples_dir.join(format!("creature_{}.fbx", creature_type));
        match export_fbx(&fbx_path, &model, None, 30.0) {
            Ok(()) => println!("  ✓ {} ({} joints, {} vertices)",
                fbx_path.display(),
                model.joint_names.len(),
                model.meshes.first().map_or(0, |m| m.vertices.len()),
            ),
            Err(e) => eprintln!("  ✗ {}: {}", fbx_path.display(), e),
        }

        // BVH for creatures (static pose)
        let bvh_path = samples_dir.join(format!("creature_{}.bvh", creature_type));
        if let Some(ref anim) = model.animation_frames {
            match export_bvh_sequence(
                &bvh_path,
                &model.joint_names,
                &model.parent_indices,
                &anim.frames,
                anim.framerate,
            ) {
                Ok(()) => println!("  ✓ {}", bvh_path.display()),
                Err(e) => eprintln!("  ✗ {}: {}", bvh_path.display(), e),
            }
        } else {
            // Export rest pose as single-frame BVH
            if !model.joint_names.is_empty() {
                let identity_frames = vec![
                    model.joint_names.iter().map(|_| glam::Mat4::IDENTITY).collect::<Vec<_>>()
                ];
                match export_bvh_sequence(
                    &bvh_path,
                    &model.joint_names,
                    &model.parent_indices,
                    &identity_frames,
                    30.0,
                ) {
                    Ok(()) => println!("  ✓ {} (rest pose)", bvh_path.display()),
                    Err(e) => eprintln!("  ✗ {}: {}", bvh_path.display(), e),
                }
            }
        }
    }

    // ── Multi-character scene (3 humanoids with different anims) ──
    println!("\n── Scène multi-personnage ──");
    let scene_configs: [(&str, &str, f32, [u8; 4], [u8; 4]); 3] = [
        ("Marcheur",  "walk", 1.70, [220, 185, 155, 255], [60, 90, 160, 255]),
        ("Coureur",   "run",  1.80, [180, 140, 100, 255], [200, 50, 50, 255]),
        ("Danseur",   "idle", 1.65, [130, 100, 80, 255],  [50, 180, 80, 255]),
    ];

    for (name, anim, height, skin, shirt) in &scene_configs {
        let config = HumanoidConfig {
            name: name.to_string(),
            height: *height,
            colors: BodyColors {
                skin: *skin,
                shirt: *shirt,
                ..Default::default()
            },
            ..Default::default()
        };
        let model = generate_humanoid_with_animation(&config, anim, 3.0);

        let bvh_path = samples_dir.join(format!("scene_{}.bvh", name.to_lowercase()));
        if let Some(ref anim_data) = model.animation_frames {
            match export_bvh_sequence(
                &bvh_path,
                &model.joint_names,
                &model.parent_indices,
                &anim_data.frames,
                anim_data.framerate,
            ) {
                Ok(()) => println!("  ✓ {}", bvh_path.display()),
                Err(e) => eprintln!("  ✗ {}: {}", bvh_path.display(), e),
            }
        }

        let fbx_path = samples_dir.join(format!("scene_{}.fbx", name.to_lowercase()));
        match export_fbx(
            &fbx_path,
            &model,
            model.animation_frames.as_ref().map(|a| &a.frames),
            30.0,
        ) {
            Ok(()) => println!("  ✓ {}", fbx_path.display()),
            Err(e) => eprintln!("  ✗ {}: {}", fbx_path.display(), e),
        }
    }

    println!("\n=== Génération terminée ===");
}
