# AI4Animation Engine — Didacticiel complet

## Table des matières

1. [Installation et premier lancement](#1-installation-et-premier-lancement)
2. [Interface utilisateur](#2-interface-utilisateur)
3. [Importer un modèle](#3-importer-un-modèle)
4. [Lecture et timeline](#4-lecture-et-timeline)
5. [Outils de transformation](#5-outils-de-transformation)
6. [Cinématique inverse (IK)](#6-cinématique-inverse-ik)
7. [Création procédurale](#7-création-procédurale)
8. [Éditeur de matériaux](#8-éditeur-de-matériaux)
9. [Simulation tissu / soft-body](#9-simulation-tissu--soft-body)
10. [Physique ragdoll](#10-physique-ragdoll)
11. [Enregistrement d'animation](#11-enregistrement-danimation)
12. [Export multi-format](#12-export-multi-format)
13. [Motion Matching & Blend Trees](#13-motion-matching--blend-trees)
14. [Locomotion IA (ONNX)](#14-locomotion-ia-onnx)
15. [L'assistant IA](#15-lassistant-ia)
16. [Raccourcis clavier](#16-raccourcis-clavier)
17. [FAQ](#17-faq)

---

## 1. Installation et premier lancement

### Prérequis
- **Rust** 1.75+ (avec `cargo`)
- **GPU** compatible Vulkan, DirectX 12 ou Metal (via wgpu)
- **Windows 10/11** (cible principale), Linux et macOS supportés

### Compilation
```bash
git clone https://github.com/Oli97430/ai4animation-rs
cd ai4animation-rs
cargo build --release
```

### Lancement
```bash
cargo run --release -p anim-app
```

### Générer les fichiers exemples
```bash
cargo run --example gen_samples -p anim-import
```
Cela crée le dossier `samples/` avec des humanoïdes animés (marche, course, repos, saut), 5 créatures procédurales et des scènes multi-personnages.

---

## 2. Interface utilisateur

L'interface est divisée en plusieurs zones :

```
┌──────────────────────────────────────────────┐
│  Barre de menu (Fichier, Affichage, Outils)  │
├──────────────────────────────────────────────┤
│                                              │
│           Viewport 3D (wgpu)                 │
│                                              │
├──────────────────────────────────────────────┤
│  Timeline / Contrôles de lecture             │
└──────────────────────────────────────────────┘
```

### Panneaux flottants
Activez les panneaux depuis le menu **Affichage** :

| Panneau | Raccourci | Description |
|---------|-----------|-------------|
| Console | F12 | Messages système et logs |
| Profiler | - | Statistiques de performance |
| Dope Sheet | - | Vue temporelle des keyframes |
| Motion Editor | - | Modules de mouvement (contact, guidance, tracking) |
| Enregistreur | - | Capture d'animation en temps réel |
| Batch | - | Traitement par lots |
| Asset Browser | - | Navigateur de fichiers avec aperçu |
| Rendu | - | Éclairage, SSAO, ombres, bloom, FXAA |
| Chat IA | - | Assistant IA intégré |
| Ragdoll | - | Simulation physique |
| Graph Editor | - | Machine d'états |
| Blend Tree | - | Arbre de mélange d'animations |
| Enregistreur anim | - | Capture de transforms |
| DeepPhase | - | Manifold de phase pour transitions |
| Matériaux | - | Éditeur PBR |
| Tissu | - | Simulation soft-body |
| IK avancé | - | Cinématique inverse FABRIK |

### Navigation 3D
- **Clic droit + glisser** : Orbiter autour du modèle
- **Molette** : Zoom avant/arrière
- **Clic molette + glisser** : Panoramique
- **Double clic** : Centrer sur l'objet cliqué

---

## 3. Importer un modèle

### Formats supportés
| Format | Extension | Contenu |
|--------|-----------|---------|
| glTF/GLB | `.glb`, `.gltf` | Mesh + squelette + animation + textures |
| BVH | `.bvh` | Squelette + animation (motion capture) |
| NPZ | `.npz` | Données numériques (positions, rotations) |
| FBX | `.fbx` | Mesh + squelette + animation (binaire ou ASCII) |
| USD/USDA | `.usd`, `.usda` | Scène complète (OpenUSD) |

### Méthode 1 : Menu Fichier
1. **Fichier > Importer GLB/BVH/FBX/NPZ/USD**
2. Sélectionnez votre fichier dans le dialogue natif
3. Le modèle apparaît dans le viewport

### Méthode 2 : Asset Browser
1. **Affichage > Asset Browser**
2. Naviguez dans vos dossiers
3. Cliquez sur le bouton **📦** pour accéder aux fichiers exemples
4. Double-cliquez sur un fichier ou sélectionnez-le et cliquez **📥 Charger**

### Méthode 3 : Assistant IA
```
> Importe le fichier samples/humanoid_walk.bvh
```

### Fichiers exemples inclus
Après `cargo run --example gen_samples`, vous disposez de :
- `humanoid_walk.bvh/npz/fbx` — Marche cyclique (75 frames)
- `humanoid_run.bvh/npz/fbx` — Course (60 frames)
- `humanoid_idle.bvh/npz/fbx` — Repos (90 frames)
- `humanoid_jump.bvh/npz/fbx` — Saut (45 frames)
- `creature_spider.fbx` — Araignée (27 joints)
- `creature_crab.fbx` — Crabe (27 joints)
- `creature_bird.fbx` — Oiseau (17 joints)
- `creature_snake.fbx` — Serpent (16 joints)
- `creature_quadruped.fbx` — Quadrupède (20 joints)

---

## 4. Lecture et timeline

### Contrôles de lecture
| Bouton | Action |
|--------|--------|
| ▶ / ⏸ | Lecture / Pause |
| ⏹ | Stop (retour au début) |
| ⏮ / ⏭ | Frame précédente / suivante |

### Paramètres
- **Vitesse** : Glisseur de 0.1× à 5× (défaut : 1×)
- **Boucle** : Active la lecture en boucle
- **Miroir** : Inverse gauche/droite

### Via l'IA
```
> Joue l'animation à vitesse 2
> Va à la frame 42
> Active la boucle
```

---

## 5. Outils de transformation

### Sélection d'outil
Accessible depuis la barre d'outils ou via l'IA :

| Outil | Description |
|-------|-------------|
| Sélection | Cliquer pour sélectionner un joint/entité |
| Déplacer | Translater l'objet sélectionné |
| Rotation | Tourner l'objet sélectionné |
| Mesure | Mesurer des distances dans la scène |
| IK | Mode cinématique inverse |

### Contrainte d'axe
Verrouillez un axe (X, Y ou Z) pour restreindre la transformation.

### Accrochage à la grille
Activez l'accrochage pour aligner les déplacements sur une grille configurable.

---

## 6. Cinématique inverse (IK)

Le solveur **FABRIK** (Forward And Backward Reaching Inverse Kinematics) permet de positionner les extrémités d'une chaîne articulée.

### Utilisation
1. Ouvrez le panneau **IK avancé** (menu Affichage)
2. Sélectionnez une chaîne prédéfinie :
   - **Bras G** / **Bras D** : Épaule → Main
   - **Jambe G** / **Jambe D** : Hanche → Pied
   - **Colonne** : Hanches → Tête
3. Ajustez la **position cible** (X, Y, Z)
4. Optionnellement, activez :
   - **Limites angulaires** : Contraindre les rotations
   - **Pole target** : Contrôler l'orientation du coude/genou
5. Cliquez **Résoudre IK**

### Préréglages
| Préréglage | Utilisation |
|------------|------------|
| Bras humain | Épaule-coude-poignet avec limites naturelles |
| Jambe humaine | Hanche-genou-cheville |
| Personnalisé | Configuration manuelle |

### Via l'IA
```
> Résous l'IK du bras gauche vers (1, 1.5, 0.5)
```

---

## 7. Création procédurale

### Humanoïdes
Créez un personnage humanoïde complet avec squelette, mesh et animation :

**Via l'IA :**
```
> Crée un humanoïde de 1.80m qui court pendant 3 secondes
> Crée un personnage avec un t-shirt rouge qui marche
```

**Paramètres disponibles :**
- **Taille** : 0.5m à 2.5m
- **Animation** : marche, course, repos, saut
- **Couleurs** : peau, chemise, pantalon, chaussures, cheveux

### Créatures procédurales
5 types de créatures avec squelettes adaptés :

| Créature | Joints | Description |
|----------|--------|-------------|
| Araignée | 27 | Corps + tête + abdomen + 8 pattes (3 segments chacune) |
| Crabe | 27 | Corps + yeux + 2 pinces + 6 pattes |
| Oiseau | 17 | Corps + cou/tête/bec + queue + 2 ailes + 2 pattes |
| Serpent | 16 | 16 segments articulés |
| Quadrupède | 20 | Colonne + queue + 4 pattes |

**Via l'IA :**
```
> Crée une araignée de 30cm
> Crée un cheval
> Crée un oiseau de 40cm
```

---

## 8. Éditeur de matériaux

### Propriétés PBR
L'éditeur de matériaux modifie le rendu physiquement correct (PBR) :

| Propriété | Plage | Description |
|-----------|-------|-------------|
| Couleur | Sélecteur | Couleur de base (albedo) |
| Alpha | 0-1 | Transparence |
| Métallique | 0-1 | 0 = diélectrique, 1 = métal |
| Rugosité | 0-1 | 0 = miroir, 1 = mat |
| Spéculaire | 0-1 | Intensité des reflets |
| Brillance | 0-128 | Taille du point spéculaire |

### Préréglages
7 matériaux prédéfinis : Pierre, Or, Plastique, Bois, Chrome, Verre, Brique.

### Via l'IA
```
> Mets un matériau doré sur le modèle
> Change la couleur en rouge
> Rends le modèle transparent
```

---

## 9. Simulation tissu / soft-body

### Créer un tissu
1. Ouvrez le panneau **Tissu / Soft-body**
2. Configurez la grille (largeur × hauteur, taille en mètres)
3. Cliquez **Créer grille** ou **Créer chaîne (corde)**
4. La simulation démarre automatiquement

### Paramètres
| Paramètre | Défaut | Description |
|-----------|--------|-------------|
| Gravité Y | -9.81 | Force gravitationnelle |
| Amortissement | 0.01 | Résistance au mouvement |
| Rigidité | 0.8 | Résistance à l'étirement |
| Itérations | 5 | Précision du solveur |
| Sol Y | 0.0 | Hauteur du sol |
| Vent X/Z | 0.0 | Force du vent |

### Contrôles
- **Pause/Reprendre** : Geler/relancer la simulation
- **Reset** : Revenir à la position initiale
- **Supprimer** : Détruire le tissu

### Via l'IA
```
> Crée un tissu 15x15 de 2 mètres
> Active le vent
```

---

## 10. Physique ragdoll

### Créer un ragdoll
1. Chargez un modèle humanoïde
2. Ouvrez le panneau **Ragdoll**
3. Cliquez **Créer ragdoll** — le squelette devient physique

### Interactions
- **Impulsion** : Appliquer une force à tous les corps
- **Explosion** : Force radiale depuis l'origine
- **Épingler** : Fixer un corps (ex: épingler la racine pour un mannequin suspendu)
- **Toggle** : Activer/désactiver la simulation

### Via l'IA
```
> Crée un ragdoll
> Applique une explosion de force 50
> Épingle le corps 0
```

---

## 11. Enregistrement d'animation

### Enregistrer en temps réel
1. Ouvrez le panneau **Enregistreur anim**
2. Cliquez **Démarrer** — chaque frame est capturée
3. Interagissez (IK, ragdoll, locomotion...)
4. Cliquez **Arrêter** — le clip est sauvegardé comme Motion

### Utilisation
Les clips enregistrés peuvent être :
- Relus dans la timeline
- Exportés en BVH/FBX/USD
- Ajoutés à la base Motion Matching

### Via l'IA
```
> Démarre l'enregistrement
> Arrête l'enregistrement
```

---

## 12. Export multi-format

### Formats d'export

| Format | Contenu exporté |
|--------|----------------|
| **GLB** | Mesh + squelette + animation + textures (binaire compact) |
| **FBX** | Squelette + animation (ASCII 7.4) |
| **USD/USDA** | Squelette + animation + mesh (ASCII, OpenUSD) |
| **BVH** | Squelette + animation (motion capture, frame actuelle ou séquence) |
| **NPZ** | Données brutes compressées (NumPy) |
| **PNG** | Capture d'écran du viewport |

### Via le menu
**Fichier > Exporter** puis choisir le format.

### Via l'IA
```
> Exporte en GLB sous output.glb
> Exporte en FBX sous animation.fbx
> Exporte en USD sous scene.usda
```

---

## 13. Motion Matching & Blend Trees

### Motion Matching
Le Motion Matching sélectionne automatiquement le clip d'animation le plus approprié en temps réel.

1. Chargez plusieurs clips d'animation
2. Cliquez **Construire base** dans le panneau dédié
3. Activez le Motion Matching
4. Le système choisit le meilleur clip à chaque instant

### Blend Trees
Les Blend Trees mélangent plusieurs animations selon des paramètres :

| Type de noeud | Description |
|---------------|-------------|
| Clip | Animation source |
| Blend 1D | Mélange selon un paramètre (ex: vitesse) |
| Blend 2D | Mélange selon deux paramètres (ex: direction X/Y) |
| Lerp | Interpolation linéaire entre deux sources |

### Via l'IA
```
> Construis la base de motion matching
> Active le motion matching
> Crée un noeud blend 1D "Locomotion" avec le paramètre "speed"
```

---

## 14. Locomotion IA (ONNX)

### Chargement de modèle
Le système supporte l'inférence ONNX pour la locomotion neurale.

1. Placez votre modèle `.onnx` et son fichier `_meta.json` dans `models/locomotion/`
2. Via l'IA : `Charge le modèle de locomotion`
3. Activez : `Active la locomotion neurale`

### Entraînement
Entraînez un nouveau modèle à partir de fichiers BVH :
```
> Entraîne un modèle avec les données dans data/bvh/ sur 100 époques
```

---

## 15. L'assistant IA

### Présentation
L'assistant IA intégré peut contrôler **toutes** les fonctionnalités de l'éditeur. Il comprend le français et l'anglais.

### Configuration
1. Par défaut : **Ollama** en local (gratuit, pas de clé API)
   - Installez [Ollama](https://ollama.com/)
   - Téléchargez un modèle : `ollama pull gemma4:26b`
2. **OpenAI** : Entrez votre clé API dans les paramètres
3. **Claude** : Entrez votre clé API Anthropic

### Catégories de commandes

#### Fichiers et import/export
```
Importe samples/humanoid_walk.bvh
Exporte en GLB sous mon_modele.glb
Exporte en FBX sous animation.fbx
Capture d'écran sous capture.png
```

#### Lecture
```
Joue l'animation
Pause
Stop
Va à la frame 42
Vitesse 2x
Active la boucle
```

#### Caméra
```
Réinitialise la caméra
Vue de face
Recule la caméra à 10 mètres
Centre sur le modèle
```

#### Création
```
Crée un humanoïde de 1.80m qui court 3 secondes
Crée une araignée
Crée un crabe de 25cm
Crée un oiseau
Crée un serpent
Crée un quadrupède
```

#### Matériaux
```
Mets un matériau doré
Change la couleur en bleu
Rends le modèle transparent
```

#### IK
```
Résous l'IK du bras gauche vers (1, 1.5, 0.5)
```

#### Physique
```
Crée un ragdoll
Applique une impulsion vers le haut
Explosion de force 30
```

#### Tissu
```
Crée un tissu 12x12 de 1.5m
Pause la simulation
Supprime le tissu
```

#### Affichage
```
Montre le squelette
Cache le mesh
Active la grille
Montre les vélocités
```

#### Rendu
```
Augmente la lumière ambiante à 0.5
Active le SSAO
Active le bloom
Désactive les ombres
```

#### Panneaux
```
Ouvre la console
Ferme le profiler
Ouvre le chat IA
```

#### Enregistrement
```
Démarre l'enregistrement
Arrête l'enregistrement
Pause l'enregistrement
```

#### Multi-personnages
```
Sélectionne le modèle 0
Place le modèle 1 à x=2 y=0 z=0
Cache le modèle 2
```

#### Scènes complexes (commandes combinées)
L'assistant peut chaîner plusieurs commandes :
```
Crée 3 personnages : un qui marche, un qui court, un au repos,
espace-les de 2 mètres, et lance la lecture en boucle
```

---

## 16. Raccourcis clavier

| Raccourci | Action |
|-----------|--------|
| Espace | Lecture / Pause |
| F12 | Console |
| Suppr | Supprimer la sélection |
| Ctrl+Z | Annuler |
| Ctrl+S | Sauvegarder le projet |
| G | Outil Déplacer |
| R | Outil Rotation |
| S | Outil Sélection |
| X/Y/Z | Contraindre à un axe |

---

## 17. FAQ

### Le viewport est noir
- Vérifiez que votre GPU supporte Vulkan/DX12/Metal
- Augmentez `ambient_strength` dans les paramètres de rendu

### L'animation ne se joue pas
- Vérifiez que le modèle contient des données d'animation
- Cliquez ▶ ou tapez "joue" dans le chat IA
- Vérifiez que la vitesse n'est pas à 0

### Le modèle est trop petit/grand
- Utilisez l'outil Échelle ou via l'IA : `Mets l'échelle à 2`
- Les fichiers BVH utilisent souvent des unités cm, le facteur 0.01 est appliqué automatiquement

### Comment utiliser l'IA sans internet ?
- Installez Ollama (local, gratuit)
- Pas de clé API nécessaire
- Fonctionne hors ligne avec des modèles comme `gemma4:26b`, `llama3.1`, `mistral`

### Comment exporter pour Unity/Unreal ?
- **Unity** : Exportez en FBX ou GLB
- **Unreal** : Exportez en FBX
- **Blender** : GLB, FBX ou USD

### Comment ajouter mes propres fichiers mocap ?
1. Placez vos fichiers `.bvh` dans un dossier
2. Ouvrez l'Asset Browser et naviguez vers ce dossier
3. Double-cliquez pour charger
4. Ou via l'IA : `Importe chemin/vers/mon_fichier.bvh`

---

## Architecture technique

### Workspace Cargo (9 crates)

```
ai4animation-rs/
├── crates/
│   ├── anim-math/       # Mathématiques (quaternions, interpolation)
│   ├── anim-core/       # Structures de base (Model, Joint, Skeleton)
│   ├── anim-import/     # Import/Export (GLB, BVH, NPZ, FBX, USD)
│   ├── anim-animation/  # Animation, cloth, motion matching
│   ├── anim-ik/         # Solveur FABRIK
│   ├── anim-render/     # Rendu wgpu (PBR, SSAO, ombres)
│   ├── anim-gui/        # Interface egui (panneaux, thème)
│   ├── anim-app/        # Application principale
│   └── anim-ai/         # Assistant IA (Ollama, OpenAI, Claude)
├── samples/             # Fichiers exemples générés
├── models/              # Modèles ONNX pour locomotion
└── docs/                # Documentation
```

---

*AI4Animation Engine — Rust Edition*
*Basé sur le travail de Sebastian Starke (AI4Animation Unity)*
*Réécrit en Rust avec wgpu + egui*
