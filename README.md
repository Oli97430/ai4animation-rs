<p align="center">
  <img src="https://img.shields.io/badge/Rust-100%25-orange?logo=rust&logoColor=white" />
  <img src="https://img.shields.io/badge/wgpu-22-blue?logo=webgpu" />
  <img src="https://img.shields.io/badge/egui-0.29-green" />
  <img src="https://img.shields.io/badge/ONNX-Runtime-purple?logo=onnx" />
  <img src="https://img.shields.io/badge/lines-38k+-brightgreen" />
  <img src="https://img.shields.io/badge/tests-148-success" />
  <img src="https://img.shields.io/badge/license-MIT-lightgrey" />
</p>

# AI4Animation Engine

**Moteur d'animation 3D temps-réel avec intelligence artificielle, entièrement écrit en Rust.**

Un éditeur professionnel de mouvement conçu pour la recherche en animation par apprentissage, le motion matching, et la synthèse de locomotion neurale — sans aucune dépendance C++ ni Unity.

---

## Pourquoi ce projet ?

Les outils de recherche en animation intelligente (PFNN, DeepPhase, motion matching) restent prisonniers d'Unity/Python. Ce moteur les libère dans un exécutable natif Rust : **un seul binaire, zéro runtime externe**, avec un rendu GPU moderne (wgpu) et une interface fluide (egui).

---

## Fonctionnalités

### Import & Export multi-format

| Format | Import | Export | Détails |
|--------|:------:|:------:|---------|
| **glTF / GLB** | ✅ | ✅ | Meshes, squelettes, skinning, animations |
| **FBX** | ✅ | ✅ | ASCII 7.4 (squelette + animation) |
| **BVH** | ✅ | ✅ | Motion capture, compatible Mixamo / CMU |
| **NPZ** | ✅ | ✅ | Données NumPy (passerelle Python/PyTorch) |
| **USD / USDA** | ✅ | ✅ | OpenUSD ASCII (squelette + animation + mesh) |
| **PNG** | — | ✅ | Capture viewport et export frame par frame |
| **a4a** | ✅ | ✅ | Projets complets (scène, caméra, paramètres) |

**Drag & drop** directement dans la fenêtre. Conversion batch intégrée.

---

### Animation avancée

- **Lecture / scrubbing** — Timeline avec play/pause, vitesse variable, boucle, miroir
- **Dope sheet** — Vue frame par frame, zoom, navigation par clé
- **Crossfade blending** — Transitions douces entre animations avec courbes d'easing (Linear, SmoothStep, EaseIn, EaseOut)
- **Layers d'animation** — Override et additive avec masques par articulation
- **Onion skinning** — Fantômes passé/futur pour analyse du mouvement
- **Copier / coller de pose** — Entre frames, entre modèles
- **Miroir de pose** — Symétrie automatique gauche/droite

---

### Motion Matching

Recherche temps-réel dans une base de données de mouvements :

- **Base de features** — Trajectoire (position + direction futures), pose articulaire, vélocités, contacts au sol
- **Normalisation** — Moyenne/écart-type par dimension pour un matching équilibré
- **Plus proche voisin** — Brute-force optimisé (< 100k frames en temps réel)
- **Contrôleur temps-réel** — Re-requête périodique (100ms), seuil de transition, fenêtre d'exclusion
- **Crossfade automatique** — Blending doux entre clips matchés
- **Poids configurables** — Trajectoire, pose, vélocité, contacts : tout est ajustable

---

### Machine d'états d'animation

Éditeur visuel de graphe de transitions :

- **États nommés** — Chaque état référence un clip d'animation chargé
- **Transitions conditionnelles** — Paramètres bool, seuils float, temps écoulé, fin d'animation
- **Priorités** — Les transitions haute priorité sont évaluées en premier
- **Éditeur node-graph** — Drag & drop des états, flèches de transition, labels de condition
- **Paramètres runtime** — Modifiables en temps réel depuis l'interface ou par commande IA

---

### Édition de pose interactive

