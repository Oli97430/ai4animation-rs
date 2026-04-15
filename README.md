<p align="center">
  <img src="https://img.shields.io/badge/Rust-100%25-orange?logo=rust&logoColor=white" />
  <img src="https://img.shields.io/badge/wgpu-22-blue?logo=webgpu" />
  <img src="https://img.shields.io/badge/egui-0.29-green" />
  <img src="https://img.shields.io/badge/ONNX-Runtime-purple?logo=onnx" />
  <img src="https://img.shields.io/badge/lines-22k+-brightgreen" />
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
| **glTF / GLB** | ✅ | — | Meshes, squelettes, skinning, animations |
| **FBX** | ✅ | — | Support natif via fbxcel (pas de SDK Autodesk) |
| **BVH** | ✅ | ✅ | Motion capture, compatible Mixamo / CMU |
| **NPZ** | ✅ | ✅ | Données NumPy (passerelle Python/PyTorch) |
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

Un chat IA directement dans l'éditeur, capable de piloter **39 commandes** :

```
"Crée un humanoïde de 1m80 avec une animation de course"
→ L'IA génère le squelette, le mesh skinné, et l'animation procédurale

"Change l'animation en marche et mets la caméra de face"
→ L'IA enchaîne create_animation + camera_view en une seule réponse

"Construis la base de motion matching et active-le"
→ build_motion_db + toggle_motion_matching
```

**Providers supportés** : Claude (Anthropic), GPT-4 (OpenAI), Ollama (local)

Commandes disponibles : import, export, playback, caméra, sélection, transforms, outils, affichage, rendu, IK, console, panneaux, requêtes scène, génération procédurale, locomotion, entraînement, motion matching, machine d'états.

---

### Interface professionnelle

- **20 panneaux** — Hiérarchie, inspecteur, timeline, dope sheet, console, éditeur de mouvement, profiler, enregistreur vidéo, navigateur d'assets, raccourcis, chat IA, entraînement, motion matching, machine d'états, éditeur de pose, réglages de rendu, batch converter...
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
│   │                    # motion matching, state machine, retarget...
│   ├── anim-ik          # FABRIK, contraintes, pole targets
│   ├── anim-render      # wgpu renderer, skinning, debug draw, capture
│   ├── anim-gui         # egui panels, app state, thème, raccourcis
│   ├── anim-ai          # Claude/GPT/Ollama, 39 commandes structurées
│   └── anim-app         # Point d'entrée, boucle principale
└── Cargo.toml           # Workspace avec 9 crates
```

**97 fichiers Rust** | **~22 000 lignes de code** | **22 tests unitaires**

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

- [ ] Export FBX et glTF
- [ ] Graphe de blend trees visuel
- [ ] GPU-accelerated motion matching (compute shaders)
- [ ] Plugin système pour extensions custom
- [ ] Support Linux / macOS natif
- [ ] Éditeur de courbes d'animation (graph editor)
- [ ] Réseau multi-utilisateur (collaboration temps réel)

---

## Inspirations

Ce projet s'inspire des travaux de recherche de [Sebastian Starke](https://github.com/sebastianstarke/AI4Animation) (AI4Animation Unity), transposés dans un moteur natif Rust pour la performance, la portabilité et l'autonomie vis-à-vis de moteurs propriétaires.

---

<p align="center">
  <em>Construit avec Rust, wgpu et beaucoup de café.</em>
</p>
