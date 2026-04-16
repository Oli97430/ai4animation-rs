# Fichiers exemples / Sample Files

Ces fichiers sont générés par `cargo run --example gen_samples -p anim-import`.

## Humanoïdes animés
| Fichier | Format | Description |
|---------|--------|-------------|
| `humanoid_walk.*` | BVH/NPZ/FBX | Marche cyclique (75 frames, 2.5s) |
| `humanoid_run.*` | BVH/NPZ/FBX | Course cyclique (60 frames, 2.0s) |
| `humanoid_idle.*` | BVH/NPZ/FBX | Repos avec balancement (90 frames, 3.0s) |
| `humanoid_jump.*` | BVH/NPZ/FBX | Saut sur place (45 frames, 1.5s) |
| `humanoid_colored_run.fbx` | FBX | Course avec couleurs personnalisées |

## Créatures procédurales
| Fichier | Joints | Description |
|---------|--------|-------------|
| `creature_spider.*` | 27 | Araignée (8 pattes articulées) |
| `creature_crab.*` | 27 | Crabe (6 pattes + 2 pinces) |
| `creature_bird.*` | 17 | Oiseau (ailes + pattes) |
| `creature_snake.*` | 16 | Serpent (segments articulés) |
| `creature_quadruped.*` | 20 | Quadrupède (4 pattes + queue) |

## Scène multi-personnage
| Fichier | Animation | Taille |
|---------|-----------|--------|
| `scene_marcheur.*` | Marche | 1.70m |
| `scene_coureur.*` | Course | 1.80m |
| `scene_danseur.*` | Repos | 1.65m |

## Régénérer
```bash
cargo run --example gen_samples -p anim-import
```