- **Sélection directe** — Cliquez sur un os dans le viewport 3D
- **Manipulation** — Drag pour déplacer, rotation contrainte par axe (X/Y/Z)
- **Éditeur numérique** — Position XYZ et rotation Euler avec DragValue précis
- **Auto-keyframe** — Chaque modification s'enregistre automatiquement dans l'animation
- **Navigation hiérarchique** — Parcourez parent/enfants en un clic
- **Réinitialisation** — Retour à la pose originale de l'animation

---

### Locomotion neurale (ONNX)

Synthèse de mouvement par réseau de neurones :

- **Inférence ONNX** — Chargement de modèles entraînés (.onnx) avec métadonnées
- **Contrôle WASD** — Locomotion interactive clavier avec sprint (Shift)
- **Entraînement** — Lancement de training PyTorch depuis l'interface, avec log en temps réel
- **Conversion** — Export PyTorch → ONNX intégré

---

### Cinématique inverse (IK)

- **FABRIK** — Algorithme itératif avec convergence paramétrable
- **Contraintes articulaires** — Presets anatomiques (coude, genou, épaule) ou angles personnalisés
- **Pole target** — Contrôle du plan de flexion (ex: direction du genou)
- **IK interactif** — Sélectionnez root + tip, puis glissez dans le viewport

---

### Modules d'analyse

| Module | Description |
|--------|-------------|
| **Contacts** | Détection automatique des contacts pieds/sol par seuil de hauteur |
| **Trajectoire** | Échantillonnage passé/futur de la trajectoire racine |
| **Guidage** | Positions de guidage futures pour le contrôle directionnel |
| **Suivi** | Trajectoires multi-joints (tête, mains) avec vélocités |
| **Root Motion** | Extraction position/vitesse/direction de la racine |
| **Phase** | Détection de phase cyclique (marche, course) |
| **Retarget** | Transfert d'animation entre squelettes différents |

---

### Rendu GPU (wgpu)

- **Skinning GPU** — Déformation de mesh en temps réel
- **Debug draw** — Squelette, vélocités, trajectoires, contacts, gizmos
- **Grille infinie** — Style professionnel avec axes RGB = XYZ
- **Caméra** — Orbit, pan, zoom, vues prédéfinies (face/droite/dessus), mode libre WASD
- **Éclairage** — Directional + ambient, ombres configurables
- **Post-processing** — Réglages de rendu (qualité, effets)
- **Capture** — Export PNG haute résolution du viewport

---

### Assistant IA intégré

Un chat IA directement dans l'éditeur, capable de piloter **70+ commandes** :

```
"Crée un humanoïde de 1m80 avec une animation de course"
→ L'IA génère le squelette, le mesh skinné, et l'animation procédurale

"Change l'animation en marche et mets la caméra de face"
→ L'IA enchaîne create_animation + camera_view en une seule réponse

"Construis la base de motion matching et active-le"
→ build_motion_db + toggle_motion_matching
```

**Providers supportés** : Claude (Anthropic), GPT-4 (OpenAI), Ollama (local)

Commandes disponibles : import, export, playback, caméra, sélection, transforms, outils, affichage, rendu, IK, console, panneaux, requêtes scène, génération procédurale, locomotion, entraînement, motion matching, machine d'états, tissu/soft-body, ragdoll, matériaux, blend trees, enregistrement d'animation, créatures procédurales.

---

### Interface professionnelle

- **25 panneaux** — Hiérarchie, inspecteur, timeline, **timeline Flash** (style Macromedia), dope sheet, console, éditeur de mouvement, profiler, enregistreur vidéo, navigateur d'assets, raccourcis, chat IA, entraînement, motion matching, machine d'états, éditeur de pose, réglages de rendu, batch converter, tissu/soft-body, matériaux PBR, IK avancé, ragdoll, blend tree, enregistreur d'animation, DeepPhase...
- **Thème sombre** — Style professionnel inspiré Blender/SketchUp
- **Raccourcis configurables** — Éditeur visuel de keybindings
- **Undo/Redo** — Historique de 100 actions
- **Multi-sélection** — Ctrl+click, sélection multiple
- **Fichiers récents** — Accès rapide aux derniers fichiers ouverts
- **Auto-sauvegarde** — Backup automatique toutes les 5 minutes
- **Internationalisation** — Interface en français par défaut

---

## Architecture

```
ai4animation-rs/
├── crates/
│   ├── anim-math        # Maths 3D, quaternions, signaux, batch ndarray
│   ├── anim-core        # Scène, entités, temps, profiler, i18n
│   ├── anim-import      # GLB, FBX, BVH, NPZ, procédural, presets
│   ├── anim-animation   # Motion, blend, IK, contacts, locomotion,
│   │                    # motion matching, state machine, retarget,
│   │                    # keyframes, shape keys, particles, camera anim...
│   ├── anim-ik          # FABRIK, contraintes, pole targets
│   ├── anim-render      # wgpu renderer, skinning, debug draw, capture
│   ├── anim-gui         # egui panels, app state, thème, raccourcis
│   ├── anim-ai          # Claude/GPT/Ollama, 70+ commandes structurées
│   └── anim-app         # Point d'entrée, boucle principale
├── samples/             # Fichiers exemples (BVH, FBX, NPZ)
├── docs/                # Didacticiel complet
└── Cargo.toml           # Workspace avec 9 crates
```

**120 fichiers Rust** | **~38 000 lignes de code** | **148 tests unitaires** | **28 fichiers exemples**

---

## Stack technique

| Catégorie | Technologie |
|-----------|-------------|
| Langage | **Rust** (100%, stable) |
| Rendu | **wgpu 22** (Vulkan/DX12/Metal) |
| Interface | **egui 0.29** + eframe |
| 3D Formats | gltf 1.4, fbxcel 0.9 |
| IA Inference | **ONNX Runtime 2.0** (ort) |
| Maths | **glam 0.29**, ndarray 0.16 |
| Réseau | ureq 3 (API calls IA) |
| Concurrence | rayon, parking_lot |
| Sérialisation | serde, serde_json |
| Target | `x86_64-pc-windows-msvc` |

---

## Build & lancement

```bash
# Prérequis : Rust toolchain (rustup)
git clone https://github.com/user/ai4animation-rs.git
cd ai4animation-rs
cargo run --release
```

L'exécutable est autonome. Aucune dépendance externe à installer.

Pour l'inférence ONNX, le runtime est téléchargé automatiquement au premier build.

---

## Utilisation rapide

1. **Glissez** un fichier `.glb`, `.fbx`, `.bvh` ou `.npz` dans la fenêtre
2. **Appuyez** sur Espace pour lancer la lecture
3. **Ouvrez** le chat IA et tapez une commande en langage naturel
4. **Explorez** les modules via le menu Affichage

---

## Roadmap

- [x] Export FBX (ASCII 7.4)
- [x] Export GLB
- [x] Export USD / USDA
- [x] Graphe de blend trees visuel
- [x] Simulation tissu / soft-body (PBD)
- [x] Ragdoll physique
- [x] Éditeur de matériaux PBR
- [x] Créatures procédurales (5 types)
- [x] Enregistrement d'animation
- [x] Assistant IA : contrôle complet (70+ commandes)
- [x] Fichiers exemples + didacticiel
- [x] Timeline Flash (style Macromedia) avec keyframes, tweens, layers
- [x] Système de keyframes avec interpolation Flash (6 types de tween)
- [x] Primitives 3D (sphère, cube, plan, cylindre, cône, tore)
- [x] Import de textures (PNG/JPG) + textures procédurales
- [x] Shape Keys / Morph Targets
- [x] Textures PBR : normal map, metallic-roughness, emission
- [x] Animation caméra (orbite, dolly, zoom) avec easing
- [x] Système de particules (feu, fumée, poussière, étincelles, neige, pluie)
- [ ] GPU-accelerated motion matching (compute shaders)
- [ ] Plugin système pour extensions custom
- [ ] Support Linux / macOS natif
- [ ] Éditeur de courbes d'animation (graph editor)
- [ ] Réseau multi-utilisateur (collaboration temps réel)

---

## Comparaison Python vs Rust

Ce moteur Rust est le successeur de la version Python (`ai4animationpy`). Voici la comparaison :

### Volume de code

| Métrique | Python | Rust | Ratio |
|----------|--------|------|-------|
| **Fichiers source** | 92 `.py` | 120 `.rs` | x1.3 |
| **Lignes de code** | 15 885 | 38 010 | x2.4 |
| **Tests unitaires** | 0 | 148 | — |
| **Fichiers exemples** | 0 | 28 | — |

### Moteur de rendu

| | Python | Rust |
|---|--------|------|
| **API graphique** | Raylib (OpenGL) | **wgpu** (Vulkan/DX12/Metal) |
| **GUI** | Widgets Raylib custom | **egui** (immediate mode) |
| **Rendu** | Forward + post-process | PBR + SSAO + Bloom + FXAA + Ombres |
| **Performance** | ~30 fps (interprete) | **300+ fps** (natif compile) |

### Fonctionnalites

| Fonctionnalite | Python | Rust |
|----------------|:------:|:----:|
| Import GLB/FBX/BVH/NPZ | ✅ | ✅ |
| Import USD | — | ✅ |
| Export BVH/NPZ | ✅ | ✅ |
| Export FBX/GLB/USD | — | ✅ |
| Squelette anime | ✅ | ✅ |
| Skinned mesh GPU | ✅ | ✅ |
| IK (FABRIK) | ✅ | ✅ + pole target + contraintes |
| Module Contact | ✅ | ✅ |
| Module Guidance | ✅ | ✅ |
| Module Tracking | ✅ | ✅ |
| Editeur materiaux PBR | — | ✅ (7 presets) |
| Simulation tissu (PBD) | — | ✅ |
| Ragdoll physique | — | ✅ |
| Motion Matching | — | ✅ |
| Blend Trees | — | ✅ |
| Machine d'etats | — | ✅ |
| DeepPhase | — | ✅ |
| Creatures procedurales | — | ✅ (5 types) |
| Humanoides proceduraux | — | ✅ (mesh+squelette+anim) |
| Multi-personnages | — | ✅ |
| Onion skinning | — | ✅ |
| Enregistrement animation | — | ✅ |
| Dope Sheet | — | ✅ |
| Projet sauvegarde (.a4a) | — | ✅ |

### Intelligence artificielle

| | Python | Rust |
|---|--------|------|
| **Entrainement** | ✅ PyTorch natif | Lance Python en subprocess |
| **Architectures** | MLP, Autoencoder, Flow, Codebook | Via modeles ONNX pre-entraines |
| **Inference ONNX** | ✅ onnxruntime-gpu | ✅ onnxruntime |
| **Assistant IA chat** | — | ✅ (Ollama/OpenAI/Claude) |
| **Commandes IA** | — | ✅ (70+ commandes JSON) |
| **Controle total par IA** | — | ✅ |

### Distribution

| | Python | Rust |
|---|--------|------|
| **Installation** | pip install + PyTorch (~2 GB) + raylib | **Un seul binaire** |
| **Dependencies runtime** | 15+ packages Python | 0 |
| **Securite memoire** | GC Python | Borrow checker (compile-time) |

### Verdict

> La version **Python** est un **outil de recherche** — excellent pour iterer sur des architectures neuronales et visualiser des resultats d'entrainement.
>
> La version **Rust** est un **editeur professionnel** — elle reprend toutes les capacites d'animation Python, ajoute 15+ fonctionnalites nouvelles (physique, export multi-format, creatures, IA chat), et tourne a des performances natives.
>
> **Les deux sont complementaires** : on entraine en Python, on deploie en Rust.

---

## Inspirations

Ce projet s'inspire des travaux de recherche de [Sebastian Starke](https://github.com/sebastianstarke/AI4Animation) (AI4Animation Unity), transposés dans un moteur natif Rust pour la performance, la portabilité et l'autonomie vis-à-vis de moteurs propriétaires.

---

<p align="center">
  <em>Construit avec Rust, wgpu et beaucoup de cafe.</em>
</p>
